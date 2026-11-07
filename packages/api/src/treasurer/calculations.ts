import type {
  BillStatus,
  BoarderPeriodSummary,
  EqualSplitInput,
  PaymentRecord,
  PaymentWithComputedChange,
  ShareFilter,
  ShareState,
  ShareStatus,
  ShareWithState,
  SplitResult,
} from "./types";

export function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

/**
 * Split equally, giving the leftover centavos one at a time to the first
 * boarders in the caller's order (their selection order in the picker).
 */
export function equalSplit(
  totalMinor: number,
  boarders: EqualSplitInput[],
): { shares: SplitResult[] } | { error: string } {
  if (boarders.length === 0) {
    return { error: "Select at least one boarder." };
  }

  if (totalMinor < boarders.length) {
    return { error: "The bill total is too small to split among the selected boarders." };
  }

  const base = Math.floor(totalMinor / boarders.length);
  const remainder = totalMinor % boarders.length;

  return {
    shares: boarders.map((boarder, index) => ({
      boarderId: boarder.boarderId,
      amountMinor: base + (index < remainder ? 1 : 0),
    })),
  };
}

export function validateCustomShares(
  totalMinor: number,
  shares: SplitResult[],
): { valid: true } | { valid: false; differenceMinor: number; error: string } {
  if (shares.length === 0) {
    return { valid: false, differenceMinor: totalMinor, error: "Select at least one boarder." };
  }

  if (shares.some((share) => share.amountMinor <= 0)) {
    return {
      valid: false,
      differenceMinor: totalMinor - sum(shares.map((share) => share.amountMinor)),
      error: "Every selected boarder needs an amount of at least ₱0.01.",
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
