import type { BillShareRow, BoarderRow, PaymentRow } from "@my-better-t-app/db/schema";

import { computeShareState } from "./calculations";
import type { PaymentRecord, ShareWithState } from "./types";

export function asPaymentRecord(row: PaymentRow): PaymentRecord {
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

export function buildShareStates(
  shareRows: BillShareRow[],
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
