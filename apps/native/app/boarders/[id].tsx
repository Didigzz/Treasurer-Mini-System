import { Ionicons } from "@expo/vector-icons";
import { router, useLocalSearchParams } from "expo-router";
import { Button, Surface, useThemeColor } from "heroui-native";
import { useMemo, useState } from "react";
import { Text, View } from "react-native";

import {
  AppDialog,
  AppInput,
  ConfirmDialog,
  EmptyState,
  ErrorBlock,
  FormField,
  LoadingBlock,
  Screen,
  SectionHeader,
  ShareStatusBadge,
  SummaryCard,
} from "@/src/features/treasurer/components/ui";
import { PaymentRow } from "@/src/features/treasurer/components/rows";
import { formatMoney, formatPeriod } from "@/src/features/treasurer/money";
import {
  useAppSettings,
  useArchiveBoarder,
  useBoarderLedger,
  useBoarders,
  useMarkChangeReturned,
  useRenameBoarder,
  useVoidPayment,
} from "@/src/features/treasurer/queries";

export default function BoarderDetailScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const ledger = useBoarderLedger(id);
  const settings = useAppSettings();
  const boarders = useBoarders();
  const rename = useRenameBoarder();
  const archive = useArchiveBoarder();
  const markChange = useMarkChangeReturned();
  const voidPayment = useVoidPayment();

  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");

  const [renameOpen, setRenameOpen] = useState(false);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | undefined>();
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [changeTarget, setChangeTarget] = useState<string | null>(null);
  const [changeNote, setChangeNote] = useState("");
  const [voidTarget, setVoidTarget] = useState<string | null>(null);
  const [voidReason, setVoidReason] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);

  const currency = settings.data?.currencyCode ?? "PHP";
  const boarder = ledger.data?.boarder;
  const isArchived = boarder?.status === "archived";
  const duplicateName = useMemo(() => {
    const trimmed = name.trim().toLowerCase();
    if (!trimmed || !boarder) return false;
    return (
      trimmed !== boarder.name.trim().toLowerCase() &&
      (boarders.data ?? []).some(
        (other) => other.id !== boarder.id && other.name.trim().toLowerCase() === trimmed,
      )
    );
  }, [name, boarder, boarders.data]);

  function openRename() {
    if (!boarder) return;
    setName(boarder.name);
    setNameError(undefined);
    setRenameOpen(true);
  }

  async function handleRename() {
    if (!boarder) return;
    const trimmed = name.trim();
    if (!trimmed) {
      setNameError("Enter a boarder name.");
      return;
    }
    try {
      await rename.mutateAsync({ id: boarder.id, name: trimmed });
      setRenameOpen(false);
    } catch (error) {
      setNameError(error instanceof Error ? error.message : "The name could not be saved.");
    }
  }

  async function handleArchiveToggle() {
    if (!boarder) return;
    setActionError(null);
    try {
      await archive.mutateAsync({ id: boarder.id, archived: !isArchived });
      setArchiveOpen(false);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The boarder could not be updated.");
      setArchiveOpen(false);
    }
  }

  async function handleMarkChange() {
    if (!changeTarget) return;
    setActionError(null);
    try {
      await markChange.mutateAsync({
        paymentId: changeTarget,
        returnedAt: Date.now(),
        note: changeNote.trim() || null,
      });
      setChangeTarget(null);
      setChangeNote("");
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Change could not be marked returned.");
      setChangeTarget(null);
    }
  }

  async function handleVoid() {
    if (!voidTarget) return;
    const reason = voidReason.trim();
    if (!reason) {
      setActionError("Enter a reason for voiding this payment.");
      return;
    }
    setActionError(null);
    try {
      await voidPayment.mutateAsync({ paymentId: voidTarget, reason });
      setVoidTarget(null);
      setVoidReason("");
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The payment could not be voided.");
    }
  }

  if (ledger.isLoading) {
    return (
      <Screen>
        <LoadingBlock label="Loading boarder history…" />
      </Screen>
    );
  }

  if (ledger.isError || !ledger.data) {
    return (
      <Screen>
        <ErrorBlock
          message="This boarder could not be loaded."
          onRetry={() => void ledger.refetch()}
        />
      </Screen>
    );
  }

  const data = ledger.data;

  return (
    <Screen>
      <View className="pt-4">
        <View className="flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Text className="text-2xl font-semibold text-foreground">{boarder?.name}</Text>
            <Text className="mt-1 text-sm text-muted">
              {isArchived ? "Archived boarder · history kept" : "Active boarder"}
            </Text>
          </View>
          <Button size="sm" variant="secondary" onPress={openRename}>
            <Button.Label>Rename</Button.Label>
          </Button>
        </View>

        <View className="mt-4 gap-2">
          <View className="flex-row gap-2">
            <SummaryCard
              label="Expected"
              value={formatMoney(data.summary.expectedMinor, currency)}
              icon="receipt-outline"
            />
            <SummaryCard
              label="Paid"
              value={formatMoney(data.summary.appliedMinor, currency)}
              tone="success"
              icon="checkmark-circle-outline"
            />
          </View>
          <View className="flex-row gap-2">
            <SummaryCard
              label="Still owed"
              value={formatMoney(data.summary.outstandingMinor, currency)}
              tone={data.summary.outstandingMinor > 0 ? "danger" : "success"}
              icon="alert-circle-outline"
            />
            <SummaryCard
              label="Change to return"
              value={formatMoney(data.summary.changeOutstandingMinor, currency)}
              tone={data.summary.changeOutstandingMinor > 0 ? "warning" : "default"}
              icon="cash-outline"
            />
          </View>
        </View>
      </View>

      {actionError ? (
        <View className="mt-4">
          <ErrorBlock message={actionError} />
        </View>
      ) : null}

      <SectionHeader title="Bills and shares" subtitle="All periods, including archived bills." />
      {data.items.length === 0 ? (
        <EmptyState
          icon="document-text-outline"
          title="No bills include this boarder"
          description="Create a bill and select this boarder to start their history."
          actionLabel="Go to dashboard"
          onAction={() => router.push("/")}
        />
      ) : (
        data.items.map((item) => (
          <Surface key={item.share.id} variant="secondary" className="mb-2 rounded-2xl p-3.5">
            <View className="flex-row items-start justify-between gap-3">
              <View className="flex-1">
                <Text className="text-sm font-semibold text-foreground">
                  {item.bill.billTypeNameSnapshot}
                </Text>
                <Text className="mt-0.5 text-xs text-muted">
                  {formatPeriod(item.bill.periodYear, item.bill.periodMonth)}
                  {item.bill.archivedAt ? " · Archived bill" : ""}
                </Text>
                <Text className="mt-1 text-xs text-muted">
                  Due {formatMoney(item.share.amountDueMinor, currency)} · Paid{" "}
                  {formatMoney(item.state.paidMinor, currency)}
                  {item.state.balanceMinor > 0
                    ? ` · ${formatMoney(item.state.balanceMinor, currency)} left`
                    : " · Paid"}
                </Text>
              </View>
              <View className="items-end gap-1.5">
                <ShareStatusBadge status={item.state.status} />
                {item.state.changeOutstandingMinor > 0 ? (
                  <Text className="text-xs text-warning">
                    {formatMoney(item.state.changeOutstandingMinor, currency)} change
                  </Text>
                ) : null}
              </View>
            </View>
            <View className="mt-2 flex-row justify-end">
              <Button
                size="sm"
                variant={item.state.balanceMinor > 0 ? "primary" : "secondary"}
                onPress={() =>
                  router.push({
                    pathname: "/payments/new",
                    params: { shareId: item.share.id },
                  })
                }
              >
                <Button.Label>
                  {item.state.balanceMinor > 0 ? "Record payment" : "Add payment"}
                </Button.Label>
              </Button>
            </View>
          </Surface>
        ))
      )}

      <SectionHeader title="Payment history" subtitle="Newest first. Voided entries stay visible." />
      <Surface variant="secondary" className="rounded-2xl px-4">
        {data.payments.length === 0 ? (
          <View className="py-5">
            <Text className="text-center text-sm text-muted">No payments recorded yet.</Text>
          </View>
        ) : (
          data.payments.map(({ payment }) => (
            <PaymentRow
              key={payment.id}
              payment={payment}
              currency={currency}
              boarderName={boarder?.name ?? ""}
              onMarkChangeReturned={() => {
                setChangeTarget(payment.id);
                setChangeNote("");
              }}
              onVoid={() => {
                setVoidTarget(payment.id);
                setVoidReason("");
              }}
            />
          ))
        )}
      </Surface>

      <Button
        variant={isArchived ? "secondary" : "danger-soft"}
        className="mt-6"
        onPress={() => setArchiveOpen(true)}
      >
        <Ionicons name={isArchived ? "refresh" : "archive-outline"} size={17} color={muted} />
        <Button.Label>{isArchived ? "Restore to active" : "Archive boarder"}</Button.Label>
      </Button>

      <AppDialog
        open={renameOpen}
        onOpenChange={setRenameOpen}
        title="Rename boarder"
        description="Old bills keep the name they were created with."
        actions={
          <>
            <Button variant="ghost" size="sm" onPress={() => setRenameOpen(false)}>
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button size="sm" isDisabled={rename.isPending} onPress={() => void handleRename()}>
              <Button.Label>{rename.isPending ? "Saving…" : "Save"}</Button.Label>
            </Button>
          </>
        }
      >
        <FormField label="Name" error={nameError} required>
          <AppInput
            autoFocus
            value={name}
            onChangeText={(value) => {
              setName(value);
              if (nameError) setNameError(undefined);
            }}
            maxLength={80}
            returnKeyType="done"
            onSubmitEditing={() => void handleRename()}
          />
          {duplicateName ? (
            <Text className="mt-1.5 text-xs text-warning">
              Another boarder already has this name. You can still use it.
            </Text>
          ) : null}
        </FormField>
      </AppDialog>

      <ConfirmDialog
        open={archiveOpen}
        onOpenChange={setArchiveOpen}
        title={isArchived ? "Restore this boarder?" : "Archive this boarder?"}
        description={
          isArchived
            ? "They will be selectable for new bills again. Their history is unchanged."
            : data.summary.outstandingMinor > 0
              ? `This boarder still owes ${formatMoney(data.summary.outstandingMinor, currency)}. Archiving keeps that debt in their history and on unpaid bills; they just won't appear in new bills.`
              : "They will disappear from new bills, but all past bills and payments stay intact."
        }
        confirmLabel={isArchived ? "Restore" : "Archive"}
        destructive={!isArchived}
        pending={archive.isPending}
        onConfirm={() => void handleArchiveToggle()}
      />

      <AppDialog
        open={changeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setChangeTarget(null);
        }}
        title="Mark change returned"
        description="This records the return time and optional note. It does not change the payment amount."
        actions={
          <>
            <Button variant="ghost" size="sm" onPress={() => setChangeTarget(null)}>
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button size="sm" isDisabled={markChange.isPending} onPress={() => void handleMarkChange()}>
              <Button.Label>{markChange.isPending ? "Saving…" : "Mark returned"}</Button.Label>
            </Button>
          </>
        }
      >
        <FormField label="Note (optional)" hint="For example: “Returned in cash at the gate.”">
          <AppInput
            value={changeNote}
            onChangeText={setChangeNote}
            maxLength={200}
            placeholder="Optional note"
          />
        </FormField>
      </AppDialog>

      <AppDialog
        open={voidTarget !== null}
        onOpenChange={(open) => {
          if (!open) setVoidTarget(null);
        }}
        title="Void this payment?"
        description="The payment stays in history with your reason. It is excluded from totals. Record a new payment if needed."
        actions={
          <>
            <Button variant="ghost" size="sm" onPress={() => setVoidTarget(null)}>
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button
              variant="danger"
              size="sm"
              isDisabled={voidPayment.isPending}
              onPress={() => void handleVoid()}
            >
              <Button.Label>{voidPayment.isPending ? "Voiding…" : "Void payment"}</Button.Label>
            </Button>
          </>
        }
      >
        <FormField label="Reason" required>
          <AppInput
            value={voidReason}
            onChangeText={setVoidReason}
            maxLength={200}
            placeholder="e.g. Entered the wrong amount"
          />
        </FormField>
      </AppDialog>
    </Screen>
  );
}
