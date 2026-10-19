export type BillStatus = "open" | "settled" | "archived";
export type ShareStatus = "unpaid" | "partial" | "paid";
export type BoarderStatus = "active" | "archived";
export type ShareFilter = "all" | "unpaid" | "paid" | "change";

export interface PaymentRecord {
  id: string;
  billShareId: string;
  amountMinor: number;
  paidAt: number;
  note: string | null;
  changeMinor: number;
  changeReturnedAt: number | null;
  changeReturnNote: string | null;
  voidedAt: number | null;
  voidReason: string | null;
  createdAt: number;
}

export interface ShareRecord {
  id: string;
  billId: string;
  boarderId: string;
  boarderNameSnapshot: string;
  amountDueMinor: number;
  createdAt: number;
}

export interface PaymentWithComputedChange extends PaymentRecord {
  computedChangeMinor: number;
  isActive: boolean;
  needsReview: boolean;
}

export interface ShareState {
  paidMinor: number;
  appliedMinor: number;
  balanceMinor: number;
  changeOutstandingMinor: number;
  changeReturnedMinor: number;
  status: ShareStatus;
  payments: PaymentWithComputedChange[];
  hasReturnedChange: boolean;
  needsReview: boolean;
}

export interface ShareWithState extends ShareRecord {
  state: ShareState;
  currentBoarderName?: string;
  boarderArchived?: boolean;
}

export interface BillSummary {
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
}

export interface BoarderPeriodSummary {
  boarderId: string;
  boarderName: string;
  boarderArchived: boolean;
  expectedMinor: number;
  appliedMinor: number;
  outstandingMinor: number;
  changeOutstandingMinor: number;
  status: ShareStatus;
  shareCount: number;
}

export interface EqualSplitInput {
  boarderId: string;
  name: string;
  createdAt: number;
}

export interface SplitResult {
  boarderId: string;
  amountMinor: number;
}
