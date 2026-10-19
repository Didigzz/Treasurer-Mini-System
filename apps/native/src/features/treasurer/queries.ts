import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  createBill,
  createBillType,
  createBoarder,
  findDuplicateBills,
  getActiveBoarders,
  getAppSettings,
  getBillDetail,
  getBoarderLedger,
  getDashboard,
  getSharePaymentContext,
  listBillTypes,
  listBoardersWithBalances,
  markChangeReturned,
  recordPayment,
  renameBillType,
  renameBoarder,
  setBillArchived,
  setBillTypeArchived,
  setBoarderArchived,
  setCurrencyCode,
  setLastBackupAt,
  updateBill,
  updateBillMeta,
  voidPayment,
  type CreateBillInput,
  type RecordPaymentInput,
} from "./repository";

export const treasurerKeys = {
  all: ["treasurer"] as const,
  settings: ["treasurer", "settings"] as const,
  billTypes: ["treasurer", "bill-types"] as const,
  boarders: ["treasurer", "boarders"] as const,
  activeBoarders: ["treasurer", "boarders", "active"] as const,
  dashboard: (year: number, month: number, includeArchived: boolean) =>
    ["treasurer", "dashboard", year, month, includeArchived] as const,
  bill: (id: string) => ["treasurer", "bill", id] as const,
  boarder: (id: string) => ["treasurer", "boarder", id] as const,
  paymentContext: (shareId: string) => ["treasurer", "payment-context", shareId] as const,
};

function useInvalidateTreasurer() {
  const queryClient = useQueryClient();
  return async () => {
    await queryClient.invalidateQueries({ queryKey: treasurerKeys.all });
  };
}

export function useAppSettings() {
  return useQuery({
    queryKey: treasurerKeys.settings,
    queryFn: getAppSettings,
  });
}

export function useBillTypes() {
  return useQuery({
    queryKey: treasurerKeys.billTypes,
    queryFn: listBillTypes,
  });
}

export function useBoarders() {
  return useQuery({
    queryKey: treasurerKeys.boarders,
    queryFn: listBoardersWithBalances,
  });
}

export function useActiveBoarders() {
  return useQuery({
    queryKey: treasurerKeys.activeBoarders,
    queryFn: getActiveBoarders,
  });
}

export function useDashboard(year: number, month: number, includeArchived: boolean) {
  return useQuery({
    queryKey: treasurerKeys.dashboard(year, month, includeArchived),
    queryFn: () => getDashboard(year, month, includeArchived),
  });
}

export function useBillDetail(id: string | undefined) {
  return useQuery({
    queryKey: id ? treasurerKeys.bill(id) : ["treasurer", "bill", "missing"],
    queryFn: () => getBillDetail(id!),
    enabled: Boolean(id),
  });
}

export function useBoarderLedger(id: string | undefined) {
  return useQuery({
    queryKey: id ? treasurerKeys.boarder(id) : ["treasurer", "boarder", "missing"],
    queryFn: () => getBoarderLedger(id!),
    enabled: Boolean(id),
  });
}

export function usePaymentContext(shareId: string | undefined) {
  return useQuery({
    queryKey: shareId ? treasurerKeys.paymentContext(shareId) : ["treasurer", "payment-context", "missing"],
    queryFn: () => getSharePaymentContext(shareId!),
    enabled: Boolean(shareId),
  });
}

export function useCreateBoarder() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: createBoarder,
    onSuccess: invalidate,
  });
}

export function useRenameBoarder() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => renameBoarder(id, name),
    onSuccess: invalidate,
  });
}

export function useArchiveBoarder() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, archived }: { id: string; archived: boolean }) =>
      setBoarderArchived(id, archived),
    onSuccess: invalidate,
  });
}

export function useCreateBillType() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: createBillType,
    onSuccess: invalidate,
  });
}

export function useRenameBillType() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => renameBillType(id, name),
    onSuccess: invalidate,
  });
}

export function useArchiveBillType() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, archived }: { id: string; archived: boolean }) =>
      setBillTypeArchived(id, archived),
    onSuccess: invalidate,
  });
}

export function useCreateBill() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: (input: CreateBillInput) => createBill(input),
    onSuccess: invalidate,
  });
}

export function useUpdateBill() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: CreateBillInput }) =>
      updateBill(id, input),
    onSuccess: invalidate,
  });
}

export function useArchiveBill() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ id, archived }: { id: string; archived: boolean }) =>
      setBillArchived(id, archived),
    onSuccess: invalidate,
  });
}

export function useUpdateBillMeta() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({
      id,
      dueDate,
      note,
    }: {
      id: string;
      dueDate: string | null;
      note: string | null;
    }) => updateBillMeta(id, dueDate, note),
    onSuccess: invalidate,
  });
}

export function useRecordPayment() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: (input: RecordPaymentInput) => recordPayment(input),
    onSuccess: invalidate,
  });
}

export function useMarkChangeReturned() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({
      paymentId,
      returnedAt,
      note,
    }: {
      paymentId: string;
      returnedAt: number;
      note: string | null;
    }) => markChangeReturned(paymentId, returnedAt, note),
    onSuccess: invalidate,
  });
}

export function useVoidPayment() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: ({ paymentId, reason }: { paymentId: string; reason: string }) =>
      voidPayment(paymentId, reason),
    onSuccess: invalidate,
  });
}

export function useSetCurrency() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: setCurrencyCode,
    onSuccess: invalidate,
  });
}

export function useSetLastBackupAt() {
  const invalidate = useInvalidateTreasurer();
  return useMutation({
    mutationFn: setLastBackupAt,
    onSuccess: invalidate,
  });
}

export async function checkDuplicateBills(
  billTypeId: string,
  year: number,
  month: number,
  excludeBillId?: string,
) {
  return findDuplicateBills(billTypeId, year, month, excludeBillId);
}
