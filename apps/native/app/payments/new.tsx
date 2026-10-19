import DateTimePicker, {
  type DateTimePickerEvent,
} from "@react-native-community/datetimepicker";
import { Ionicons } from "@expo/vector-icons";
import { router, useLocalSearchParams } from "expo-router";
import { Button, Surface, useThemeColor } from "heroui-native";
import { useEffect, useRef, useState } from "react";
import { Pressable, Text, View } from "react-native";

import { previewPayment } from "@/src/features/treasurer/calculations";
import {
  AppInput,
  EmptyState,
  ErrorBlock,
  FormField,
  LoadingBlock,
  Screen,
  ShareStatusBadge,
} from "@/src/features/treasurer/components/ui";
import {
  formatDate,
  formatDateTime,
  formatMoney,
  formatPeriod,
  minorToInputValue,
  parseMoneyToMinor,
} from "@/src/features/treasurer/money";
import {
  useAppSettings,
  usePaymentContext,
  useRecordPayment,
} from "@/src/features/treasurer/queries";

export default function PaymentScreen() {
  const { shareId } = useLocalSearchParams<{ shareId?: string | string[] }>();
  const id = Array.isArray(shareId) ? shareId[0] : shareId;
  const context = usePaymentContext(id);
  const settings = useAppSettings();
  const recordPayment = useRecordPayment();

  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");
  const success = useThemeColor("success");

  const [amountInput, setAmountInput] = useState("");
  const [paidAt, setPaidAt] = useState(Date.now());
  const [note, setNote] = useState("");
  const [datePickerOpen, setDatePickerOpen] = useState(false);
  const [timePickerOpen, setTimePickerOpen] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [saved, setSaved] = useState<{
    amountMinor: number;
    changeMinor: number;
    balanceMinor: number;
  } | null>(null);
  const initialized = useRef(false);

  const currency = settings.data?.currencyCode ?? "PHP";

  useEffect(() => {
    if (initialized.current || !context.data) return;
    const balance = context.data.share.state.balanceMinor;
    if (balance > 0) setAmountInput(minorToInputValue(balance));
    initialized.current = true;
  }, [context.data]);

  if (!id) {
    return (
      <Screen>
        <EmptyState
          icon="receipt-outline"
          title="Choose a boarder first"
          description="Open a bill or boarder and tap Record payment."
          actionLabel="Go back"
          onAction={() => router.back()}
        />
      </Screen>
    );
  }

  if (context.isLoading) {
    return (
      <Screen>
        <LoadingBlock label="Loading payment details…" />
      </Screen>
    );
  }

  if (context.isError || !context.data) {
    return (
      <Screen>
        <ErrorBlock message="This share could not be loaded." onRetry={() => void context.refetch()} />
      </Screen>
    );
  }

  const { share, bill, boarderName } = context.data;
  const amountMinor = parseMoneyToMinor(amountInput);
  const preview =
    amountMinor !== null && amountMinor > 0
      ? previewPayment(share.amountDueMinor, share.state.payments, amountMinor, paidAt)
      : null;

  async function handleSave() {
    if (amountMinor === null || amountMinor <= 0) {
      setFormError("Enter a payment amount greater than zero.");
      return;
    }
    if (note.length > 500) {
      setFormError("Keep the note to 500 characters or fewer.");
      return;
    }

    setFormError(null);
    try {
      await recordPayment.mutateAsync({
        billShareId: share.id,
        amountMinor,
        paidAt,
        note: note.trim() || null,
      });
      setSaved({
        amountMinor,
        changeMinor: preview?.changeFromThisPaymentMinor ?? 0,
        balanceMinor: preview?.balanceAfterMinor ?? 0,
      });
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "The payment could not be saved.");
    }
  }

  if (saved) {
    return (
      <Screen>
        <View className="flex-1 items-center justify-center py-12">
          <View className="h-16 w-16 items-center justify-center rounded-full bg-success/15">
            <Ionicons name="checkmark" size={32} color={success} />
          </View>
          <Text className="mt-4 text-2xl font-semibold text-foreground">Payment recorded</Text>
          <Text className="mt-1 text-sm text-muted">
            {boarderName} · {bill.billTypeNameSnapshot} ·{" "}
            {formatPeriod(bill.periodYear, bill.periodMonth)}
          </Text>
          <Text
            className="mt-4 text-2xl font-semibold text-foreground"
            style={{ fontVariant: ["tabular-nums"] }}
          >
            {formatMoney(saved.amountMinor, currency)}
          </Text>
          {saved.balanceMinor > 0 ? (
            <Text className="mt-2 text-sm font-medium text-danger">
              {formatMoney(saved.balanceMinor, currency)} still owed
            </Text>
          ) : saved.changeMinor > 0 ? (
            <Text className="mt-2 text-sm font-medium text-warning">
              Paid · {formatMoney(saved.changeMinor, currency)} change to return
            </Text>
          ) : (
            <Text className="mt-2 text-sm font-medium text-success">Paid in full</Text>
          )}
          <Text className="mt-2 text-xs text-muted">{formatDateTime(paidAt)}</Text>
          <Button className="mt-6 w-full max-w-xs" onPress={() => router.back()}>
            <Button.Label>Done</Button.Label>
          </Button>
        </View>
      </Screen>
    );
  }

  const alreadyPaid = share.state.balanceMinor === 0;

  return (
    <Screen>
      <View className="pt-4">
        <Text className="text-2xl font-semibold text-foreground">{boarderName}</Text>
        <Text className="mt-1 text-sm text-muted">
          {bill.billTypeNameSnapshot} · {formatPeriod(bill.periodYear, bill.periodMonth)}
        </Text>
      </View>

      <Surface variant="secondary" className="mt-4 rounded-2xl p-4">
        <View className="flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Text className="text-xs uppercase tracking-wide text-muted">Amount due</Text>
            <Text
              className="mt-1 text-2xl font-semibold text-foreground"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {formatMoney(share.amountDueMinor, currency)}
            </Text>
          </View>
          <ShareStatusBadge status={share.state.status} />
        </View>
        <View className="mt-3 flex-row flex-wrap gap-x-5 gap-y-1">
          <Text className="text-xs text-muted">
            Paid so far {formatMoney(share.state.paidMinor, currency)}
          </Text>
          <Text className="text-xs text-muted">
            Balance {formatMoney(share.state.balanceMinor, currency)}
          </Text>
        </View>
        {share.state.changeOutstandingMinor > 0 ? (
          <Text className="mt-2 text-xs font-medium text-warning">
            {formatMoney(share.state.changeOutstandingMinor, currency)} change is still outstanding
            from an earlier payment. It will not be netted automatically.
          </Text>
        ) : null}
      </Surface>

      {formError ? (
        <View className="mt-4">
          <ErrorBlock message={formError} />
        </View>
      ) : null}

      <View className="mt-5">
        <FormField
          label={`Amount received (${currency})`}
          hint={
            alreadyPaid
              ? "This share is already paid. Any new amount will create change to return."
              : "Prefilled with the remaining balance. Clear it to enter a partial payment."
          }
          required
        >
          <AppInput
            autoFocus={!alreadyPaid}
            value={amountInput}
            onChangeText={(value) => {
              setAmountInput(value);
              if (formError) setFormError(null);
            }}
            keyboardType="decimal-pad"
            placeholder="0.00"
            maxLength={16}
          />
        </FormField>

        <FormField label="Payment date and time" required>
          <View className="flex-row gap-2">
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`Payment date ${formatDate(paidAt)}. Change date.`}
              onPress={() => setDatePickerOpen(true)}
              className="min-h-12 flex-1 flex-row items-center justify-between rounded-xl border border-separator px-3"
            >
              <Text className="text-sm text-foreground">{formatDate(paidAt)}</Text>
              <Ionicons name="calendar-outline" size={17} color={muted} />
            </Pressable>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`Payment time ${formatDateTime(paidAt)}. Change time.`}
              onPress={() => setTimePickerOpen(true)}
              className="min-h-12 flex-1 flex-row items-center justify-between rounded-xl border border-separator px-3"
            >
              <Text className="text-sm text-foreground">
                {new Intl.DateTimeFormat("en-PH", {
                  hour: "numeric",
                  minute: "2-digit",
                }).format(new Date(paidAt))}
              </Text>
              <Ionicons name="time-outline" size={17} color={muted} />
            </Pressable>
          </View>
        </FormField>

        <FormField label="Note (optional)" hint="Up to 500 characters.">
          <AppInput
            value={note}
            onChangeText={setNote}
            maxLength={500}
            multiline
            numberOfLines={3}
            className="min-h-20"
            textAlignVertical="top"
            placeholder="e.g. Cash handed over at the gate"
          />
        </FormField>
      </View>

      <Surface variant="secondary" className="mt-2 rounded-2xl p-4">
        <Text className="text-xs font-semibold uppercase tracking-wide text-muted">After saving</Text>
        {preview ? (
          <>
            <Text
              className={
                preview.balanceAfterMinor > 0
                  ? "mt-2 text-lg font-semibold text-danger"
                  : preview.changeFromThisPaymentMinor > 0
                    ? "mt-2 text-lg font-semibold text-warning"
                    : "mt-2 text-lg font-semibold text-success"
              }
            >
              {preview.balanceAfterMinor > 0
                ? `${formatMoney(preview.balanceAfterMinor, currency)} still owed`
                : preview.changeFromThisPaymentMinor > 0
                  ? `Paid · ${formatMoney(preview.changeFromThisPaymentMinor, currency)} change to return`
                  : "Paid in full"}
            </Text>
            <Text className="mt-1 text-xs text-muted">
              Total paid after this entry: {formatMoney(preview.paidAfterMinor, currency)}
              {preview.changeOutstandingAfterMinor > 0
                ? ` · Change outstanding: ${formatMoney(preview.changeOutstandingAfterMinor, currency)}`
                : ""}
            </Text>
          </>
        ) : (
          <Text className="mt-2 text-sm text-muted">
            Enter an amount to see the remaining balance or change.
          </Text>
        )}
      </Surface>

      <Button
        className="mt-5"
        size="lg"
        isDisabled={amountMinor === null || amountMinor <= 0 || recordPayment.isPending}
        onPress={() => void handleSave()}
      >
        <Button.Label>{recordPayment.isPending ? "Saving…" : "Save payment"}</Button.Label>
      </Button>
      <Button variant="ghost" className="mt-2" onPress={() => router.back()}>
        <Button.Label>Cancel</Button.Label>
      </Button>

      {datePickerOpen ? (
        <DateTimePicker
          value={new Date(paidAt)}
          mode="date"
          onChange={(event: DateTimePickerEvent, date?: Date) => {
            setDatePickerOpen(false);
            if (event.type === "dismissed" || !date) return;
            const next = new Date(paidAt);
            next.setFullYear(date.getFullYear(), date.getMonth(), date.getDate());
            setPaidAt(next.getTime());
          }}
        />
      ) : null}

      {timePickerOpen ? (
        <DateTimePicker
          value={new Date(paidAt)}
          mode="time"
          onChange={(event: DateTimePickerEvent, date?: Date) => {
            setTimePickerOpen(false);
            if (event.type === "dismissed" || !date) return;
            const next = new Date(paidAt);
            next.setHours(date.getHours(), date.getMinutes(), 0, 0);
            setPaidAt(next.getTime());
          }}
        />
      ) : null}
    </Screen>
  );
}
