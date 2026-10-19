import {
  billShares,
  billTypes,
  bills,
  boarders,
  payments,
  type BillRow,
  type BoarderRow,
} from "@my-better-t-app/db/schema";
import { and, asc, desc, eq, inArray, max } from "drizzle-orm";
import { randomUUID } from "expo-crypto";

import { db, withTransaction } from "@/src/db";
import { getSetting, setSetting } from "@/src/db/seed";

import {
  computeBillSummary,
  computeBoarderPeriodSummaries,
  computeShareState,
  recomputePaymentChanges,
  shareStatusLabel,
} from "./calculations";
import type {
  BillStatus,
  BoarderPeriodSummary,
  PaymentRecord,
  PaymentWithComputedChange,
  ShareState,
  ShareWithState,
  SplitResult,
} from "./types";

export interface AppSettings {
  currencyCode: string;
  lastBackupAt: string | null;
}

export interface BillTypeWithCount {
  id: string;
  name: string;
  isArchived: boolean;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
  billCount: number;
}

export interface BoarderWithBalance {
  id: string;
  name: string;
  status: "active" | "archived";
  archivedAt: number | null;
  createdAt: number;
  updatedAt: number;
  outstandingMinor: number;
  changeOutstandingMinor: number;
}

export interface BillWithDetails {
  bill: BillRow;
  shares: ShareWithState[];
  summary: ReturnType<typeof computeBillSummary>;
  currentTypeName: string;
  isLocked: boolean;
}

export interface DashboardData {
  bills: BillWithDetails[];
  boarderSummaries: BoarderPeriodSummary[];
  archivedBillCount: number;
  totals: {
    expectedMinor: number;
    appliedMinor: number;
    outstandingMinor: number;
    changeOutstandingMinor: number;
    paidShareCount: number;
    partialShareCount: number;
    unpaidShareCount: number;
    totalShareCount: number;
  };
}

export interface BoarderLedgerItem {
  bill: BillRow;
  share: ShareWithState;
  state: ShareState;
}

export interface BoarderLedger {
  boarder: BoarderRow;
  items: BoarderLedgerItem[];
  summary: {
    expectedMinor: number;
    appliedMinor: number;
    outstandingMinor: number;
    changeOutstandingMinor: number;
  };
  payments: Array<{
    payment: PaymentWithComputedChange;
    bill: BillRow;
    share: ShareWithState;
  }>;
}

export interface CreateBillInput {
  billTypeId: string;
  periodYear: number;
  periodMonth: number;
  totalMinor: number;
  dueDate: string | null;
  note: string | null;
  shares: SplitResult[];
}

export interface RecordPaymentInput {
  billShareId: string;
  amountMinor: number;
  paidAt: number;
  note: string | null;
}

function asPaymentRecord(row: typeof payments.$inferSelect): PaymentRecord {
  return {
    id: row.id,
    billShareId: row.billShareId,
    amountMinor: row.amountMinor,
    paidAt: row.paidAt,
    note: row.note,
    changeMinor: row.changeMinor,
    changeReturnedAt: row.changeReturnedAt,
    changeReturnNote: row.changeReturnNote,
    voidedAt: row.voidedAt,
    voidReason: row.voidReason,
    createdAt: row.createdAt,
  };
}

function buildShareStates(
  shareRows: Array<typeof billShares.$inferSelect>,
  paymentsByShare: Map<string, PaymentRecord[]>,
  boardersById: Map<string, BoarderRow>,
): ShareWithState[] {
  return shareRows.map((share) => {
    const boarder = boardersById.get(share.boarderId);
    return {
      ...share,
      currentBoarderName: boarder?.name,
      boarderArchived: boarder?.status === "archived",
      state: computeShareState(share.amountDueMinor, paymentsByShare.get(share.id) ?? []),
    };
  });
}

async function loadPaymentsByShare(
  shareIds: string[],
): Promise<Map<string, PaymentRecord[]>> {
  if (shareIds.length === 0) return new Map();

  const rows = await db.select().from(payments).where(inArray(payments.billShareId, shareIds));
  const grouped = new Map<string, PaymentRecord[]>();

  for (const row of rows) {
    const record = asPaymentRecord(row);
    const current = grouped.get(record.billShareId) ?? [];
    current.push(record);
    grouped.set(record.billShareId, current);
  }

  return grouped;
}

async function loadSharesForBills(billIds: string[]): Promise<ShareWithState[]> {
  if (billIds.length === 0) return [];

  const shareRows = await db.select().from(billShares).where(inArray(billShares.billId, billIds));
  if (shareRows.length === 0) return [];

  const boarderIds = [...new Set(shareRows.map((share) => share.boarderId))];
  const boarderRows =
    boarderIds.length > 0
      ? await db.select().from(boarders).where(inArray(boarders.id, boarderIds))
      : [];
  const boardersById = new Map(boarderRows.map((boarder) => [boarder.id, boarder]));
  const paymentsByShare = await loadPaymentsByShare(shareRows.map((share) => share.id));

  return buildShareStates(shareRows, paymentsByShare, boardersById);
}

async function loadBillWithDetails(
  bill: BillRow,
  typeName: string | undefined,
): Promise<BillWithDetails> {
  const shares = await loadSharesForBills([bill.id]);

  return {
    bill,
    shares,
    summary: computeBillSummary(shares, bill.archivedAt),
    currentTypeName: typeName ?? bill.billTypeNameSnapshot,
    isLocked: shares.some((share) =>
      share.state.payments.some((payment) => payment.isActive),
    ),
  };
}

export async function getAppSettings(): Promise<AppSettings> {
  const [currencyCode, lastBackupAt] = await Promise.all([
    getSetting("currency_code"),
    getSetting("last_backup_at"),
  ]);

  return {
    currencyCode: currencyCode || "PHP",
    lastBackupAt: lastBackupAt || null,
  };
}

export async function setCurrencyCode(currencyCode: string): Promise<void> {
  await setSetting("currency_code", currencyCode);
}

export async function setLastBackupAt(value: string): Promise<void> {
  await setSetting("last_backup_at", value);
}

export async function listBillTypes(): Promise<BillTypeWithCount[]> {
  const [typeRows, billRows] = await Promise.all([
    db.select().from(billTypes).orderBy(asc(billTypes.sortOrder), asc(billTypes.name)),
    db.select({ billTypeId: bills.billTypeId }).from(bills),
  ]);

  const counts = new Map<string, number>();
  for (const row of billRows) {
    counts.set(row.billTypeId, (counts.get(row.billTypeId) ?? 0) + 1);
  }

  return typeRows.map((type) => ({
    ...type,
    billCount: counts.get(type.id) ?? 0,
  }));
}

export async function createBillType(name: string): Promise<string> {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Enter a bill type name.");

  const existing = await db.select().from(billTypes);
  if (
    existing.some(
      (type) =>
        !type.isArchived && type.name.localeCompare(trimmed, undefined, { sensitivity: "base" }) === 0,
    )
  ) {
    throw new Error("An active bill type with that name already exists.");
  }

  const maxRow = await db
    .select({ value: max(billTypes.sortOrder) })
    .from(billTypes)
    .limit(1);
  const now = Date.now();
  const id = randomUUID();

  await db.insert(billTypes).values({
    id,
    name: trimmed,
    isArchived: false,
    sortOrder: (maxRow[0]?.value ?? 0) + 1,
    createdAt: now,
    updatedAt: now,
  });

  return id;
}

export async function renameBillType(id: string, name: string): Promise<void> {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Enter a bill type name.");

  const existing = await db.select().from(billTypes);
  if (
    existing.some(
      (type) =>
        type.id !== id &&
        !type.isArchived &&
        type.name.localeCompare(trimmed, undefined, { sensitivity: "base" }) === 0,
    )
  ) {
    throw new Error("An active bill type with that name already exists.");
  }

  await db
    .update(billTypes)
    .set({ name: trimmed, updatedAt: Date.now() })
    .where(eq(billTypes.id, id));
}

export async function setBillTypeArchived(id: string, isArchived: boolean): Promise<void> {
  if (!isArchived) {
    const rows = await db.select().from(billTypes);
    const target = rows.find((type) => type.id === id);
    if (!target) throw new Error("Bill type not found.");
    if (
      rows.some(
        (type) =>
          type.id !== id &&
          !type.isArchived &&
          type.name.localeCompare(target.name, undefined, { sensitivity: "base" }) === 0,
      )
    ) {
      throw new Error("Rename this type before restoring it; another active type uses that name.");
    }
  }

  await db
    .update(billTypes)
    .set({ isArchived, updatedAt: Date.now() })
    .where(eq(billTypes.id, id));
}

export async function listBoardersWithBalances(): Promise<BoarderWithBalance[]> {
  const [boarderRows, shareRows] = await Promise.all([
    db.select().from(boarders).orderBy(asc(boarders.name)),
    db.select().from(billShares),
  ]);

  const shareIds = shareRows.map((share) => share.id);
  const paymentsByShare = await loadPaymentsByShare(shareIds);
  const totals = new Map<string, { outstanding: number; change: number }>();

  for (const share of shareRows) {
    const state = computeShareState(share.amountDueMinor, paymentsByShare.get(share.id) ?? []);
    const current = totals.get(share.boarderId) ?? { outstanding: 0, change: 0 };
    current.outstanding += state.balanceMinor;
    current.change += state.changeOutstandingMinor;
    totals.set(share.boarderId, current);
  }

  return boarderRows.map((boarder) => ({
    ...boarder,
    outstandingMinor: totals.get(boarder.id)?.outstanding ?? 0,
    changeOutstandingMinor: totals.get(boarder.id)?.change ?? 0,
  }));
}

export async function getBoarder(id: string): Promise<BoarderRow | undefined> {
  const rows = await db.select().from(boarders).where(eq(boarders.id, id)).limit(1);
  return rows[0];
}

export async function createBoarder(name: string): Promise<string> {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Enter a boarder name.");

  const now = Date.now();
  const id = randomUUID();
  await db.insert(boarders).values({
    id,
    name: trimmed,
    status: "active",
    archivedAt: null,
    createdAt: now,
    updatedAt: now,
  });
  return id;
}

export async function renameBoarder(id: string, name: string): Promise<void> {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Enter a boarder name.");

  await db
    .update(boarders)
    .set({ name: trimmed, updatedAt: Date.now() })
    .where(eq(boarders.id, id));
}

export async function setBoarderArchived(id: string, isArchived: boolean): Promise<void> {
  const now = Date.now();
  await db
    .update(boarders)
    .set({
      status: isArchived ? "archived" : "active",
      archivedAt: isArchived ? now : null,
      updatedAt: now,
    })
    .where(eq(boarders.id, id));
}

export async function findDuplicateBills(
  billTypeId: string,
  periodYear: number,
  periodMonth: number,
  excludeBillId?: string,
): Promise<BillRow[]> {
  const rows = await db
    .select()
    .from(bills)
    .where(
      and(
        eq(bills.billTypeId, billTypeId),
        eq(bills.periodYear, periodYear),
        eq(bills.periodMonth, periodMonth),
      ),
    );

  return rows.filter((bill) => bill.id !== excludeBillId);
}

export async function getDashboard(
  periodYear: number,
  periodMonth: number,
  includeArchived: boolean,
): Promise<DashboardData> {
  const [billRows, typeRows] = await Promise.all([
    db
      .select()
      .from(bills)
      .where(and(eq(bills.periodYear, periodYear), eq(bills.periodMonth, periodMonth))),
    db.select().from(billTypes),
  ]);

  const typesById = new Map(typeRows.map((type) => [type.id, type]));
  const filteredBills = includeArchived
    ? billRows
    : billRows.filter((bill) => bill.archivedAt === null);

  const allShareRows = await loadSharesForBills(filteredBills.map((bill) => bill.id));
  const sharesByBill = new Map<string, ShareWithState[]>();
  for (const share of allShareRows) {
    const current = sharesByBill.get(share.billId) ?? [];
    current.push(share);
    sharesByBill.set(share.billId, current);
  }

  const billsWithDetails: BillWithDetails[] = filteredBills.map((bill) => {
    const shares = sharesByBill.get(bill.id) ?? [];
    return {
      bill,
      shares,
      summary: computeBillSummary(shares, bill.archivedAt),
      currentTypeName: typesById.get(bill.billTypeId)?.name ?? bill.billTypeNameSnapshot,
      isLocked: shares.some((share) =>
        share.state.payments.some((payment) => payment.voidedAt === null),
      ),
    };
  });

  billsWithDetails.sort((a, b) => {
    const rank = (status: BillStatus) => (status === "open" ? 0 : status === "settled" ? 1 : 2);
    const byStatus = rank(a.summary.status) - rank(b.summary.status);
    if (byStatus !== 0) return byStatus;

    const typeA = typesById.get(a.bill.billTypeId);
    const typeB = typesById.get(b.bill.billTypeId);
    const byOrder = (typeA?.sortOrder ?? 0) - (typeB?.sortOrder ?? 0);
    if (byOrder !== 0) return byOrder;

    return a.bill.createdAt - b.bill.createdAt;
  });

  const activeBills = billsWithDetails.filter((item) => item.bill.archivedAt === null);
  const activeShares = activeBills.flatMap((item) => item.shares);
  const nonArchivedSummaries = activeBills.map((item) => item.summary);

  const boarderSummaries = computeBoarderPeriodSummaries(
    activeShares.map((share) => ({
      boarderId: share.boarderId,
      boarderName: share.currentBoarderName ?? share.boarderNameSnapshot,
      boarderArchived: share.boarderArchived ?? false,
      amountDueMinor: share.amountDueMinor,
      state: share.state,
    })),
  );

  return {
    bills: billsWithDetails,
    boarderSummaries,
    archivedBillCount: billRows.filter((bill) => bill.archivedAt !== null).length,
    totals: {
      expectedMinor: nonArchivedSummaries.reduce((total, item) => total + item.expectedMinor, 0),
      appliedMinor: nonArchivedSummaries.reduce((total, item) => total + item.appliedMinor, 0),
      outstandingMinor: nonArchivedSummaries.reduce(
        (total, item) => total + item.outstandingMinor,
        0,
      ),
      changeOutstandingMinor: nonArchivedSummaries.reduce(
        (total, item) => total + item.changeOutstandingMinor,
        0,
      ),
      paidShareCount: nonArchivedSummaries.reduce(
        (total, item) => total + item.paidShareCount,
        0,
      ),
      partialShareCount: nonArchivedSummaries.reduce(
        (total, item) => total + item.partialShareCount,
        0,
      ),
      unpaidShareCount: nonArchivedSummaries.reduce(
        (total, item) => total + item.unpaidShareCount,
        0,
      ),
      totalShareCount: nonArchivedSummaries.reduce(
        (total, item) => total + item.totalShareCount,
        0,
      ),
    },
  };
}

export async function getBillDetail(id: string): Promise<BillWithDetails | undefined> {
  const rows = await db.select().from(bills).where(eq(bills.id, id)).limit(1);
  const bill = rows[0];
  if (!bill) return undefined;

  const typeRows = await db
    .select()
    .from(billTypes)
    .where(eq(billTypes.id, bill.billTypeId))
    .limit(1);

  return loadBillWithDetails(bill, typeRows[0]?.name);
}

export async function getBoarderLedger(id: string): Promise<BoarderLedger | undefined> {
  const boarder = await getBoarder(id);
  if (!boarder) return undefined;

  const shareRows = await db
    .select()
    .from(billShares)
    .where(eq(billShares.boarderId, id))
    .orderBy(desc(billShares.createdAt));

  const billIds = [...new Set(shareRows.map((share) => share.billId))];
  const billRows =
    billIds.length > 0 ? await db.select().from(bills).where(inArray(bills.id, billIds)) : [];
  const billsById = new Map(billRows.map((bill) => [bill.id, bill]));
  const paymentsByShare = await loadPaymentsByShare(shareRows.map((share) => share.id));
  const boardersById = new Map([[boarder.id, boarder]]);
  const shares = buildShareStates(shareRows, paymentsByShare, boardersById);

  const items: BoarderLedgerItem[] = shares
    .map((share) => {
      const bill = billsById.get(share.billId);
      if (!bill) return undefined;
      return { bill, share, state: share.state };
    })
    .filter((item): item is BoarderLedgerItem => Boolean(item))
    .sort((a, b) => {
      if (a.bill.periodYear !== b.bill.periodYear) return b.bill.periodYear - a.bill.periodYear;
      if (a.bill.periodMonth !== b.bill.periodMonth) {
        return b.bill.periodMonth - a.bill.periodMonth;
      }
      return b.bill.createdAt - a.bill.createdAt;
    });

  const flattenedPayments = shares
    .flatMap((share) => {
      const bill = billsById.get(share.billId);
      if (!bill) return [];
      return share.state.payments.map((payment) => ({
        payment,
        bill,
        share,
      }));
    })
    .sort((a, b) => b.payment.paidAt - a.payment.paidAt);

  return {
    boarder,
    items,
    summary: {
      expectedMinor: items.reduce((total, item) => total + item.share.amountDueMinor, 0),
      appliedMinor: items.reduce((total, item) => total + item.state.appliedMinor, 0),
      outstandingMinor: items.reduce((total, item) => total + item.state.balanceMinor, 0),
      changeOutstandingMinor: items.reduce(
        (total, item) => total + item.state.changeOutstandingMinor,
        0,
      ),
    },
    payments: flattenedPayments,
  };
}

export async function createBill(input: CreateBillInput): Promise<string> {
  if (input.totalMinor <= 0) throw new Error("Enter a bill total greater than zero.");
  if (input.shares.length === 0) throw new Error("Select at least one boarder.");

  const shareTotal = input.shares.reduce((total, share) => total + share.amountMinor, 0);
  if (shareTotal !== input.totalMinor) {
    throw new Error("The assigned shares must equal the bill total.");
  }
  if (new Set(input.shares.map((share) => share.boarderId)).size !== input.shares.length) {
    throw new Error("A boarder can only appear once on a bill.");
  }

  const typeRows = await db
    .select()
    .from(billTypes)
    .where(eq(billTypes.id, input.billTypeId))
    .limit(1);
  const type = typeRows[0];
  if (!type) throw new Error("Bill type not found.");
  if (type.isArchived) throw new Error("That bill type is archived.");

  const boarderIds = input.shares.map((share) => share.boarderId);
  const boarderRows = await db.select().from(boarders).where(inArray(boarders.id, boarderIds));
  const boardersById = new Map(boarderRows.map((boarder) => [boarder.id, boarder]));

  for (const boarderId of boarderIds) {
    const boarder = boardersById.get(boarderId);
    if (!boarder) throw new Error("One of the selected boarders no longer exists.");
    if (boarder.status !== "active") throw new Error("Archived boarders cannot be added to a new bill.");
  }

  const now = Date.now();
  const billId = randomUUID();

  await withTransaction(async () => {
    await db.insert(bills).values({
      id: billId,
      billTypeId: type.id,
      billTypeNameSnapshot: type.name,
      periodYear: input.periodYear,
      periodMonth: input.periodMonth,
      totalMinor: input.totalMinor,
      dueDate: input.dueDate,
      note: input.note,
      archivedAt: null,
      createdAt: now,
      updatedAt: now,
    });

    await db.insert(billShares).values(
      input.shares.map((share) => ({
        id: randomUUID(),
        billId,
        boarderId: share.boarderId,
        boarderNameSnapshot: boardersById.get(share.boarderId)?.name ?? "",
        amountDueMinor: share.amountMinor,
        createdAt: now,
      })),
    );
  });

  return billId;
}

export async function updateBill(id: string, input: CreateBillInput): Promise<void> {
  const existing = await getBillDetail(id);
  if (!existing) throw new Error("Bill not found.");
  if (existing.bill.archivedAt !== null) throw new Error("Unarchive the bill before editing it.");
  if (existing.isLocked) {
    throw new Error("Money fields are locked because payments have been recorded.");
  }

  const shareTotal = input.shares.reduce((total, share) => total + share.amountMinor, 0);
  if (shareTotal !== input.totalMinor) {
    throw new Error("The assigned shares must equal the bill total.");
  }
  if (input.shares.length === 0) throw new Error("Select at least one boarder.");

  const typeRows = await db
    .select()
    .from(billTypes)
    .where(eq(billTypes.id, input.billTypeId))
    .limit(1);
  const type = typeRows[0];
  if (!type) throw new Error("Bill type not found.");
  if (type.isArchived) throw new Error("That bill type is archived.");

  const boarderIds = input.shares.map((share) => share.boarderId);
  const boarderRows = await db.select().from(boarders).where(inArray(boarders.id, boarderIds));
  const boardersById = new Map(boarderRows.map((boarder) => [boarder.id, boarder]));
  const oldSnapshots = new Map(existing.shares.map((share) => [share.boarderId, share.boarderNameSnapshot]));
  const existingBoarderIds = new Set(existing.shares.map((share) => share.boarderId));

  for (const boarderId of boarderIds) {
    const boarder = boardersById.get(boarderId);
    if (!boarder) throw new Error("One of the selected boarders no longer exists.");
    if (boarder.status !== "active" && !existingBoarderIds.has(boarder.id)) {
      throw new Error("Archived boarders cannot be added to a bill.");
    }
  }

  await withTransaction(async () => {
    await db
      .update(bills)
      .set({
        billTypeId: type.id,
        billTypeNameSnapshot:
          type.id === existing.bill.billTypeId
            ? existing.bill.billTypeNameSnapshot
            : type.name,
        periodYear: input.periodYear,
        periodMonth: input.periodMonth,
        totalMinor: input.totalMinor,
        dueDate: input.dueDate,
        note: input.note,
        updatedAt: Date.now(),
      })
      .where(eq(bills.id, id));

    await db.delete(billShares).where(eq(billShares.billId, id));
    await db.insert(billShares).values(
      input.shares.map((share) => ({
        id: randomUUID(),
        billId: id,
        boarderId: share.boarderId,
        boarderNameSnapshot:
          oldSnapshots.get(share.boarderId) ??
          boardersById.get(share.boarderId)?.name ??
          "",
        amountDueMinor: share.amountMinor,
        createdAt: Date.now(),
      })),
    );
  });
}

export async function setBillArchived(id: string, isArchived: boolean): Promise<void> {
  await db
    .update(bills)
    .set({ archivedAt: isArchived ? Date.now() : null, updatedAt: Date.now() })
    .where(eq(bills.id, id));
}

export async function updateBillMeta(
  id: string,
  dueDate: string | null,
  note: string | null,
): Promise<void> {
  await db
    .update(bills)
    .set({ dueDate, note, updatedAt: Date.now() })
    .where(eq(bills.id, id));
}

async function getShareWithPayments(billShareId: string): Promise<{
  share: typeof billShares.$inferSelect;
  payments: PaymentRecord[];
} | undefined> {
  const shareRows = await db
    .select()
    .from(billShares)
    .where(eq(billShares.id, billShareId))
    .limit(1);
  const share = shareRows[0];
  if (!share) return undefined;

  const paymentRows = await db
    .select()
    .from(payments)
    .where(eq(payments.billShareId, billShareId));
  return { share, payments: paymentRows.map(asPaymentRecord) };
}

export async function recordPayment(input: RecordPaymentInput): Promise<string> {
  if (input.amountMinor <= 0) throw new Error("Enter a payment amount greater than zero.");

  const data = await getShareWithPayments(input.billShareId);
  if (!data) throw new Error("Bill share not found.");

  const paymentId = randomUUID();
  const now = Date.now();
  const draft: PaymentRecord = {
    id: paymentId,
    billShareId: input.billShareId,
    amountMinor: input.amountMinor,
    paidAt: input.paidAt,
    note: input.note,
    changeMinor: 0,
    changeReturnedAt: null,
    changeReturnNote: null,
    voidedAt: null,
    voidReason: null,
    createdAt: now,
  };

  const computed = recomputePaymentChanges(data.share.amountDueMinor, [...data.payments, draft]);
  const newPayment = computed.find((payment) => payment.id === paymentId);
  if (!newPayment) throw new Error("The payment could not be calculated.");

  await withTransaction(async () => {
    await db.insert(payments).values({
      id: paymentId,
      billShareId: input.billShareId,
      amountMinor: input.amountMinor,
      paidAt: input.paidAt,
      note: input.note,
      changeMinor: newPayment.computedChangeMinor,
      changeReturnedAt: null,
      changeReturnNote: null,
      voidedAt: null,
      voidReason: null,
      createdAt: now,
    });

    for (const payment of computed) {
      if (payment.id === paymentId || !payment.isActive) continue;
      if (payment.changeMinor !== payment.computedChangeMinor) {
        await db
          .update(payments)
          .set({ changeMinor: payment.computedChangeMinor })
          .where(eq(payments.id, payment.id));
      }
    }
  });

  return paymentId;
}

export async function markChangeReturned(
  paymentId: string,
  returnedAt: number,
  note: string | null,
): Promise<void> {
  const rows = await db.select().from(payments).where(eq(payments.id, paymentId)).limit(1);
  const payment = rows[0];
  if (!payment) throw new Error("Payment not found.");
  if (payment.voidedAt !== null) throw new Error("Voided payments cannot return change.");
  if (payment.changeReturnedAt !== null) throw new Error("This change was already marked returned.");

  const data = await getShareWithPayments(payment.billShareId);
  if (!data) throw new Error("Bill share not found.");
  const computed = recomputePaymentChanges(data.share.amountDueMinor, data.payments);
  const current = computed.find((item) => item.id === paymentId);
  if (!current || current.computedChangeMinor <= 0) {
    throw new Error("There is no change to return for this payment.");
  }

  await db
    .update(payments)
    .set({
      changeMinor: current.computedChangeMinor,
      changeReturnedAt: returnedAt,
      changeReturnNote: note,
    })
    .where(eq(payments.id, paymentId));
}

export async function voidPayment(paymentId: string, reason: string): Promise<void> {
  const trimmed = reason.trim();
  if (!trimmed) throw new Error("Enter a reason for voiding this payment.");

  const rows = await db.select().from(payments).where(eq(payments.id, paymentId)).limit(1);
  const payment = rows[0];
  if (!payment) throw new Error("Payment not found.");
  if (payment.voidedAt !== null) throw new Error("This payment is already voided.");

  const data = await getShareWithPayments(payment.billShareId);
  if (!data) throw new Error("Bill share not found.");

  const now = Date.now();
  const updatedPayments = data.payments.map((item) =>
    item.id === paymentId ? { ...item, voidedAt: now, voidReason: trimmed } : item,
  );
  const computed = recomputePaymentChanges(data.share.amountDueMinor, updatedPayments);

  await withTransaction(async () => {
    await db
      .update(payments)
      .set({ voidedAt: now, voidReason: trimmed })
      .where(eq(payments.id, paymentId));

    for (const item of computed) {
      if (!item.isActive) continue;
      if (item.changeMinor !== item.computedChangeMinor) {
        await db
          .update(payments)
          .set({ changeMinor: item.computedChangeMinor })
          .where(eq(payments.id, item.id));
      }
    }
  });
}

export async function getSharePaymentContext(
  billShareId: string,
): Promise<{ share: ShareWithState; bill: BillRow; boarderName: string } | undefined> {
  const data = await getShareWithPayments(billShareId);
  if (!data) return undefined;

  const billRows = await db
    .select()
    .from(bills)
    .where(eq(bills.id, data.share.billId))
    .limit(1);
  const bill = billRows[0];
  if (!bill) return undefined;

  const boarderRows = await db
    .select()
    .from(boarders)
    .where(eq(boarders.id, data.share.boarderId))
    .limit(1);
  const boarder = boarderRows[0];

  const state = computeShareState(data.share.amountDueMinor, data.payments);
  return {
    share: {
      ...data.share,
      currentBoarderName: boarder?.name,
      boarderArchived: boarder?.status === "archived",
      state,
    },
    bill,
    boarderName: data.share.boarderNameSnapshot,
  };
}

export async function getActiveBoarders(): Promise<BoarderRow[]> {
  return db
    .select()
    .from(boarders)
    .where(eq(boarders.status, "active"))
    .orderBy(asc(boarders.name));
}

export async function getAllActiveBoardersForPicker(): Promise<BoarderRow[]> {
  return getActiveBoarders();
}

export function shareStatusText(share: ShareWithState): string {
  return shareStatusLabel(share.state.status);
}


export async function getBillTypeById(id: string) {
  const rows = await db.select().from(billTypes).where(eq(billTypes.id, id)).limit(1);
  return rows[0];
}

export async function getBoarderRow(id: string): Promise<BoarderRow | undefined> {
  return getBoarder(id);
}
