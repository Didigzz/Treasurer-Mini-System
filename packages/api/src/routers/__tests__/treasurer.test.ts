import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { createDb, type Database } from "@my-better-t-app/db";
import { migrateDatabase } from "@my-better-t-app/db/migrate";
import { rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { seedDatabase } from "../../treasurer/seed";
import { appRouter } from "../index";

let db: Database;
let caller: ReturnType<typeof appRouter.createCaller>;
let databasePath: string;

beforeEach(async () => {
  databasePath = join(tmpdir(), `treasurer-test-${crypto.randomUUID()}.db`).replace(/\\/g, "/");
  db = createDb({ DATABASE_URL: `file:${databasePath}` });
  await migrateDatabase(db);
  await seedDatabase(db);
  caller = appRouter.createCaller({ db });
});

afterEach(async () => {
  await db.$client.close();
  // Windows can keep the SQLite handle briefly after close; a leftover temp
  // file is harmless, so cleanup is best-effort.
  for (const path of [databasePath, `${databasePath}-shm`, `${databasePath}-wal`]) {
    try {
      await rm(path, { force: true, maxRetries: 3, retryDelay: 100 });
    } catch {
      // ignore locked temp files
    }
  }
});

describe("treasurer router", () => {
  test("creates boarders, bill types, bills, and payments with matching totals", async () => {
    const types = await caller.billTypes.list();
    expect(types.length).toBe(5);

    const water = types.find((type) => type.name === "Water");
    expect(water).toBeDefined();

    const anaId = await caller.boarders.create({ name: "Ana" });
    const benId = await caller.boarders.create({ name: "Ben" });
    const caraId = await caller.boarders.create({ name: "Cara" });

    const billId = await caller.bills.create({
      billTypeId: water!.id,
      periodYear: 2026,
      periodMonth: 11,
      totalMinor: 100_000,
      dueDate: "2026-11-15",
      note: "November water",
      shares: [
        { boarderId: anaId, amountMinor: 33_334 },
        { boarderId: benId, amountMinor: 33_333 },
        { boarderId: caraId, amountMinor: 33_333 },
      ],
    });

    const detail = await caller.bills.get({ id: billId });
    expect(detail).toBeDefined();
    expect(detail!.summary.expectedMinor).toBe(100_000);
    expect(detail!.summary.outstandingMinor).toBe(100_000);
    expect(detail!.shares.length).toBe(3);

    const anaShare = detail!.shares.find((share) => share.boarderId === anaId)!;

    await caller.payments.record({
      billShareId: anaShare.id,
      amountMinor: 33_334,
      paidAt: Date.now(),
      note: null,
    });

    await caller.payments.record({
      billShareId: anaShare.id,
      amountMinor: 5_000,
      paidAt: Date.now(),
      note: "overpaid",
    });

    const afterPayments = await caller.bills.get({ id: billId });
    expect(afterPayments!.summary.appliedMinor).toBe(33_334);
    expect(afterPayments!.summary.outstandingMinor).toBe(66_666);
    expect(afterPayments!.summary.changeOutstandingMinor).toBe(5_000);
    expect(afterPayments!.summary.paidShareCount).toBe(1);

    const dashboard = await caller.bills.dashboard({
      year: 2026,
      month: 11,
      includeArchived: false,
    });
    expect(dashboard.totals.expectedMinor).toBe(100_000);
    expect(dashboard.totals.appliedMinor).toBe(33_334);
    expect(dashboard.totals.outstandingMinor).toBe(66_666);
    expect(dashboard.totals.changeOutstandingMinor).toBe(5_000);
    expect(dashboard.totals.totalShareCount).toBe(3);
    expect(dashboard.boarderSummaries.length).toBe(3);

    const anaSummary = dashboard.boarderSummaries.find(
      (summary) => summary.boarderId === anaId,
    )!;
    expect(anaSummary.status).toBe("paid");
    expect(anaSummary.changeOutstandingMinor).toBe(5_000);
  });

  test("requires shares to equal the bill total and rejects duplicate boarders", async () => {
    const types = await caller.billTypes.list();
    const rent = types.find((type) => type.name === "Rent")!;
    const boarderId = await caller.boarders.create({ name: "Dee" });

    expect(
      caller.bills.create({
        billTypeId: rent.id,
        periodYear: 2026,
        periodMonth: 11,
        totalMinor: 1_000,
        dueDate: null,
        note: null,
        shares: [{ boarderId, amountMinor: 900 }],
      }),
    ).rejects.toThrow("assigned shares must equal the bill total");
  });

  test("detects duplicate bills for the same type and period", async () => {
    const types = await caller.billTypes.list();
    const rent = types.find((type) => type.name === "Rent")!;
    const boarderId = await caller.boarders.create({ name: "Eli" });

    const input = {
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
      totalMinor: 1_000,
      dueDate: null as string | null,
      note: null as string | null,
      shares: [{ boarderId, amountMinor: 1_000 }],
    };

    const billId = await caller.bills.create(input);
    const duplicates = await caller.bills.findDuplicates({
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
    });
    expect(duplicates.map((bill) => bill.id)).toEqual([billId]);

    const excluded = await caller.bills.findDuplicates({
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
      excludeBillId: billId,
    });
    expect(excluded).toEqual([]);
  });

  test("voids a payment and recomputes change", async () => {
    const types = await caller.billTypes.list();
    const rent = types.find((type) => type.name === "Rent")!;
    const boarderId = await caller.boarders.create({ name: "Fay" });
    const billId = await caller.bills.create({
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
      totalMinor: 1_000,
      dueDate: null,
      note: null,
      shares: [{ boarderId, amountMinor: 1_000 }],
    });
    const detail = await caller.bills.get({ id: billId });
    const shareId = detail!.shares[0]!.id;

    const paymentId = await caller.payments.record({
      billShareId: shareId,
      amountMinor: 1_500,
      paidAt: Date.now(),
      note: null,
    });

    await caller.payments.void({ paymentId, reason: "Wrong amount" });

    const afterVoid = await caller.payments.context({ shareId });
    expect(afterVoid!.share.state.paidMinor).toBe(0);
    expect(afterVoid!.share.state.changeOutstandingMinor).toBe(0);
    expect(afterVoid!.share.state.payments[0]!.voidedAt).not.toBeNull();
  });

  test("archives bills and excludes them from default totals", async () => {
    const types = await caller.billTypes.list();
    const rent = types.find((type) => type.name === "Rent")!;
    const boarderId = await caller.boarders.create({ name: "Gus" });
    const billId = await caller.bills.create({
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
      totalMinor: 2_000,
      dueDate: null,
      note: null,
      shares: [{ boarderId, amountMinor: 2_000 }],
    });

    await caller.bills.setArchived({ id: billId, isArchived: true });

    const withoutArchived = await caller.bills.dashboard({
      year: 2026,
      month: 11,
      includeArchived: false,
    });
    expect(withoutArchived.bills).toEqual([]);
    expect(withoutArchived.archivedBillCount).toBe(1);

    const withArchived = await caller.bills.dashboard({
      year: 2026,
      month: 11,
      includeArchived: true,
    });
    expect(withArchived.bills.length).toBe(1);
    expect(withArchived.bills[0]!.summary.status).toBe("archived");
  });

  test("round-trips a backup export and import", async () => {
    const boarderId = await caller.boarders.create({ name: "Hana" });
    const types = await caller.billTypes.list();
    const rent = types.find((type) => type.name === "Rent")!;
    await caller.bills.create({
      billTypeId: rent.id,
      periodYear: 2026,
      periodMonth: 11,
      totalMinor: 3_000,
      dueDate: null,
      note: "Backup test",
      shares: [{ boarderId, amountMinor: 3_000 }],
    });

    const payload = await caller.backup.export();
    const json = JSON.stringify(payload);

    const preview = await caller.backup.inspect({ json });
    expect(preview.counts.bills).toBe(1);
    expect(preview.counts.boarders).toBe(1);

    await caller.backup.import({ json });

    const boarders = await caller.boarders.listWithBalances();
    expect(boarders.map((boarder) => boarder.name)).toContain("Hana");

    const dashboard = await caller.bills.dashboard({
      year: 2026,
      month: 11,
      includeArchived: false,
    });
    expect(dashboard.totals.expectedMinor).toBe(3_000);
  });
});
