import { Ionicons } from "@expo/vector-icons";
import { Button, Chip, Surface, useThemeColor } from "heroui-native";
import { Pressable, Text, View } from "react-native";

import { formatDateOnly, formatDateTime, formatMoney, formatPeriod, isOverdue } from "../money";
import type { PaymentWithComputedChange, ShareWithState } from "../types";
import type { BillWithDetails } from "../repository";
import {
  BillStatusBadge,
  ChangeBadge,
  MoneyText,
  ShareStatusBadge,
  StatusBadge,
} from "./ui";

export function BillCard({
  item,
  currency,
  onPress,
}: {
  item: BillWithDetails;
  currency: string;
  onPress: () => void;
}) {
  const muted = useThemeColor("muted");
  const overdue = isOverdue(item.bill.dueDate, item.summary.outstandingMinor > 0);
  const renamed =
    item.currentTypeName.localeCompare(item.bill.billTypeNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${item.bill.billTypeNameSnapshot}, ${formatPeriod(item.bill.periodYear, item.bill.periodMonth)}, ${formatMoney(item.bill.totalMinor, currency)}, ${item.summary.status}`}
      onPress={onPress}
      className="mb-3"
    >
      <Surface variant="secondary" className="rounded-2xl p-4">
        <View className="flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Text className="text-base font-semibold text-foreground">
              {item.bill.billTypeNameSnapshot}
            </Text>
            {renamed ? (
              <Text className="mt-0.5 text-xs text-muted">Now called {item.currentTypeName}</Text>
            ) : null}
            <Text className="mt-0.5 text-xs text-muted">
              {formatPeriod(item.bill.periodYear, item.bill.periodMonth)}
            </Text>
          </View>
          <View className="items-end gap-1.5">
            <BillStatusBadge status={item.summary.status} />
            <MoneyText minor={item.bill.totalMinor} currency={currency} size="md" />
          </View>
        </View>

        <View className="mt-3 flex-row flex-wrap items-center gap-2">
          <Text className="text-xs text-muted">
            {formatMoney(item.summary.appliedMinor, currency)} of{" "}
            {formatMoney(item.summary.expectedMinor, currency)} paid
          </Text>
          <Text className="text-xs text-muted">
            • {item.summary.paidShareCount} of {item.summary.totalShareCount} paid
          </Text>
          {item.summary.changeOutstandingMinor > 0 ? (
            <ChangeBadge minor={item.summary.changeOutstandingMinor} currency={currency} />
          ) : null}
          {overdue ? <StatusBadge label="Overdue" tone="danger" icon="calendar-outline" /> : null}
        </View>

        <View className="mt-2 flex-row items-center gap-2">
          {item.bill.dueDate ? (
            <Text className="text-xs text-muted">Due {formatDateOnly(item.bill.dueDate)}</Text>
          ) : null}
          {item.summary.needsReview ? (
            <Text className="text-xs font-medium text-warning">Review a corrected change</Text>
          ) : null}
        </View>

        <View className="mt-3 h-2 overflow-hidden rounded-full bg-default">
          <View
            className="h-2 rounded-full bg-success"
            style={{ width: `${Math.max(2, Math.round(item.summary.progress * 100))}%` }}
          />
        </View>

        <View className="mt-3 flex-row items-center justify-end gap-1">
          <Text className="text-xs font-medium text-accent">View bill</Text>
          <Ionicons name="chevron-forward" size={14} color={muted} />
        </View>
      </Surface>
    </Pressable>
  );
}

export function ShareRow({
  share,
  currency,
  onRecordPayment,
  readOnly = false,
}: {
  share: ShareWithState;
  currency: string;
  onRecordPayment: () => void;
  readOnly?: boolean;
}) {
  const nameChanged =
    share.currentBoarderName &&
    share.currentBoarderName.localeCompare(share.boarderNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;

  return (
    <Surface variant="secondary" className="mb-2 rounded-2xl p-3.5">
      <View className="flex-row items-start justify-between gap-3">
        <View className="flex-1">
          <View className="flex-row flex-wrap items-center gap-1.5">
            <Text className="text-sm font-semibold text-foreground">{share.boarderNameSnapshot}</Text>
            {share.boarderArchived ? (
              <Chip size="sm" variant="secondary">
                <Chip.Label>Archived</Chip.Label>
              </Chip>
            ) : null}
          </View>
          {nameChanged ? (
            <Text className="mt-0.5 text-xs text-muted">Currently: {share.currentBoarderName}</Text>
          ) : null}
          <View className="mt-2 flex-row flex-wrap items-center gap-x-4 gap-y-1">
            <Text className="text-xs text-muted">
              Due {formatMoney(share.amountDueMinor, currency)}
            </Text>
            <Text className="text-xs text-muted">
              Paid {formatMoney(share.state.paidMinor, currency)}
            </Text>
            {share.state.balanceMinor > 0 ? (
              <Text className="text-xs font-medium text-danger">
                {formatMoney(share.state.balanceMinor, currency)} left
              </Text>
            ) : share.state.changeOutstandingMinor === 0 ? (
              <Text className="text-xs font-medium text-success">Paid in full</Text>
            ) : null}
          </View>
        </View>
        <View className="items-end gap-1.5">
          <ShareStatusBadge status={share.state.status} />
          {share.state.changeOutstandingMinor > 0 ? (
            <ChangeBadge minor={share.state.changeOutstandingMinor} currency={currency} />
          ) : null}
        </View>
      </View>

      <View className="mt-3 flex-row items-center justify-between gap-2">
        <Text className="text-xs text-muted">
          {share.state.status === "paid"
            ? "Covered"
            : share.state.status === "partial"
              ? "Partially paid"
              : "No payments yet"}
        </Text>
        {!readOnly ? (
          <Button
            size="sm"
            variant={share.state.balanceMinor > 0 ? "primary" : "secondary"}
            onPress={onRecordPayment}
          >
            <Button.Label>
              {share.state.balanceMinor > 0 ? "Record payment" : "Add payment"}
            </Button.Label>
          </Button>
        ) : null}
      </View>
    </Surface>
  );
}

export function PaymentRow({
  payment,
  currency,
  boarderName,
  onMarkChangeReturned,
  onVoid,
  readOnly = false,
}: {
  payment: PaymentWithComputedChange;
  currency: string;
  boarderName: string;
  onMarkChangeReturned: () => void;
  onVoid: () => void;
  readOnly?: boolean;
}) {
  const muted = useThemeColor("muted");
  const voided = payment.voidedAt !== null;
  const changeOutstanding =
    !voided && payment.computedChangeMinor > 0 && payment.changeReturnedAt === null;
  const changeReturned = payment.changeReturnedAt !== null;

  return (
    <View className="border-b border-separator py-3 last:border-b-0">
      <View className="flex-row items-start justify-between gap-3">
        <View className="flex-1">
          <View className="flex-row flex-wrap items-center gap-2">
            <Text
              className={
                voided
                  ? "text-sm font-medium text-muted line-through"
                  : "text-sm font-semibold text-foreground"
              }
            >
              {boarderName}
            </Text>
            {voided ? (
              <StatusBadge label="Voided" tone="danger" icon="close-circle-outline" />
            ) : null}
            {changeOutstanding ? (
              <StatusBadge label="Change due" tone="warning" icon="cash-outline" />
            ) : null}
            {changeReturned ? (
              <StatusBadge label="Change returned" tone="success" icon="return-down-back-outline" />
            ) : null}
          </View>
          <Text className="mt-1 text-xs text-muted">
            {formatDateTime(payment.paidAt)}
            {payment.note ? ` • ${payment.note}` : ""}
          </Text>
          {voided ? (
            <Text className="mt-1 text-xs text-danger">
              Voided {payment.voidedAt ? formatDateTime(payment.voidedAt) : ""} — {payment.voidReason}
            </Text>
          ) : null}
          {payment.computedChangeMinor > 0 ? (
            <Text className="mt-1 text-xs text-muted">
              {formatMoney(payment.computedChangeMinor, currency)} change
              {changeReturned && payment.changeReturnedAt
                ? ` • returned ${formatDateTime(payment.changeReturnedAt)}`
                : ""}
              {payment.changeReturnNote ? ` • ${payment.changeReturnNote}` : ""}
            </Text>
          ) : null}
          {payment.needsReview ? (
            <Text className="mt-1 text-xs font-medium text-warning">
              Change no longer matches after a correction — review this entry.
            </Text>
          ) : null}
        </View>
        <MoneyText
          minor={payment.amountMinor}
          currency={currency}
          size="sm"
          className={voided ? "text-muted line-through" : undefined}
        />
      </View>

      {!voided && !readOnly ? (
        <View className="mt-2 flex-row justify-end gap-2">
          {changeOutstanding ? (
            <Button size="sm" variant="secondary" onPress={onMarkChangeReturned}>
              <Ionicons name="cash-outline" size={14} color={muted} />
              <Button.Label>Mark change returned</Button.Label>
            </Button>
          ) : null}
          <Button size="sm" variant="ghost" onPress={onVoid}>
            <Button.Label>Void</Button.Label>
          </Button>
        </View>
      ) : null}
    </View>
  );
}

export function BoarderBalanceRow({
  name,
  archived,
  expectedMinor,
  appliedMinor,
  outstandingMinor,
  changeOutstandingMinor,
  currency,
  onPress,
}: {
  name: string;
  archived: boolean;
  expectedMinor: number;
  appliedMinor: number;
  outstandingMinor: number;
  changeOutstandingMinor: number;
  currency: string;
  onPress: () => void;
}) {
  const status = outstandingMinor === 0 ? "paid" : appliedMinor > 0 ? "partial" : "unpaid";
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${name}, ${formatMoney(outstandingMinor, currency)} outstanding`}
      onPress={onPress}
      className="mb-2"
    >
      <Surface variant="secondary" className="rounded-2xl p-3.5">
        <View className="flex-row items-center justify-between gap-3">
          <View className="flex-1">
            <View className="flex-row flex-wrap items-center gap-1.5">
              <Text className="text-sm font-semibold text-foreground">{name}</Text>
              {archived ? (
                <Chip size="sm" variant="secondary">
                  <Chip.Label>Archived</Chip.Label>
                </Chip>
              ) : null}
            </View>
            <Text className="mt-1 text-xs text-muted">
              {formatMoney(appliedMinor, currency)} of {formatMoney(expectedMinor, currency)} paid
              {changeOutstandingMinor > 0
                ? ` • ${formatMoney(changeOutstandingMinor, currency)} change`
                : ""}
            </Text>
          </View>
          <View className="items-end gap-1">
            <ShareStatusBadge status={status} />
            <Text className="text-xs text-muted">
              {outstandingMinor > 0 ? `${formatMoney(outstandingMinor, currency)} left` : "Settled"}
            </Text>
          </View>
        </View>
      </Surface>
    </Pressable>
  );
}
