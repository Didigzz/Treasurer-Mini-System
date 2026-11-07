import { Ionicons } from "@expo/vector-icons";
import { router, useFocusEffect } from "expo-router";
import { cn } from "heroui-native";
import { useCallback, useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";

import {
  computeHistoryTotals,
  matchesHistoryFilter,
} from "@my-better-t-app/api/treasurer/calculations";
import { formatMoney, formatPeriod } from "@my-better-t-app/api/treasurer/money";
import type { HistoryFilter } from "@my-better-t-app/api/treasurer/types";
import { HistoryBillRow } from "@/src/features/treasurer/components/rows";
import {
  AppDialog,
  EmptyState,
  ErrorBlock,
  LoadingBlock,
  Screen,
  SummaryCard,
} from "@/src/features/treasurer/components/ui";
import { useAppSettings, useBillHistory } from "@/src/features/treasurer/queries";

const ALL = "all";

function FilterSelect({
  label,
  selectedKey,
  options,
  onChange,
}: {
  label: string;
  selectedKey: string;
  options: Array<{ key: string; label: string }>;
  onChange: (key: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const selectedLabel =
    options.find((option) => option.key === selectedKey)?.label ?? options[0]?.label ?? "";

  return (
    <>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${label}: ${selectedLabel}. Change filter.`}
        onPress={() => setOpen(true)}
        className="min-h-11 flex-1 flex-row items-center justify-between gap-2 rounded-xl border border-separator bg-surface px-3"
      >
        <Text className="flex-1 text-sm text-foreground" numberOfLines={1}>
          {selectedLabel}
        </Text>
        <Ionicons name="chevron-down" size={15} className="text-muted" />
      </Pressable>

      <AppDialog open={open} onOpenChange={setOpen} title={`Filter by ${label.toLowerCase()}`}>
        <View className="gap-2">
          {options.map((option) => {
            const selected = option.key === selectedKey;
            return (
              <Pressable
                key={option.key}
                accessibilityRole="button"
                accessibilityState={{ selected }}
                onPress={() => {
                  onChange(option.key);
                  setOpen(false);
                }}
                className={cn(
                  "min-h-11 justify-center rounded-xl border px-3",
                  selected ? "border-accent bg-accent" : "border-separator bg-surface",
                )}
              >
                <Text
                  className={cn(
                    "text-sm font-medium",
                    selected ? "text-accent-foreground" : "text-foreground",
                  )}
                >
                  {option.label}
                </Text>
              </Pressable>
            );
          })}
        </View>
      </AppDialog>
    </>
  );
}

export default function HistoryScreen() {
  const history = useBillHistory();
  const appSettings = useAppSettings();
  const currency = appSettings.data?.currencyCode ?? "PHP";

  const [periodKey, setPeriodKey] = useState(ALL);
  const [typeId, setTypeId] = useState(ALL);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());

  // Tabs stay mounted, so clear the filters every time History is opened.
  useFocusEffect(
    useCallback(() => {
      setPeriodKey(ALL);
      setTypeId(ALL);
      setExpanded(new Set());
    }, []),
  );

  const data = history.data;

  const filter = useMemo<HistoryFilter>(() => {
    const [yearText, monthText] = periodKey.split("-");
    return {
      year: periodKey === ALL ? null : Number(yearText),
      month: periodKey === ALL ? null : Number(monthText),
      billTypeId: typeId === ALL ? null : typeId,
    };
  }, [periodKey, typeId]);

  const periodOptions = useMemo(
    () => [
      { key: ALL, label: "All months" },
      ...(data?.periods ?? []).map((period) => ({
        key: `${period.year}-${String(period.month).padStart(2, "0")}`,
        label: `${formatPeriod(period.year, period.month)} (${period.billCount})`,
      })),
    ],
    [data],
  );

  const typeOptions = useMemo(
    () => [
      { key: ALL, label: "All types" },
      ...(data?.billTypes ?? []).map((type) => ({
        key: type.id,
        label: `${type.name} (${type.billCount})`,
      })),
    ],
    [data],
  );

  const visible = useMemo(
    () => (data?.bills ?? []).filter((item) => matchesHistoryFilter(item.bill, filter)),
    [data, filter],
  );

  const totals = useMemo(
    () => computeHistoryTotals(visible.map((item) => item.summary)),
    [visible],
  );

  function toggleExpanded(id: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <Screen>
      <View className="pb-1 pt-4">
        <Text className="text-xs font-medium uppercase tracking-wide text-muted">
          Boarding house treasurer
        </Text>
        <Text className="mt-1 text-2xl font-semibold text-foreground">History</Text>
        <Text className="mt-1 text-sm text-muted">
          Every bill recorded, across all months. Read only.
        </Text>
      </View>

      {history.isLoading ? (
        <LoadingBlock label="Loading bill history…" />
      ) : history.isError || !data ? (
        <ErrorBlock
          message="The bill history could not be loaded."
          onRetry={() => void history.refetch()}
        />
      ) : (
        <>
          <View className="mt-4 flex-row gap-2">
            <FilterSelect
              label="Month"
              selectedKey={periodKey}
              options={periodOptions}
              onChange={setPeriodKey}
            />
            <FilterSelect
              label="Bill type"
              selectedKey={typeId}
              options={typeOptions}
              onChange={setTypeId}
            />
          </View>

          <View className="mt-4 gap-2">
            <View className="flex-row gap-2">
              <SummaryCard
                label="Expected"
                value={formatMoney(totals.expectedMinor, currency)}
                icon="receipt-outline"
              />
              <SummaryCard
                label="Received"
                value={formatMoney(totals.appliedMinor, currency)}
                tone="success"
                icon="checkmark-circle-outline"
              />
            </View>
            <View className="flex-row gap-2">
              <SummaryCard
                label="Still owed"
                value={formatMoney(totals.outstandingMinor, currency)}
                tone={totals.outstandingMinor > 0 ? "danger" : "success"}
                icon="alert-circle-outline"
              />
              <SummaryCard
                label="Change to return"
                value={formatMoney(totals.changeOutstandingMinor, currency)}
                tone={totals.changeOutstandingMinor > 0 ? "warning" : "default"}
                icon="cash-outline"
              />
            </View>
          </View>

          <Text className="mt-3 text-xs text-muted">
            {visible.length} {visible.length === 1 ? "bill" : "bills"} · includes archived bills
          </Text>

          <View className="mt-4">
            {visible.length === 0 ? (
              data.bills.length === 0 ? (
                <EmptyState
                  icon="file-tray-outline"
                  title="No bills yet"
                  description="Create a bill from the Dashboard to start the history."
                  actionLabel="Go to Dashboard"
                  onAction={() => router.push("/")}
                />
              ) : (
                <EmptyState
                  icon="filter-outline"
                  title="No bills match this filter"
                  description="Try another month or bill type."
                  actionLabel="Show all"
                  onAction={() => {
                    setPeriodKey(ALL);
                    setTypeId(ALL);
                  }}
                />
              )
            ) : (
              visible.map((item) => (
                <HistoryBillRow
                  key={item.bill.id}
                  item={item}
                  currency={currency}
                  expanded={expanded.has(item.bill.id)}
                  onToggleExpanded={() => toggleExpanded(item.bill.id)}
                  onOpen={() =>
                    router.push({
                      pathname: "/history/[id]",
                      params: { id: item.bill.id },
                    })
                  }
                />
              ))
            )}
          </View>
        </>
      )}
    </Screen>
  );
}
