import type {
  BillStatus,
  BoarderPeriodSummary,
  EqualSplitInput,
  HistoryFilter,
  HistoryPeriod,
  HistoryTotals,
  PaymentRecord,
  PaymentWithComputedChange,
  ShareFilter,
  ShareState,
  ShareStatus,
  ShareWithState,
  SplitOptions,
  SplitResult,
} from "./types";

export const MINOR_STEP_FOR_WHOLE_UNITS = 100;

export type SplitBoarder = EqualSplitInput;

export interface SplitSuccess {
  shares: SplitResult[];
  leftoverMinor?: number;
  poolCount?: number;
  /** True when whole-peso mode had to hand out a larger unit than the base share. */
  rounded?: boolean;
}

export interface SplitFailure {
  error: string;
  overByMinor?: number;
}

export type MixedSplitResult = SplitSuccess | SplitFailure;

function splitStep(options?: SplitOptions): number {
  return options?.allowCentavos === false ? MINOR_STEP_FOR_WHOLE_UNITS : 1;
}

export function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

/**
 * Split equally, giving the leftover units one at a time to the first boarders
 * in the caller's order (their selection order in the picker). With centavos
 * disabled the leftover is handed out in whole pesos instead.
 */
export function equalSplit(
  totalMinor: number,
  boarders: EqualSplitInput[],
  options?: SplitOptions,
): MixedSplitResult {
  if (boarders.length === 0) {
    return { error: "Select at least one boarder." };
  }

  const step = splitStep(options);

  if (totalMinor < boarders.length * step) {
    return { error: "The bill total is too small to split among the selected boarders." };
  }

  const base = Math.floor(totalMinor / (boarders.length * step)) * step;
  const remainderUnits = (totalMinor - base * boarders.length) / step;

  return {
    shares: boarders.map((boarder, index) => ({
      boarderId: boarder.boarderId,
      amountMinor: base + (index < remainderUnits ? step : 0),
    })),
  };
}

/**
 * Mixed split: boarders with an override keep their typed amount, and the
 * leftover is divided among everyone else in selection order. A boarder may
 * end up with zero when the overrides consume the whole bill.
 */
export function splitWithOverrides(
  totalMinor: number,
  boarders: EqualSplitInput[],
  overrides: Map<string, number>,
  options?: SplitOptions,
): MixedSplitResult {
  if (boarders.length === 0) {
    return { error: "Select at least one boarder." };
  }

  if (totalMinor <= 0) {
    return { error: "Enter a bill total greater than zero." };
  }

  const step = splitStep(options);
  const pinned = new Map<string, number>();

  for (const boarder of boarders) {
    const override = overrides.get(boarder.boarderId);
    if (override === undefined) continue;

    if (!Number.isInteger(override) || override < 0) {
      return { error: "Amounts cannot be negative." };
    }
    if (override % step !== 0) {
      return { error: "Whole pesos only. Turn on centavos to use decimals." };
    }
    pinned.set(boarder.boarderId, override);
  }

  const customTotal = sum([...pinned.values()]);
  const leftoverMinor = totalMinor - customTotal;

  if (leftoverMinor < 0) {
    return { error: "Custom amounts are over the bill total.", overByMinor: -leftoverMinor };
  }

  const pool = boarders.filter((boarder) => !pinned.has(boarder.boarderId));

  if (pool.length === 0) {
    if (leftoverMinor !== 0) {
      return { error: "The assigned shares are short of the bill total." };
    }

    return {
      shares: boarders.map((boarder) => ({
        boarderId: boarder.boarderId,
        amountMinor: pinned.get(boarder.boarderId) ?? 0,
      })),
      leftoverMinor: 0,
      poolCount: 0,
      rounded: false,
    };
  }

  const base = Math.floor(leftoverMinor / (pool.length * step)) * step;
  const remainderUnits = (leftoverMinor - base * pool.length) / step;

  if (!Number.isInteger(remainderUnits)) {
    return { error: "Whole pesos only. Turn on centavos to use decimals." };
  }

  const autoShares = new Map(
    pool.map((boarder, index) => [
      boarder.boarderId,
      base + (index < remainderUnits ? step : 0),
    ]),
  );

  return {
    shares: boarders.map((boarder) => ({
      boarderId: boarder.boarderId,
      amountMinor: pinned.get(boarder.boarderId) ?? autoShares.get(boarder.boarderId) ?? 0,
    })),
    leftoverMinor,
    poolCount: pool.length,
    rounded: step > 1 && remainderUnits > 0,
  };
}

export function validateCustomShares(
  totalMinor: number,
  shares: SplitResult[],
): { valid: true } | { valid: false; differenceMinor: number; error: string } {
  if (shares.length === 0) {
    return { valid: false, differenceMinor: totalMinor, error: "Select at least one boarder." };
  }

  if (shares.some((share) => share.amountMinor < 0)) {
    return {
      valid: false,
      differenceMinor: totalMinor - sum(shares.map((share) => share.amountMinor)),
      error: "Amounts cannot be negative.",
    };
  }

  const assigned = sum(shares.map((share) => share.amountMinor));
  const differenceMinor = totalMinor - assigned;

  if (differenceMinor !== 0) {
    return {
      valid: false,
      differenceMinor,
      error:
        differenceMinor > 0
          ? "The assigned shares are short of the bill total."
          : "The assigned shares are over the bill total.",
    };
  }

  return { valid: true };
}

export function sortPaymentsChronologically(payments: PaymentRecord[]): PaymentRecord[] {
  return [...payments].sort((a, b) => {
    if (a.paidAt !== b.paidAt) return a.paidAt - b.paidAt;
    if (a.createdAt !== b.createdAt) return a.createdAt - b.createdAt;
    return a.id.localeCompare(b.id);
  });
}

/**
 * Assign the overpayment portion to the payment that crosses the amount due.
 * Voided payments are retained for history but never contribute to totals.
 */
export function recomputePaymentChanges(
  amountDueMinor: number,
  payments: PaymentRecord[],
): PaymentWithComputedChange[] {
  const ordered = sortPaymentsChronologically(payments);
  let running = 0;

  return ordered.map((payment) => {
    const isActive = payment.voidedAt === null;
    if (!isActive) {
      return {
        ...payment,
        computedChangeMinor: 0,
        isActive: false,
        needsReview: false,
      };
    }

    const before = Math.max(0, running - amountDueMinor);
    running += payment.amountMinor;
    const after = Math.max(0, running - amountDueMinor);
    const computedChangeMinor = Math.max(0, after - before);
    const returnedAmount = payment.changeReturnedAt !== null ? payment.changeMinor : 0;

    return {
      ...payment,
      computedChangeMinor,
      isActive: true,
      needsReview: payment.changeReturnedAt !== null && computedChangeMinor < returnedAmount,
    };
  });
}

export function computeShareState(amountDueMinor: number, payments: PaymentRecord[]): ShareState {
  const withChanges = recomputePaymentChanges(amountDueMinor, payments);
  const active = withChanges.filter((payment) => payment.isActive);
  const paidMinor = sum(active.map((payment) => payment.amountMinor));

  const changeOutstandingMinor = sum(
    active
      .filter((payment) => payment.changeReturnedAt === null)
      .map((payment) => payment.computedChangeMinor),
  );
  const changeReturnedMinor = sum(
    active
      .filter((payment) => payment.changeReturnedAt !== null)
      .map((payment) => payment.computedChangeMinor),
  );

  return {
    paidMinor,
    appliedMinor: Math.min(paidMinor, amountDueMinor),
    balanceMinor: Math.max(0, amountDueMinor - paidMinor),
    changeOutstandingMinor,
    changeReturnedMinor,
    status: shareStatus(amountDueMinor, paidMinor),
    payments: withChanges,
    hasReturnedChange: active.some((payment) => payment.changeReturnedAt !== null),
    needsReview: active.some((payment) => payment.needsReview),
  };
}

export function shareStatus(amountDueMinor: number, paidMinor: number): ShareStatus {
  if (paidMinor >= amountDueMinor) return "paid";
  if (paidMinor > 0) return "partial";
  return "unpaid";
}

export function computeBillSummary(shares: ShareWithState[], archivedAt: number | null): {
  expectedMinor: number;
  appliedMinor: number;
  outstandingMinor: number;
  cashReceivedMinor: number;
  changeOutstandingMinor: number;
  changeReturnedMinor: number;
  progress: number;
  paidShareCount: number;
  partialShareCount: number;
  unpaidShareCount: number;
  totalShareCount: number;
  status: BillStatus;
  needsReview: boolean;
} {
  const expectedMinor = sum(shares.map((share) => share.amountDueMinor));
  const appliedMinor = sum(shares.map((share) => share.state.appliedMinor));
  const cashReceivedMinor = sum(shares.map((share) => share.state.paidMinor));
  const changeOutstandingMinor = sum(
    shares.map((share) => share.state.changeOutstandingMinor),
  );
  const changeReturnedMinor = sum(shares.map((share) => share.state.changeReturnedMinor));
  const paidShareCount = shares.filter((share) => share.state.balanceMinor === 0).length;
  const partialShareCount = shares.filter(
    (share) => share.state.balanceMinor > 0 && share.state.paidMinor > 0,
  ).length;
  const unpaidShareCount = shares.filter((share) => share.state.paidMinor === 0).length;
  const settled = shares.length > 0 && shares.every((share) => share.state.balanceMinor === 0);

  return {
    expectedMinor,
    appliedMinor,
    outstandingMinor: expectedMinor - appliedMinor,
    cashReceivedMinor,
    changeOutstandingMinor,
    changeReturnedMinor,
    progress: expectedMinor === 0 ? 0 : Math.min(1, appliedMinor / expectedMinor),
    paidShareCount,
    partialShareCount,
    unpaidShareCount,
    totalShareCount: shares.length,
    status: archivedAt !== null ? "archived" : settled ? "settled" : "open",
    needsReview: shares.some((share) => share.state.needsReview),
  };
}

export function computeBoarderPeriodSummaries(
  shares: Array<{
    boarderId: string;
    boarderName: string;
    boarderArchived: boolean;
    amountDueMinor: number;
    state: ShareState;
  }>,
): BoarderPeriodSummary[] {
  const grouped = new Map<string, BoarderPeriodSummary>();

  for (const item of shares) {
    const existing = grouped.get(item.boarderId);
    const expected = (existing?.expectedMinor ?? 0) + item.amountDueMinor;
    const applied = (existing?.appliedMinor ?? 0) + item.state.appliedMinor;
    const change = (existing?.changeOutstandingMinor ?? 0) + item.state.changeOutstandingMinor;

    grouped.set(item.boarderId, {
      boarderId: item.boarderId,
      boarderName: item.boarderName,
      boarderArchived: item.boarderArchived,
      expectedMinor: expected,
      appliedMinor: applied,
      outstandingMinor: Math.max(0, expected - applied),
      changeOutstandingMinor: change,
      status:
        applied >= expected ? "paid" : applied > 0 ? "partial" : "unpaid",
      shareCount: (existing?.shareCount ?? 0) + 1,
    });
  }

  return [...grouped.values()].sort((a, b) =>
    a.boarderName.localeCompare(b.boarderName, undefined, { sensitivity: "base" }),
  );
}

export function matchesShareFilter(
  share: Pick<ShareWithState, "state">,
  filter: ShareFilter,
): boolean {
  switch (filter) {
    case "all":
      return true;
    case "unpaid":
      return share.state.balanceMinor > 0;
    case "paid":
      return share.state.balanceMinor === 0;
    case "change":
      return share.state.changeOutstandingMinor > 0;
  }
}

export function matchesBoarderFilter(
  summary: BoarderPeriodSummary,
  filter: ShareFilter,
): boolean {
  switch (filter) {
    case "all":
      return true;
    case "unpaid":
      return summary.outstandingMinor > 0;
    case "paid":
      return summary.outstandingMinor === 0;
    case "change":
      return summary.changeOutstandingMinor > 0;
  }
}

export interface PaymentPreview {
  paidAfterMinor: number;
  balanceAfterMinor: number;
  changeFromThisPaymentMinor: number;
  changeOutstandingAfterMinor: number;
  message: string;
}

export function previewPayment(
  amountDueMinor: number,
  existingPayments: PaymentRecord[],
  amountMinor: number,
  now = Date.now(),
): PaymentPreview {
  const previewPayment: PaymentRecord = {
    id: "__preview__",
    billShareId: "preview",
    amountMinor,
    paidAt: now,
    note: null,
    changeMinor: 0,
    changeReturnedAt: null,
    changeReturnNote: null,
    voidedAt: null,
    voidReason: null,
    createdAt: now,
  };

  const state = computeShareState(amountDueMinor, [...existingPayments, previewPayment]);
  const previewComputed = state.payments.find((payment) => payment.id === "__preview__");
  const changeFromThisPaymentMinor = previewComputed?.computedChangeMinor ?? 0;

  let message: string;
  if (state.balanceMinor > 0) {
    message = "still owed";
  } else if (changeFromThisPaymentMinor > 0) {
    message = "change to return";
  } else {
    message = "paid in full";
  }

  return {
    paidAfterMinor: state.paidMinor,
    balanceAfterMinor: state.balanceMinor,
    changeFromThisPaymentMinor,
    changeOutstandingAfterMinor: state.changeOutstandingMinor,
    message,
  };
}

export function billStatusLabel(status: BillStatus): string {
  switch (status) {
    case "open":
      return "Open";
    case "settled":
      return "Settled";
    case "archived":
      return "Archived";
  }
}

export function shareStatusLabel(status: ShareStatus): string {
  switch (status) {
    case "paid":
      return "Paid";
    case "partial":
      return "Partial";
    case "unpaid":
      return "Unpaid";
  }
}

export interface HistoryBillRef {
  id: string;
  billTypeId: string;
  periodYear: number;
  periodMonth: number;
  createdAt: number;
}

export function matchesHistoryFilter(
  bill: Pick<HistoryBillRef, "billTypeId" | "periodYear" | "periodMonth">,
  filter: HistoryFilter,
): boolean {
  if (filter.year !== null && bill.periodYear !== filter.year) return false;
  if (filter.month !== null && bill.periodMonth !== filter.month) return false;
  if (filter.billTypeId !== null && bill.billTypeId !== filter.billTypeId) return false;
  return true;
}

/** Newest period first, then the most recently created bill inside the month. */
export function sortHistoryBills<
  T extends { bill: Pick<HistoryBillRef, "id" | "periodYear" | "periodMonth" | "createdAt"> },
>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    if (a.bill.periodYear !== b.bill.periodYear) return b.bill.periodYear - a.bill.periodYear;
    if (a.bill.periodMonth !== b.bill.periodMonth) return b.bill.periodMonth - a.bill.periodMonth;
    if (a.bill.createdAt !== b.bill.createdAt) return b.bill.createdAt - a.bill.createdAt;
    return a.bill.id.localeCompare(b.bill.id);
  });
}

export function listHistoryPeriods(
  bills: Array<Pick<HistoryBillRef, "periodYear" | "periodMonth">>,
): HistoryPeriod[] {
  const periods = new Map<string, HistoryPeriod>();

  for (const bill of bills) {
    const key = `${bill.periodYear}-${bill.periodMonth}`;
    const existing = periods.get(key);
    if (existing) existing.billCount += 1;
    else periods.set(key, { year: bill.periodYear, month: bill.periodMonth, billCount: 1 });
  }

  return [...periods.values()].sort((a, b) => {
    if (a.year !== b.year) return b.year - a.year;
    return b.month - a.month;
  });
}

export function computeHistoryTotals(summaries: HistoryTotals[]): HistoryTotals {
  return {
    expectedMinor: sum(summaries.map((summary) => summary.expectedMinor)),
    appliedMinor: sum(summaries.map((summary) => summary.appliedMinor)),
    outstandingMinor: sum(summaries.map((summary) => summary.outstandingMinor)),
    changeOutstandingMinor: sum(
      summaries.map((summary) => summary.changeOutstandingMinor),
    ),
  };
}
