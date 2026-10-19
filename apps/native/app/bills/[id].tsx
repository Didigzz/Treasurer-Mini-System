import { Ionicons } from "@expo/vector-icons";
import { router, useLocalSearchParams } from "expo-router";
import { Button, Surface, useThemeColor } from "heroui-native";
import { useMemo, useState } from "react";
import { Text, View } from "react-native";

import { matchesShareFilter } from "@/src/features/treasurer/calculations";
import { PaymentRow, ShareRow } from "@/src/features/treasurer/components/rows";
import {
  AppDialog,
  AppInput,
  BillStatusBadge,
  ConfirmDialog,
  EmptyState,
  ErrorBlock,
  FilterChips,
  FormField,
  LoadingBlock,
  Screen,
  SectionHeader,
  StatusBadge,
  SummaryCard,
} from "@/src/features/treasurer/components/ui";
import {
  daysOverdue,
  formatDateOnly,
  formatMoney,
  formatPeriod,
  isOverdue,
} from "@/src/features/treasurer/money";
import {
  useAppSettings,
  useArchiveBill,
  useBillDetail,
  useMarkChangeReturned,
  useVoidPayment,
} from "@/src/features/treasurer/queries";
import type { ShareFilter } from "@/src/features/treasurer/types";

export default function BillDetailScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const detail = useBillDetail(id);
  const settings = useAppSettings();
  const archive = useArchiveBill();
  const markChange = useMarkChangeReturned();
  const voidPayment = useVoidPayment();

  const muted = useThemeColor("muted");

  const [filter, setFilter] = useState<ShareFilter>("all");
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [changeTarget, setChangeTarget] = useState<string | null>(null);
  const [changeNote, setChangeNote] = useState("");
  const [voidTarget, setVoidTarget] = useState<string | null>(null);
  const [voidReason, setVoidReason] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);

  const currency = settings.data?.currencyCode ?? "PHP";
  const data = detail.data;
  const bill = data?.bill;
  const isArchived = bill?.archivedAt !== null && bill?.archivedAt !== undefined;

  const filteredShares = useMemo(
    () => (data?.shares ?? []).filter((share) => matchesShareFilter(share, filter)),
    [data, filter],
  );

  const counts = useMemo(() => {
    const shares = data?.shares ?? [];
    return {
      all: shares.length,
      unpaid: shares.filter((share) => matchesShareFilter(share, "unpaid")).length,
      paid: shares.filter((share) => matchesShareFilter(share, "paid")).length,
      change: shares.filter((share) => matchesShareFilter(share, "change")).length,
    };
  }, [data]);

  const paymentRows = useMemo(() => {
    if (!data) return [];
    const names = new Map(
      data.shares.map((share) => [
        share.id,
        share.currentBoarderName ?? share.boarderNameSnapshot,
      ]),
    );
    return data.shares
      .flatMap((share) => share.state.payments)
      .sort((a, b) => b.paidAt - a.paidAt)
      .map((payment) => ({
        payment,
        boarderName: names.get(payment.billShareId) ?? "Boarder",
      }));
  }, [data]);

  async function handleArchiveToggle() {
    if (!bill) return;
    setActionError(null);
    try {
      await archive.mutateAsync({ id: bill.id, archived: !isArchived });
      setArchiveOpen(false);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The bill could not be updated.");
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

  if (detail.isLoading) {
    return (
      <Screen>
        <LoadingBlock label="Loading bill…" />
      </Screen>
    );
  }

  if (detail.isError || !data || !bill) {
    return (
      <Screen>
        <ErrorBlock message="This bill could not be loaded." onRetry={() => void detail.refetch()} />
      </Screen>
    );
  }

  const overdue = isOverdue(bill.dueDate, data.summary.outstandingMinor > 0);
  const renamed =
    data.currentTypeName.localeCompare(bill.billTypeNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;

  return (
    <Screen>
      <View className="pt-4">
        <View className="flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Text className="text-2xl font-semibold text-foreground">
              {bill.billTypeNameSnapshot}
            </Text>
            {renamed ? (
              <Text className="mt-0.5 text-xs text-muted">Now called {data.currentTypeName}</Text>
            ) : null}
            <Text className="mt-1 text-sm text-muted">
              {formatPeriod(bill.periodYear, bill.periodMonth)}
            </Text>
          </View>
          <BillStatusBadge status={data.summary.status} />
        </View>

        <View className="mt-3 flex-row flex-wrap items-center gap-2">
          <Text className="text-sm text-muted">
            Total {formatMoney(bill.totalMinor, currency)}
          </Text>
          {bill.dueDate ? (
            <Text className="text-xs text-muted">Due {formatDateOnly(bill.dueDate)}</Text>
          ) : null}
          {overdue && bill.dueDate ? (
            <StatusBadge
              label={`Overdue ${daysOverdue(bill.dueDate)}d`}
              tone="danger"
              icon="calendar-outline"
            />
          ) : null}
          {data.summary.needsReview ? (
            <StatusBadge label="Review change" tone="warning" icon="alert-circle-outline" />
          ) : null}
        </View>

        {bill.note ? (
          <Surface variant="secondary" className="mt-3 rounded-xl p-3">
            <Text className="text-sm text-foreground">{bill.note}</Text>
          </Surface>
        ) : null}

        {data.summary.changeOutstandingMinor > 0 ? (
          <View className="mt-3 rounded-xl border border-warning/30 bg-warning/10 p-3">
            <Text className="text-sm font-medium text-warning">
              {formatMoney(data.summary.changeOutstandingMinor, currency)} change still to return
            </Text>
            <Text className="mt-1 text-xs text-muted">
              The bill can be settled while change is outstanding. Return it from the payment
              history below.
            </Text>
          </View>
        ) : null}

        {isArchived ? (
          <View className="mt-3 rounded-xl border border-separator bg-surface p-3">
            <Text className="text-sm text-muted">
              This bill is archived. Unarchive it to record or correct payments.
            </Text>
          </View>
        ) : null}

        <View className="mt-4 gap-2">
          <View className="flex-row gap-2">
            <SummaryCard
              label="Expected"
              value={formatMoney(data.summary.expectedMinor, currency)}
              icon="receipt-outline"
            />
            <SummaryCard
              label="Received"
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
              label="Change"
              value={formatMoney(data.summary.changeOutstandingMinor, currency)}
              tone={data.summary.changeOutstandingMinor > 0 ? "warning" : "default"}
              icon="cash-outline"
            />
          </View>
        </View>

        <View className="mt-3 flex-row gap-2">
          <Button
            size="sm"
            variant="secondary"
            className="flex-1"
            onPress={() =>
              router.push({ pathname: "/bills/new", params: { id: bill.id } })
            }
          >
            <Ionicons name="create-outline" size={15} color={muted} />
            <Button.Label>{data.isLocked ? "Edit details" : "Edit bill"}</Button.Label>
          </Button>
          <Button
            size="sm"
            variant={isArchived ? "secondary" : "danger-soft"}
            className="flex-1"
            onPress={() => setArchiveOpen(true)}
          >
            <Ionicons
              name={isArchived ? "refresh" : "archive-outline"}
              size={15}
              color={muted}
            />
            <Button.Label>{isArchived ? "Unarchive" : "Archive"}</Button.Label>
          </Button>
        </View>
      </View>

      {actionError ? (
        <View className="mt-4">
          <ErrorBlock message={actionError} />
        </View>
      ) : null}

      <SectionHeader
        title={`Boarders (${data.shares.length})`}
        subtitle="Paid means the share is covered; change is tracked separately."
      />
      {data.shares.length === 0 ? (
        <EmptyState
          icon="people-outline"
          title="No boarders on this bill"
          description="This should not happen; recreate the bill with at least one boarder."
        />
      ) : (
        <>
          <FilterChips value={filter} onChange={setFilter} counts={counts} />
          <View className="mt-3">
            {filteredShares.length === 0 ? (
              <EmptyState
                icon="filter-outline"
                title="No boarders match this filter"
                description="Try another filter or clear it."
                actionLabel="Show all"
                onAction={() => setFilter("all")}
              />
            ) : (
              filteredShares.map((share) => (
                <ShareRow
                  key={share.id}
                  share={share}
                  currency={currency}
                  readOnly={isArchived}
                  onRecordPayment={() =>
                    router.push({
                      pathname: "/payments/new",
                      params: { shareId: share.id },
                    })
                  }
                />
              ))
            )}
          </View>
        </>
      )}

      <SectionHeader title="Payment history" subtitle="Newest first. Voided entries stay visible." />
      <Surface variant="secondary" className="rounded-2xl px-4">
        {paymentRows.length === 0 ? (
          <View className="py-5">
            <Text className="text-center text-sm text-muted">No payments recorded yet.</Text>
          </View>
        ) : (
          paymentRows.map(({ payment, boarderName }) => (
            <PaymentRow
              key={payment.id}
              payment={payment}
              currency={currency}
              boarderName={boarderName}
              readOnly={isArchived}
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


      <ConfirmDialog
        open={archiveOpen}
        onOpenChange={setArchiveOpen}
        title={isArchived ? "Unarchive this bill?" : "Archive this bill?"}
        description={
          isArchived
            ? "It will return to the default period list and totals. All records are unchanged."
            : data.summary.outstandingMinor > 0
              ? `This bill still has ${formatMoney(data.summary.outstandingMinor, currency)} outstanding. Archiving hides it from default totals but keeps the debt in boarder history.`
              : "It will disappear from the default period list and totals. All records stay intact and you can unarchive it later."
        }
        confirmLabel={isArchived ? "Unarchive" : "Archive"}
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
        <FormField label="Note (optional)" hint="For example: “Returned in cash.”">
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
