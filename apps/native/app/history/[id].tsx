import { useLocalSearchParams } from "expo-router";
import { Surface } from "heroui-native";
import { useMemo } from "react";
import { Text, View } from "react-native";

import {
  daysOverdue,
  formatDateOnly,
  formatMoney,
  formatPeriod,
  isOverdue,
} from "@my-better-t-app/api/treasurer/money";
import { PaymentRow, ShareRow } from "@/src/features/treasurer/components/rows";
import {
  BillStatusBadge,
  EmptyState,
  ErrorBlock,
  Screen,
  SectionHeader,
  StatusBadge,
  SummaryCard,
} from "@/src/features/treasurer/components/ui";
import {
  DetailHeaderSkeleton,
  ShareRowsSkeleton,
  SummaryCardsSkeleton,
} from "@/src/features/treasurer/components/skeleton";
import { useAppSettings, useBillDetail } from "@/src/features/treasurer/queries";

export default function HistoryBillScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const detail = useBillDetail(id);
  const settings = useAppSettings();

  const currency = settings.data?.currencyCode ?? "PHP";
  const data = detail.data;
  const bill = data?.bill;

  const paymentRows = useMemo(() => {
    if (!data) return [];
    const names = new Map(
      data.shares.map((share) => [share.id, share.boarderNameSnapshot]),
    );
    return data.shares
      .flatMap((share) => share.state.payments)
      .sort((a, b) => b.paidAt - a.paidAt)
      .map((payment) => ({
        payment,
        boarderName: names.get(payment.billShareId) ?? "Boarder",
      }));
  }, [data]);

  if (detail.isLoading) {
    return (
      <Screen>
        <DetailHeaderSkeleton />
        <View className="mt-4">
          <SummaryCardsSkeleton />
        </View>
        <SectionHeader
          title="Tenants"
          subtitle="Snapshot names from the time the bill was created."
        />
        <ShareRowsSkeleton />
      </Screen>
    );
  }

  if (detail.isError || !data || !bill) {
    return (
      <Screen>
        <ErrorBlock
          message="This bill could not be loaded."
          onRetry={() => void detail.refetch()}
        />
      </Screen>
    );
  }

  const overdue = isOverdue(bill.dueDate, data.summary.outstandingMinor > 0);
  const renamed =
    data.currentTypeName.localeCompare(bill.billTypeNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;
  const isArchived = bill.archivedAt !== null;

  return (
    <Screen>
      <View className="pt-4">
        <Text className="text-xs font-medium uppercase tracking-wide text-muted">
          {formatPeriod(bill.periodYear, bill.periodMonth)}
        </Text>
        <View className="mt-1 flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Text className="text-2xl font-semibold text-foreground">
              {bill.billTypeNameSnapshot}
            </Text>
            {renamed ? (
              <Text className="mt-0.5 text-xs text-muted">
                Now called {data.currentTypeName}
              </Text>
            ) : null}
          </View>
          <View className="items-end gap-1">
            <BillStatusBadge status={data.summary.status} />
            <StatusBadge label="Read-only" tone="default" icon="lock-closed-outline" />
          </View>
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
          {isArchived ? (
            <StatusBadge label="Archived" tone="default" icon="archive-outline" />
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
              label="Change to return"
              value={formatMoney(data.summary.changeOutstandingMinor, currency)}
              tone={data.summary.changeOutstandingMinor > 0 ? "warning" : "default"}
              icon="cash-outline"
            />
          </View>
        </View>
      </View>

      <SectionHeader
        title={`Boarders (${data.shares.length})`}
        subtitle="Snapshot names from the time the bill was created."
      />

      {data.shares.length === 0 ? (
        <EmptyState
          icon="people-outline"
          title="No boarders on this bill"
          description="This bill was created without any shares."
        />
      ) : (
        data.shares.map((share) => (
          <ShareRow
            key={share.id}
            share={share}
            currency={currency}
            readOnly
            onRecordPayment={() => {}}
          />
        ))
      )}

      <SectionHeader
        title="Payment history"
        subtitle="Newest first. Voided entries stay visible."
      />

      <Surface variant="secondary" className="rounded-2xl px-4">
        {paymentRows.length === 0 ? (
          <View className="py-5">
            <Text className="text-center text-sm text-muted">No payments recorded.</Text>
          </View>
        ) : (
          paymentRows.map(({ payment, boarderName }) => (
            <PaymentRow
              key={payment.id}
              payment={payment}
              currency={currency}
              boarderName={boarderName}
              readOnly
              onMarkChangeReturned={() => {}}
              onVoid={() => {}}
            />
          ))
        )}
      </Surface>
    </Screen>
  );
}
