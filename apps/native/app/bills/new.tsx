import DateTimePicker, {
  type DateTimePickerEvent,
} from "@react-native-community/datetimepicker";
import { Ionicons } from "@expo/vector-icons";
import type { BillRow } from "@my-better-t-app/db/schema";
import { router, useLocalSearchParams } from "expo-router";
import { Button, Surface, cn, useThemeColor } from "heroui-native";
import { useEffect, useMemo, useRef, useState } from "react";
import { Pressable, ScrollView, Text, View } from "react-native";

import { equalSplit, splitWithOverrides } from "@my-better-t-app/api/treasurer/calculations";
import {
  AppDialog,
  AppInput,
  EmptyState,
  ErrorBlock,
  FormField,
  MonthSwitcher,
  Screen,
  SectionHeader,
} from "@/src/features/treasurer/components/ui";
import {
  ChipRowSkeleton,
  FormSkeleton,
  TitleSkeleton,
} from "@/src/features/treasurer/components/skeleton";
import {
  currentPeriod,
  formatDateOnly,
  formatMoney,
  formatPeriod,
  isoDateToDate,
  minorToInputValue,
  parseAmountInput,
  parseMoneyToMinor,
  toIsoDate,
} from "@my-better-t-app/api/treasurer/money";
import {
  checkDuplicateBills,
  useActiveBoarders,
  useAppSettings,
  useBillDetail,
  useBillTypes,
  useCreateBill,
  useUpdateBill,
  useUpdateBillMeta,
} from "@/src/features/treasurer/queries";
import type { SplitResult } from "@my-better-t-app/api/treasurer/types";

type SplitMethod = "equal" | "custom";

const WHOLE_PESO_MESSAGE = "Whole pesos only. Turn on centavos in Settings to use decimals.";

interface PickerBoarder {
  id: string;
  name: string;
  status: "active" | "archived";
  createdAt: number;
}

function firstParam(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

export default function BillFormScreen() {
  const params = useLocalSearchParams<{
    id?: string | string[];
    year?: string | string[];
    month?: string | string[];
  }>();
  const editId = firstParam(params.id);
  const detail = useBillDetail(editId);
  const types = useBillTypes();
  const activeBoarders = useActiveBoarders();
  const createBill = useCreateBill();
  const updateBill = useUpdateBill();
  const updateMeta = useUpdateBillMeta();
  const settings = useAppSettings();

  const currency = settings.data?.currencyCode ?? "PHP";
  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");

  const [billTypeId, setBillTypeId] = useState("");
  const [period, setPeriod] = useState(() => {
    const initial = currentPeriod();
    const year = Number(firstParam(params.year));
    const month = Number(firstParam(params.month));
    return {
      year: Number.isFinite(year) && year >= 2000 ? year : initial.year,
      month: Number.isFinite(month) && month >= 1 && month <= 12 ? month : initial.month,
    };
  });
  const [totalInput, setTotalInput] = useState("");
  const [dueDate, setDueDate] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [method, setMethod] = useState<SplitMethod>("equal");
  const [customInputs, setCustomInputs] = useState<Record<string, string>>({});
  const [pinnedIds, setPinnedIds] = useState<string[]>([]);
  const [datePickerOpen, setDatePickerOpen] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [duplicates, setDuplicates] = useState<BillRow[] | null>(null);
  const [duplicateOpen, setDuplicateOpen] = useState(false);
  const [metaNotice, setMetaNotice] = useState<string | null>(null);
  const initialized = useRef(false);

  const bill = detail.data?.bill;
  const locked = Boolean(editId && detail.data?.isLocked);
  const archived = bill?.archivedAt !== null && bill?.archivedAt !== undefined;

  const pickerBoarders = useMemo(() => {
    const map = new Map<string, PickerBoarder>();

    for (const boarder of activeBoarders.data ?? []) {
      map.set(boarder.id, {
        id: boarder.id,
        name: boarder.name,
        status: "active",
        createdAt: boarder.createdAt,
      });
    }

    for (const share of detail.data?.shares ?? []) {
      if (!map.has(share.boarderId)) {
        map.set(share.boarderId, {
          id: share.boarderId,
          name: share.currentBoarderName ?? share.boarderNameSnapshot,
          status: share.boarderArchived ? "archived" : "active",
          createdAt: share.createdAt,
        });
      }
    }

    return [...map.values()].sort((a, b) =>
      a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
    );
  }, [activeBoarders.data, detail.data]);

  useEffect(() => {
    if (initialized.current) return;

    if (editId) {
      if (!detail.data) return;
      const currentBill = detail.data.bill;
      setBillTypeId(currentBill.billTypeId);
      setPeriod({ year: currentBill.periodYear, month: currentBill.periodMonth });
      setTotalInput(minorToInputValue(currentBill.totalMinor));
      setDueDate(currentBill.dueDate);
      setNote(currentBill.note ?? "");
      setSelectedIds(detail.data.shares.map((share) => share.boarderId));

      const savedAmounts = detail.data.shares.map((share) => share.amountDueMinor);
      const evenlySplit = savedAmounts.every((amount) => amount === savedAmounts[0]);

      if (evenlySplit) {
        setMethod("equal");
      } else {
        // The split method is not stored, so an uneven bill opens with every row
        // pinned to its saved amount and can be re-saved unchanged.
        setMethod("custom");
        setPinnedIds(detail.data.shares.map((share) => share.boarderId));
        setCustomInputs(
          Object.fromEntries(
            detail.data.shares.map((share) => [
              share.boarderId,
              minorToInputValue(share.amountDueMinor),
            ]),
          ),
        );
      }

      initialized.current = true;
      return;
    }

    if (!activeBoarders.isLoading) {
      setSelectedIds((activeBoarders.data ?? []).map((boarder) => boarder.id));
      initialized.current = true;
    }
  }, [editId, detail.data, activeBoarders.data, activeBoarders.isLoading]);

  const allowCentavos = settings.data?.allowCentavos ?? true;

  const parsedTotal = parseAmountInput(totalInput, { allowCentavos });
  const selectedBoarders = selectedIds
    .map((id) => pickerBoarders.find((boarder) => boarder.id === id))
    .filter((boarder): boarder is PickerBoarder => Boolean(boarder));

  const equalResult = useMemo(() => {
    if (parsedTotal === null) return null;
    return equalSplit(
      parsedTotal,
      selectedBoarders.map((boarder) => ({
        boarderId: boarder.id,
        name: boarder.name,
        createdAt: boarder.createdAt,
      })),
      { allowCentavos },
    );
  }, [parsedTotal, selectedBoarders, allowCentavos]);

  const pinnedBoarders = selectedBoarders.filter((boarder) => pinnedIds.includes(boarder.id));

  const overrides = new Map<string, number>();
  let invalidPin: { name: string; wholePesosOnly: boolean } | null = null;

  for (const boarder of pinnedBoarders) {
    const raw = customInputs[boarder.id] ?? "";
    const amount = parseAmountInput(raw, { allowCentavos });

    if (amount === null) {
      invalidPin = { name: boarder.name, wholePesosOnly: parseMoneyToMinor(raw) !== null };
      break;
    }

    overrides.set(boarder.id, amount);
  }

  const customResult =
    invalidPin || parsedTotal === null
      ? null
      : pinnedBoarders.length === 0
        ? equalResult
        : splitWithOverrides(
            parsedTotal,
            selectedBoarders.map((boarder) => ({
              boarderId: boarder.id,
              name: boarder.name,
              createdAt: boarder.createdAt,
            })),
            overrides,
            { allowCentavos },
          );

  const sharesForSave: SplitResult[] | null =
    method === "equal"
      ? equalResult && !("error" in equalResult)
        ? equalResult.shares
        : null
      : customResult && !("error" in customResult)
        ? customResult.shares
        : null;

  const splitError =
    parsedTotal === null
      ? "Enter a valid total greater than zero."
      : selectedIds.length === 0
        ? "Select at least one boarder."
        : method === "equal"
          ? equalResult && "error" in equalResult
            ? equalResult.error
            : null
          : invalidPin
            ? invalidPin.wholePesosOnly
              ? WHOLE_PESO_MESSAGE
              : `Enter a valid amount for ${invalidPin.name}.`
            : customResult && "error" in customResult
              ? customResult.overByMinor !== undefined
                ? `Custom amounts are ${formatMoney(customResult.overByMinor, currency)} over the bill total.`
                : customResult.error
              : null;

  const customAssignedMinor = [...overrides.values()].reduce((total, value) => total + value, 0);
  const leftoverMinor = parsedTotal === null ? 0 : parsedTotal - customAssignedMinor;
  const poolCount = selectedBoarders.length - pinnedBoarders.length;

  function toggleBoarder(id: string) {
    setSelectedIds((current) => {
      if (current.includes(id)) {
        return current.filter((value) => value !== id);
      }
      return [...current, id];
    });
  }

  function togglePinned(id: string) {
    if (pinnedIds.includes(id)) {
      setPinnedIds((current) => current.filter((value) => value !== id));
      return;
    }

    const preview =
      customResult && !("error" in customResult)
        ? customResult.shares.find((share) => share.boarderId === id)
        : equalResult && !("error" in equalResult)
          ? equalResult.shares.find((share) => share.boarderId === id)
          : undefined;

    setPinnedIds((current) => [...current, id]);
    setCustomInputs((current) =>
      current[id] || !preview ? current : { ...current, [id]: minorToInputValue(preview.amountMinor) },
    );
  }

  function chooseMethod(next: SplitMethod) {
    // Custom mode starts neutral: nothing is pinned until the treasurer says so.
    setMethod(next);
  }

  async function performSave(allowDuplicate: boolean) {
    if (!billTypeId) {
      setFormError("Choose a bill type.");
      return;
    }
    if (parsedTotal === null || parsedTotal <= 0) {
      setFormError("Enter a valid bill total greater than zero.");
      return;
    }
    if (!sharesForSave) {
      setFormError(splitError ?? "Complete the split before saving.");
      return;
    }
    if (note.length > 500) {
      setFormError("Keep the note to 500 characters or fewer.");
      return;
    }

    setFormError(null);

    if (!allowDuplicate) {
      const matches = await checkDuplicateBills(
        billTypeId,
        period.year,
        period.month,
        editId,
      );
      if (matches.length > 0) {
        setDuplicates(matches);
        setDuplicateOpen(true);
        return;
      }
    }

    const input = {
      billTypeId,
      periodYear: period.year,
      periodMonth: period.month,
      totalMinor: parsedTotal,
      dueDate,
      note: note.trim() || null,
      shares: sharesForSave,
    };

    try {
      if (editId) {
        await updateBill.mutateAsync({ id: editId, input });
        router.back();
      } else {
        const id = await createBill.mutateAsync(input);
        router.replace({ pathname: "/bills/[id]", params: { id } });
      }
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "The bill could not be saved.");
    }
  }

  async function saveMeta() {
    if (!editId) return;
    setMetaNotice(null);
    setFormError(null);
    try {
      await updateMeta.mutateAsync({
        id: editId,
        dueDate,
        note: note.trim() || null,
      });
      setMetaNotice("Due date and note updated.");
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "The bill could not be updated.");
    }
  }

  if (editId && detail.isLoading) {
    return (
      <Screen>
        <TitleSkeleton />
        <FormSkeleton fields={3} />
      </Screen>
    );
  }

  if (editId && (detail.isError || !detail.data)) {
    return (
      <Screen>
        <ErrorBlock message="This bill could not be loaded." onRetry={() => void detail.refetch()} />
      </Screen>
    );
  }

  if (archived) {
    return (
      <Screen>
        <EmptyState
          icon="archive-outline"
          title="This bill is archived"
          description="Unarchive it from the bill detail screen before making changes."
          actionLabel="Back to bill"
          onAction={() => router.back()}
        />
      </Screen>
    );
  }

  if (locked && detail.data) {
    return (
      <Screen>
        <View className="pt-4">
          <Text className="text-2xl font-semibold text-foreground">Edit bill details</Text>
          <Text className="mt-1 text-sm text-muted">
            Payments have been recorded, so the type, period, total, and shares are locked.
            You can still update the due date and note.
          </Text>
        </View>

        <Surface variant="secondary" className="mt-4 rounded-2xl p-4">
          <Text className="text-sm font-semibold text-foreground">
            {detail.data.bill.billTypeNameSnapshot}
          </Text>
          <Text className="mt-1 text-xs text-muted">
            {formatPeriod(detail.data.bill.periodYear, detail.data.bill.periodMonth)} ·{" "}
            {formatMoney(detail.data.bill.totalMinor, currency)}
          </Text>
          <Text className="mt-2 text-xs text-muted">
            {detail.data.shares.length} boarders · {detail.data.summary.paidShareCount} paid
          </Text>
        </Surface>

        {metaNotice ? (
          <View className="mt-4 rounded-xl border border-success/30 bg-success/10 p-3">
            <Text className="text-sm text-success">{metaNotice}</Text>
          </View>
        ) : null}
        {formError ? (
          <View className="mt-4">
            <ErrorBlock message={formError} />
          </View>
        ) : null}

        <View className="mt-5">
          <FormField label="Due date (optional)">
            <Pressable
              accessibilityRole="button"
              className="min-h-12 flex-row items-center justify-between rounded-xl border border-separator bg-surface px-3"
              onPress={() => setDatePickerOpen(true)}
            >
              <Text className={dueDate ? "text-sm text-foreground" : "text-sm text-muted"}>
                {dueDate ? formatDateOnly(dueDate) : "No due date"}
              </Text>
              <Ionicons name="calendar-outline" size={18} color={muted} />
            </Pressable>
            {dueDate ? (
              <Button
                size="sm"
                variant="ghost"
                className="mt-1 self-start"
                onPress={() => setDueDate(null)}
              >
                <Button.Label>Clear due date</Button.Label>
              </Button>
            ) : null}
          </FormField>

          <FormField label="Note (optional)" hint="Up to 500 characters.">
            <AppInput
              value={note}
              onChangeText={setNote}
              maxLength={500}
              multiline
              numberOfLines={4}
              className="min-h-24"
              textAlignVertical="top"
            />
          </FormField>

          <Button
            className="mt-2"
            isDisabled={updateMeta.isPending}
            onPress={() => void saveMeta()}
          >
            <Button.Label>{updateMeta.isPending ? "Saving…" : "Save due date and note"}</Button.Label>
          </Button>
        </View>

        <Button variant="secondary" className="mt-3" onPress={() => router.back()}>
          <Button.Label>Back to bill</Button.Label>
        </Button>

        {datePickerOpen ? (
          <DateTimePicker
            value={dueDate ? isoDateToDate(dueDate) : new Date()}
            mode="date"
            onChange={(event: DateTimePickerEvent, date?: Date) => {
              setDatePickerOpen(false);
              if (event.type !== "dismissed" && date) setDueDate(toIsoDate(date));
            }}
          />
        ) : null}
      </Screen>
    );
  }

  const activeTypes = (types.data ?? []).filter((type) => !type.isArchived);

  if (activeTypes.length === 0 && !types.isLoading) {
    return (
      <Screen>
        <EmptyState
          icon="pricetags-outline"
          title="No active bill types"
          description="Add or restore a bill type before creating a bill."
          actionLabel="Manage bill types"
          onAction={() => router.push("/bill-types")}
        />
      </Screen>
    );
  }

  if (pickerBoarders.length === 0 && !activeBoarders.isLoading) {
    return (
      <Screen>
        <EmptyState
          icon="people-outline"
          title="No active boarders"
          description="Add the people staying in the boarding house first."
          actionLabel="Go to boarders"
          onAction={() => router.push("/boarders")}
        />
      </Screen>
    );
  }

  return (
    <Screen>
      <View className="pt-4">
        <Text className="text-2xl font-semibold text-foreground">
          {editId ? "Edit bill" : "Create a bill"}
        </Text>
        <Text className="mt-1 text-sm text-muted">
          Choose a type and month, then split the total among boarders.
        </Text>
      </View>

      {formError ? (
        <View className="mt-4">
          <ErrorBlock message={formError} />
        </View>
      ) : null}

      {types.isLoading ? (
        <View className="mt-4">
          <ChipRowSkeleton />
          <FormSkeleton fields={3} />
        </View>
      ) : (
        <>
          <SectionHeader title="Bill type" subtitle="One type per bill." />
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            contentContainerStyle={{ gap: 8 }}
          >
            {activeTypes.map((type) => {
              const selected = type.id === billTypeId;
              return (
                <Pressable
                  key={type.id}
                  accessibilityRole="button"
                  accessibilityState={{ selected }}
                  onPress={() => setBillTypeId(type.id)}
                  className={cn(
                    "min-h-11 items-center justify-center rounded-full border px-4",
                    selected ? "border-accent bg-accent" : "border-separator bg-surface",
                  )}
                >
                  <Text
                    className={cn(
                      "text-sm font-medium",
                      selected ? "text-accent-foreground" : "text-foreground",
                    )}
                  >
                    {type.name}
                  </Text>
                </Pressable>
              );
            })}
          </ScrollView>

          <SectionHeader title="Billing period" />
          <MonthSwitcher
            year={period.year}
            month={period.month}
            onChange={(year, month) => setPeriod({ year, month })}
          />

          <SectionHeader title="Amount and details" />
          <FormField
            label={`Total bill (${currency})`}
            hint={
              allowCentavos
                ? "Use a decimal point for centavos, e.g. 1234.50."
                : "Whole pesos only. Turn on centavos in Settings to use decimals."
            }
            required
          >
            <AppInput
              value={totalInput}
              onChangeText={(value) => {
                setTotalInput(value);
                if (formError) setFormError(null);
              }}
              keyboardType={allowCentavos ? "decimal-pad" : "number-pad"}
              placeholder="0.00"
              maxLength={16}
            />
          </FormField>

          <FormField label="Due date (optional)">
            <Pressable
              accessibilityRole="button"
              className="min-h-12 flex-row items-center justify-between rounded-xl border border-separator bg-surface px-3"
              onPress={() => setDatePickerOpen(true)}
            >
              <Text className={dueDate ? "text-sm text-foreground" : "text-sm text-muted"}>
                {dueDate ? formatDateOnly(dueDate) : "No due date"}
              </Text>
              <Ionicons name="calendar-outline" size={18} color={muted} />
            </Pressable>
            {dueDate ? (
              <Button
                size="sm"
                variant="ghost"
                className="mt-1 self-start"
                onPress={() => setDueDate(null)}
              >
                <Button.Label>Clear due date</Button.Label>
              </Button>
            ) : null}
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
              placeholder="e.g. October water bill"
            />
          </FormField>

          <SectionHeader
            title={`Boarders (${selectedIds.length} selected)`}
            subtitle="All active boarders start selected."
            action={
              <View className="flex-row gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  onPress={() =>
                    setSelectedIds(
                      pickerBoarders
                        .filter((boarder) => boarder.status === "active")
                        .map((boarder) => boarder.id),
                    )
                  }
                >
                  <Button.Label>All</Button.Label>
                </Button>
                <Button size="sm" variant="ghost" onPress={() => setSelectedIds([])}>
                  <Button.Label>None</Button.Label>
                </Button>
              </View>
            }
          />

          <Surface variant="secondary" className="rounded-2xl p-2">
            {pickerBoarders.map((boarder) => {
              const selected = selectedIds.includes(boarder.id);
              return (
                <Pressable
                  key={boarder.id}
                  accessibilityRole="checkbox"
                  accessibilityState={{ checked: selected }}
                  accessibilityLabel={`${boarder.name}${boarder.status === "archived" ? ", archived" : ""}`}
                  onPress={() => toggleBoarder(boarder.id)}
                  className="min-h-12 flex-row items-center gap-3 rounded-xl px-2"
                >
                  <Ionicons
                    name={selected ? "checkbox" : "square-outline"}
                    size={22}
                    color={selected ? accent : muted}
                  />
                  <Text className="flex-1 text-sm text-foreground">{boarder.name}</Text>
                  {boarder.status === "archived" ? (
                    <Text className="text-xs text-muted">Archived</Text>
                  ) : null}
                </Pressable>
              );
            })}
          </Surface>

          <SectionHeader title="How to divide" subtitle="Shares must add up to the total." />
          <View className="flex-row rounded-xl bg-surface p-1">
            {(
              [
                ["equal", "Equal split"],
                ["custom", "Custom amounts"],
              ] as const
            ).map(([value, label]) => (
              <Pressable
                key={value}
                accessibilityRole="tab"
                accessibilityState={{ selected: method === value }}
                onPress={() => chooseMethod(value)}
                className={cn(
                  "min-h-11 flex-1 items-center justify-center rounded-lg",
                  method === value ? "bg-accent" : "bg-transparent",
                )}
              >
                <Text
                  className={cn(
                    "text-sm font-medium",
                    method === value ? "text-accent-foreground" : "text-foreground",
                  )}
                >
                  {label}
                </Text>
              </Pressable>
            ))}
          </View>

          <Surface variant="secondary" className="mt-3 rounded-2xl p-4">
            {selectedBoarders.length === 0 ? (
              <Text className="text-sm text-muted">Select at least one boarder to see shares.</Text>
            ) : method === "equal" ? (
              equalResult && "error" in equalResult ? (
                <Text className="text-sm text-danger">{equalResult.error}</Text>
              ) : equalResult ? (
                <>
                  {equalResult.shares.map((share) => {
                    const boarder = selectedBoarders.find(
                      (item) => item.id === share.boarderId,
                    );
                    return (
                      <View
                        key={share.boarderId}
                        className="flex-row items-center justify-between border-b border-separator py-2 last:border-b-0"
                      >
                        <Text className="text-sm text-foreground">{boarder?.name}</Text>
                        <Text
                          className="text-sm font-semibold text-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {formatMoney(share.amountMinor, currency)}
                        </Text>
                      </View>
                    );
                  })}
                  {parsedTotal !== null && parsedTotal % selectedBoarders.length !== 0 ? (
                    <Text className="mt-2 text-xs text-muted">
                      The remainder is assigned one centavo at a time in the order you selected boarders.
                    </Text>
                  ) : null}
                </>
              ) : null
            ) : (
              <>
                {pinnedIds.length === 0 ? (
                  <Text className="mb-2 text-xs text-muted">
                    Turn on Custom for any boarder to override their amount.
                  </Text>
                ) : null}
                {selectedBoarders.map((boarder) => {
                  const pinned = pinnedIds.includes(boarder.id);
                  const computed =
                    customResult && !("error" in customResult)
                      ? customResult.shares.find((share) => share.boarderId === boarder.id)
                      : undefined;
                  const stale = pinned ? undefined : customInputs[boarder.id];

                  return (
                    <View key={boarder.id} className="mb-2 rounded-xl bg-surface p-3">
                      <View className="flex-row items-center gap-2">
                        <Text className="flex-1 text-sm text-foreground">{boarder.name}</Text>
                        <Pressable
                          accessibilityRole="checkbox"
                          accessibilityState={{ checked: pinned }}
                          accessibilityLabel={`Custom amount for ${boarder.name}`}
                          className="min-h-11 flex-row items-center gap-1"
                          onPress={() => togglePinned(boarder.id)}
                        >
                          <Ionicons
                            name={pinned ? "checkbox" : "square-outline"}
                            size={20}
                            color={pinned ? accent : muted}
                          />
                          <Text className="text-xs font-medium text-muted">Custom</Text>
                        </Pressable>
                      </View>
                      {pinned ? (
                        <AppInput
                          className="mt-2"
                          value={customInputs[boarder.id] ?? ""}
                          onChangeText={(value) =>
                            setCustomInputs((current) => ({ ...current, [boarder.id]: value }))
                          }
                          keyboardType={allowCentavos ? "decimal-pad" : "number-pad"}
                          placeholder="0.00"
                        />
                      ) : (
                        <View className="mt-1 flex-row items-baseline justify-between gap-2">
                          <Text className="flex-1 text-xs text-muted">
                            Auto · split equally
                            {stale ? ` · was ${stale}` : ""}
                          </Text>
                          <Text
                            className="text-sm font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {computed ? formatMoney(computed.amountMinor, currency) : "—"}
                          </Text>
                        </View>
                      )}
                    </View>
                  );
                })}
                <View className="flex-row items-center justify-between border-t border-separator pt-2">
                  <Text className="text-sm text-muted">Custom amounts</Text>
                  <Text className="text-sm font-semibold text-foreground">
                    {formatMoney(customAssignedMinor, currency)}
                  </Text>
                </View>
                <View className="mt-1 flex-row items-center justify-between gap-2">
                  <Text className="text-sm text-muted">Leftover</Text>
                  <Text className="flex-1 text-right text-xs text-muted">
                    {poolCount === 0
                      ? "every boarder is custom"
                      : `${formatMoney(Math.max(0, leftoverMinor), currency)} split equally among ${poolCount} ${poolCount === 1 ? "boarder" : "boarders"}`}
                  </Text>
                </View>
                {customResult && !("error" in customResult) && customResult.rounded ? (
                  <Text className="mt-1 text-xs text-muted">
                    Rounded to whole pesos. Turn on centavos in Settings for exact division.
                  </Text>
                ) : null}
              </>
            )}
          </Surface>

          {splitError ? <Text className="mt-2 text-xs text-danger">{splitError}</Text> : null}

          <Button
            className="mt-5"
            size="lg"
            isDisabled={!sharesForSave || !billTypeId || createBill.isPending || updateBill.isPending}
            onPress={() => void performSave(false)}
          >
            <Button.Label>
              {createBill.isPending || updateBill.isPending
                ? "Saving…"
                : editId
                  ? "Save changes"
                  : "Create bill"}
            </Button.Label>
          </Button>

          {editId ? (
            <Button variant="ghost" className="mt-2" onPress={() => router.back()}>
              <Button.Label>Cancel</Button.Label>
            </Button>
          ) : null}
        </>
      )}

      {datePickerOpen ? (
        <DateTimePicker
          value={dueDate ? isoDateToDate(dueDate) : new Date()}
          mode="date"
          onChange={(event: DateTimePickerEvent, date?: Date) => {
            setDatePickerOpen(false);
            if (event.type !== "dismissed" && date) setDueDate(toIsoDate(date));
          }}
        />
      ) : null}

      <AppDialog
        open={duplicateOpen}
        onOpenChange={setDuplicateOpen}
        title="A bill already exists for this type and month"
        description="You can still create another one if you need a separate bill, but check that this is not accidental."
        actions={
          <>
            <Button variant="ghost" size="sm" onPress={() => setDuplicateOpen(false)}>
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button
              size="sm"
              isDisabled={createBill.isPending || updateBill.isPending}
              onPress={() => {
                setDuplicateOpen(false);
                void performSave(true);
              }}
            >
              <Button.Label>Create anyway</Button.Label>
            </Button>
          </>
        }
      >
        <View className="gap-2">
          {duplicates?.map((duplicate) => (
            <Surface key={duplicate.id} variant="secondary" className="rounded-xl p-3">
              <Text className="text-sm font-medium text-foreground">
                {duplicate.billTypeNameSnapshot} · {formatPeriod(duplicate.periodYear, duplicate.periodMonth)}
              </Text>
              <Text className="mt-0.5 text-xs text-muted">
                {formatMoney(duplicate.totalMinor, currency)}
                {duplicate.archivedAt ? " · Archived" : ""}
              </Text>
            </Surface>
          ))}
        </View>
      </AppDialog>
    </Screen>
  );
}
