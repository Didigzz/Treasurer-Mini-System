import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useTRPC } from "@/trpc";

function useInvalidateAll() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries();
}

export function useAppSettings() {
  const trpc = useTRPC();
  return useQuery(trpc.settings.get.queryOptions());
}

export function useBillTypes() {
  const trpc = useTRPC();
  return useQuery(trpc.billTypes.list.queryOptions());
}

export function useActiveBoarders() {
  const trpc = useTRPC();
  return useQuery(trpc.boarders.active.queryOptions());
}

export function useBoardersWithBalances() {
  const trpc = useTRPC();
  return useQuery(trpc.boarders.listWithBalances.queryOptions());
}

export function useBoarderLedger(id: string | undefined) {
  const trpc = useTRPC();
  return useQuery({
    ...trpc.boarders.ledger.queryOptions({ id: id ?? "" }),
    enabled: Boolean(id),
  });
}

export function useDashboard(year: number, month: number, includeArchived: boolean) {
  const trpc = useTRPC();
  return useQuery(trpc.bills.dashboard.queryOptions({ year, month, includeArchived }));
}

export function useBillDetail(id: string | undefined) {
  const trpc = useTRPC();
  return useQuery({
    ...trpc.bills.get.queryOptions({ id: id ?? "" }),
    enabled: Boolean(id),
  });
}

export function useBillHistory() {
  const trpc = useTRPC();
  return useQuery(trpc.bills.history.queryOptions());
}

export function usePaymentContext(shareId: string | undefined) {
  const trpc = useTRPC();
  return useQuery({
    ...trpc.payments.context.queryOptions({ shareId: shareId ?? "" }),
    enabled: Boolean(shareId),
  });
}

export function useBillTypeMutations() {
  const trpc = useTRPC();
  const invalidate = useInvalidateAll();
  const create = useMutation(trpc.billTypes.create.mutationOptions({ onSuccess: invalidate }));
  const rename = useMutation(trpc.billTypes.rename.mutationOptions({ onSuccess: invalidate }));
  const setArchived = useMutation(
    trpc.billTypes.setArchived.mutationOptions({ onSuccess: invalidate }),
  );
  return { create, rename, setArchived };
}

export function useBoarderMutations() {
  const trpc = useTRPC();
  const invalidate = useInvalidateAll();
  const create = useMutation(trpc.boarders.create.mutationOptions({ onSuccess: invalidate }));
  const rename = useMutation(trpc.boarders.rename.mutationOptions({ onSuccess: invalidate }));
  const setArchived = useMutation(
    trpc.boarders.setArchived.mutationOptions({ onSuccess: invalidate }),
  );
  return { create, rename, setArchived };
}

export function useBillMutations() {
  const trpc = useTRPC();
  const invalidate = useInvalidateAll();
  const create = useMutation(trpc.bills.create.mutationOptions({ onSuccess: invalidate }));
  const update = useMutation(trpc.bills.update.mutationOptions({ onSuccess: invalidate }));
  const setArchived = useMutation(trpc.bills.setArchived.mutationOptions({ onSuccess: invalidate }));
  const updateMeta = useMutation(trpc.bills.updateMeta.mutationOptions({ onSuccess: invalidate }));
  return { create, update, setArchived, updateMeta };
}

export function usePaymentMutations() {
  const trpc = useTRPC();
  const invalidate = useInvalidateAll();
  const record = useMutation(trpc.payments.record.mutationOptions({ onSuccess: invalidate }));
  const voidPayment = useMutation(trpc.payments.void.mutationOptions({ onSuccess: invalidate }));
  const markChangeReturned = useMutation(
    trpc.payments.markChangeReturned.mutationOptions({ onSuccess: invalidate }),
  );
  return { record, voidPayment, markChangeReturned };
}

export function useSettingsMutations() {
  const trpc = useTRPC();
  const invalidate = useInvalidateAll();
  const setCurrency = useMutation(trpc.settings.setCurrency.mutationOptions({ onSuccess: invalidate }));
  const setAllowCentavos = useMutation(
    trpc.settings.setAllowCentavos.mutationOptions({ onSuccess: invalidate }),
  );
  const setLastBackupAt = useMutation(
    trpc.settings.setLastBackupAt.mutationOptions({ onSuccess: invalidate }),
  );
  const importBackup = useMutation(trpc.backup.import.mutationOptions({ onSuccess: invalidate }));
  return { setCurrency, setAllowCentavos, setLastBackupAt, importBackup };
}

export function useBackupQueries() {
  const trpc = useTRPC();
  const queryClient = useQueryClient();
  return {
    exportPayload: () => queryClient.fetchQuery(trpc.backup.export.queryOptions()),
    inspect: (json: string) =>
      queryClient.fetchQuery(trpc.backup.inspect.queryOptions({ json })),
  };
}
