import {
  billShares,
  billTypes,
  bills,
  boarders,
  payments,
  settings,
} from "@my-better-t-app/db/schema";
import * as DocumentPicker from "expo-document-picker";
import { File, Paths } from "expo-file-system";
import * as Sharing from "expo-sharing";
import { z } from "zod";

import { db, withTransaction } from "@/src/db";
import { setLastBackupAt } from "./repository";
import { MAX_AMOUNT_MINOR } from "./money";
import { recomputePaymentChanges } from "./calculations";
import type { PaymentRecord } from "./types";

const MAX_BACKUP_BYTES = 10 * 1024 * 1024;

const isoTimestamp = z.string().refine((value) => !Number.isNaN(Date.parse(value)), {
  message: "Invalid timestamp.",
});

const moneySchema = z.number().int().min(1).max(MAX_AMOUNT_MINOR);
const optionalText = (max: number) => z.string().max(max).nullable();

const billTypeSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1).max(40),
  isArchived: z.boolean(),
  sortOrder: z.number().int(),
  createdAt: isoTimestamp,
  updatedAt: isoTimestamp,
});

const boarderSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1).max(80),
  status: z.enum(["active", "archived"]),
  archivedAt: isoTimestamp.nullable(),
  createdAt: isoTimestamp,
  updatedAt: isoTimestamp,
});

const billSchema = z.object({
  id: z.string().min(1),
  billTypeId: z.string().min(1),
  billTypeNameSnapshot: z.string().min(1).max(40),
  periodYear: z.number().int().min(2000).max(2100),
  periodMonth: z.number().int().min(1).max(12),
  totalMinor: moneySchema,
  dueDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).nullable(),
  note: optionalText(500),
  archivedAt: isoTimestamp.nullable(),
  createdAt: isoTimestamp,
  updatedAt: isoTimestamp,
});

const billShareSchema = z.object({
  id: z.string().min(1),
  billId: z.string().min(1),
  boarderId: z.string().min(1),
  boarderNameSnapshot: z.string().min(1).max(80),
  amountDueMinor: moneySchema,
  createdAt: isoTimestamp,
});

const paymentSchema = z.object({
  id: z.string().min(1),
  billShareId: z.string().min(1),
  amountMinor: moneySchema,
  paidAt: isoTimestamp,
  note: optionalText(500),
  changeMinor: z.number().int().min(0).max(MAX_AMOUNT_MINOR),
  changeReturnedAt: isoTimestamp.nullable(),
  changeReturnNote: optionalText(200),
  voidedAt: isoTimestamp.nullable(),
  voidReason: optionalText(200),
  createdAt: isoTimestamp,
});

const settingsSchema = z.object({
  currencyCode: z.string().min(3).max(3),
  lastBackupAt: isoTimestamp.nullable(),
});

export const backupSchema = z.object({
  format: z.literal("boarding-house-treasurer"),
  version: z.literal(1),
  exportedAt: isoTimestamp,
  data: z.object({
    settings: settingsSchema,
    billTypes: z.array(billTypeSchema),
    boarders: z.array(boarderSchema),
    bills: z.array(billSchema),
    billShares: z.array(billShareSchema),
    payments: z.array(paymentSchema),
  }),
});

export type BackupPayload = z.infer<typeof backupSchema>;

export interface BackupPreview {
  payload: BackupPayload;
  fileName: string;
  counts: {
    billTypes: number;
    boarders: number;
    bills: number;
    shares: number;
    payments: number;
  };
}

function toIso(epochMs: number | null): string | null {
  return epochMs === null ? null : new Date(epochMs).toISOString();
}

function toMs(iso: string): number {
  return new Date(iso).getTime();
}

export async function createBackupPayload(): Promise<BackupPayload> {
  const [settingRows, typeRows, boarderRows, billRows, shareRows, paymentRows] =
    await Promise.all([
      db.select().from(settings),
      db.select().from(billTypes),
      db.select().from(boarders),
      db.select().from(bills),
      db.select().from(billShares),
      db.select().from(payments),
    ]);

  const settingsMap = new Map(settingRows.map((row) => [row.key, row.value]));

  return {
    format: "boarding-house-treasurer",
    version: 1,
    exportedAt: new Date().toISOString(),
    data: {
      settings: {
        currencyCode: settingsMap.get("currency_code") || "PHP",
        lastBackupAt: settingsMap.get("last_backup_at")
          ? settingsMap.get("last_backup_at")!
          : null,
      },
      billTypes: typeRows.map((row) => ({
        id: row.id,
        name: row.name,
        isArchived: row.isArchived,
        sortOrder: row.sortOrder,
        createdAt: toIso(row.createdAt)!,
        updatedAt: toIso(row.updatedAt)!,
      })),
      boarders: boarderRows.map((row) => ({
        id: row.id,
        name: row.name,
        status: row.status,
        archivedAt: toIso(row.archivedAt),
        createdAt: toIso(row.createdAt)!,
        updatedAt: toIso(row.updatedAt)!,
      })),
      bills: billRows.map((row) => ({
        id: row.id,
        billTypeId: row.billTypeId,
        billTypeNameSnapshot: row.billTypeNameSnapshot,
        periodYear: row.periodYear,
        periodMonth: row.periodMonth,
        totalMinor: row.totalMinor,
        dueDate: row.dueDate,
        note: row.note,
        archivedAt: toIso(row.archivedAt),
        createdAt: toIso(row.createdAt)!,
        updatedAt: toIso(row.updatedAt)!,
      })),
      billShares: shareRows.map((row) => ({
        id: row.id,
        billId: row.billId,
        boarderId: row.boarderId,
        boarderNameSnapshot: row.boarderNameSnapshot,
        amountDueMinor: row.amountDueMinor,
        createdAt: toIso(row.createdAt)!,
      })),
      payments: paymentRows.map((row) => ({
        id: row.id,
        billShareId: row.billShareId,
        amountMinor: row.amountMinor,
        paidAt: toIso(row.paidAt)!,
        note: row.note,
        changeMinor: row.changeMinor,
        changeReturnedAt: toIso(row.changeReturnedAt),
        changeReturnNote: row.changeReturnNote,
        voidedAt: toIso(row.voidedAt),
        voidReason: row.voidReason,
        createdAt: toIso(row.createdAt)!,
      })),
    },
  };
}

export function parseBackupJson(json: string): BackupPayload {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new Error("This file is not valid JSON.");
  }

  const result = backupSchema.safeParse(parsed);
  if (!result.success) {
    throw new Error("This file is not a valid boarding house backup.");
  }

  validateBackup(result.data);
  return result.data;
}

export function validateBackup(payload: BackupPayload): void {
  const { billTypes: types, boarders: people, bills: billRows, billShares: shares, payments: paymentRows } =
    payload.data;

  ensureUnique(types.map((row) => row.id), "bill type IDs");
  ensureUnique(people.map((row) => row.id), "boarder IDs");
  ensureUnique(billRows.map((row) => row.id), "bill IDs");
  ensureUnique(shares.map((row) => row.id), "share IDs");
  ensureUnique(paymentRows.map((row) => row.id), "payment IDs");

  const typeIds = new Set(types.map((row) => row.id));
  const boarderIds = new Set(people.map((row) => row.id));
  const billIds = new Set(billRows.map((row) => row.id));
  const shareIds = new Set(shares.map((row) => row.id));

  for (const bill of billRows) {
    if (!typeIds.has(bill.billTypeId)) {
      throw new Error("The backup contains a bill with a missing bill type.");
    }
  }

  const shareKeys = new Set<string>();
  for (const share of shares) {
    if (!billIds.has(share.billId)) {
      throw new Error("The backup contains a share with a missing bill.");
    }
    if (!boarderIds.has(share.boarderId)) {
      throw new Error("The backup contains a share with a missing boarder.");
    }
    const key = `${share.billId}:${share.boarderId}`;
    if (shareKeys.has(key)) {
      throw new Error("The backup contains a duplicate boarder share.");
    }
    shareKeys.add(key);
  }

  for (const bill of billRows) {
    const assigned = shares
      .filter((share) => share.billId === bill.id)
      .reduce((total, share) => total + share.amountDueMinor, 0);
    if (assigned !== bill.totalMinor) {
      throw new Error("The backup contains a bill whose shares do not equal its total.");
    }
  }

  for (const payment of paymentRows) {
    if (!shareIds.has(payment.billShareId)) {
      throw new Error("The backup contains a payment with a missing share.");
    }
  }
}

function ensureUnique(values: string[], label: string): void {
  if (new Set(values).size !== values.length) {
    throw new Error(`The backup contains duplicate ${label}.`);
  }
}

export async function exportBackup(): Promise<{ fileName: string }> {
  const payload = await createBackupPayload();
  const now = new Date();
  const stamp = now.toISOString().slice(0, 16).replace("T", "-").replace(":", "");
  const fileName = `boarding-house-treasurer-backup-${stamp}.json`;
  const file = new File(Paths.cache, fileName);

  if (file.exists) file.delete();
  file.create();
  await file.write(JSON.stringify(payload, null, 2));

  if (!(await Sharing.isAvailableAsync())) {
    throw new Error("Sharing is not available on this device.");
  }

  await Sharing.shareAsync(file.uri, {
    mimeType: "application/json",
    dialogTitle: "Save boarding house backup",
  });

  await setLastBackupAt(now.toISOString());
  return { fileName };
}

export async function pickBackupFile(): Promise<BackupPreview | null> {
  const result = await DocumentPicker.getDocumentAsync({
    type: ["application/json", "text/plain"],
    copyToCacheDirectory: true,
    multiple: false,
  });

  if (result.canceled) return null;

  const asset = result.assets[0];
  if (!asset) return null;
  if (asset.size && asset.size > MAX_BACKUP_BYTES) {
    throw new Error("That backup is too large. Choose a file under 10 MB.");
  }

  const file = new File(asset.uri);
  const text = await file.text();
  const payload = parseBackupJson(text);

  return {
    payload,
    fileName: asset.name,
    counts: {
      billTypes: payload.data.billTypes.length,
      boarders: payload.data.boarders.length,
      bills: payload.data.bills.length,
      shares: payload.data.billShares.length,
      payments: payload.data.payments.length,
    },
  };
}

export async function restoreBackup(payload: BackupPayload): Promise<void> {
  validateBackup(payload);
  const data = payload.data;

  const paymentsByShare = new Map<string, PaymentRecord[]>();
  for (const payment of data.payments) {
    const record: PaymentRecord = {
      id: payment.id,
      billShareId: payment.billShareId,
      amountMinor: payment.amountMinor,
      paidAt: toMs(payment.paidAt),
      note: payment.note,
      changeMinor: payment.changeMinor,
      changeReturnedAt: payment.changeReturnedAt ? toMs(payment.changeReturnedAt) : null,
      changeReturnNote: payment.changeReturnNote,
      voidedAt: payment.voidedAt ? toMs(payment.voidedAt) : null,
      voidReason: payment.voidReason,
      createdAt: toMs(payment.createdAt),
    };
    const current = paymentsByShare.get(record.billShareId) ?? [];
    current.push(record);
    paymentsByShare.set(record.billShareId, current);
  }

  const computedPayments: Array<PaymentRecord & { computedChangeMinor: number }> = [];
  for (const share of data.billShares) {
    const sharePayments = paymentsByShare.get(share.id) ?? [];
    const computed = recomputePaymentChanges(share.amountDueMinor, sharePayments);
    for (const payment of computed) {
      computedPayments.push({
        ...payment,
        computedChangeMinor: payment.computedChangeMinor,
      });
    }
  }

  await withTransaction(async () => {
    await db.delete(payments);
    await db.delete(billShares);
    await db.delete(bills);
    await db.delete(boarders);
    await db.delete(billTypes);
    await db.delete(settings);

    const now = Date.now();
    await db.insert(settings).values([
      {
        key: "currency_code",
        value: data.settings.currencyCode,
        updatedAt: now,
      },
      {
        key: "last_backup_at",
        value: data.settings.lastBackupAt ?? "",
        updatedAt: now,
      },
    ]);

    if (data.billTypes.length > 0) {
      await db.insert(billTypes).values(
        data.billTypes.map((row) => ({
          id: row.id,
          name: row.name,
          isArchived: row.isArchived,
          sortOrder: row.sortOrder,
          createdAt: toMs(row.createdAt),
          updatedAt: toMs(row.updatedAt),
        })),
      );
    }

    if (data.boarders.length > 0) {
      await db.insert(boarders).values(
        data.boarders.map((row) => ({
          id: row.id,
          name: row.name,
          status: row.status,
          archivedAt: row.archivedAt ? toMs(row.archivedAt) : null,
          createdAt: toMs(row.createdAt),
          updatedAt: toMs(row.updatedAt),
        })),
      );
    }

    if (data.bills.length > 0) {
      await db.insert(bills).values(
        data.bills.map((row) => ({
          id: row.id,
          billTypeId: row.billTypeId,
          billTypeNameSnapshot: row.billTypeNameSnapshot,
          periodYear: row.periodYear,
          periodMonth: row.periodMonth,
          totalMinor: row.totalMinor,
          dueDate: row.dueDate,
          note: row.note,
          archivedAt: row.archivedAt ? toMs(row.archivedAt) : null,
          createdAt: toMs(row.createdAt),
          updatedAt: toMs(row.updatedAt),
        })),
      );
    }

    if (data.billShares.length > 0) {
      await db.insert(billShares).values(
        data.billShares.map((row) => ({
          id: row.id,
          billId: row.billId,
          boarderId: row.boarderId,
          boarderNameSnapshot: row.boarderNameSnapshot,
          amountDueMinor: row.amountDueMinor,
          createdAt: toMs(row.createdAt),
        })),
      );
    }

    if (computedPayments.length > 0) {
      await db.insert(payments).values(
        computedPayments.map((payment) => ({
          id: payment.id,
          billShareId: payment.billShareId,
          amountMinor: payment.amountMinor,
          paidAt: payment.paidAt,
          note: payment.note,
          changeMinor: payment.computedChangeMinor,
          changeReturnedAt: payment.changeReturnedAt,
          changeReturnNote: payment.changeReturnNote,
          voidedAt: payment.voidedAt,
          voidReason: payment.voidReason,
          createdAt: payment.createdAt,
        })),
      );
    }
  });
}
