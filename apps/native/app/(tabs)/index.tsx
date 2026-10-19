import { Ionicons } from "@expo/vector-icons";
import { router } from "expo-router";
import { Button, Surface, useThemeColor } from "heroui-native";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";

import {
  matchesBoarderFilter,
} from "@/src/features/treasurer/calculations";
import { BillCard, BoarderBalanceRow } from "@/src/features/treasurer/components/rows";
import {
  EmptyState,
  ErrorBlock,
  FilterChips,
  LoadingBlock,
  MonthSwitcher,
  Screen,
  SectionHeader,
  SummaryCard,
} from "@/src/features/treasurer/components/ui";
import { currentPeriod, formatMoney, formatPeriod } from "@/src/features/treasurer/money";
import { useAppSettings, useDashboard } from "@/src/features/treasurer/queries";
import type { ShareFilter } from "@/src/features/treasurer/types";

export default function DashboardScreen() {
  const [period, setPeriod] = useState(() => currentPeriod());
  const [includeArchived, setIncludeArchived] = useState(false);
  const [filter, setFilter] = useState<ShareFilter>("all");

  const dashboard = useDashboard(period.year, period.month, includeArchived);
  const appSettings = useAppSettings();
  const currency = appSettings.data?.currencyCode ?? "PHP";
  const accent = useThemeColor("accent");

  const data = dashboard.data;
  const filteredBoarders = useMemo(
    () =>
      (data?.boarderSummaries ?? []).filter((summary) =>
        matchesBoarderFilter(summary, filter),
      ),
    [data, filter],
  );

  const filterCounts = useMemo(() => {
    const summaries = data?.boarderSummaries ?? [];
    return {
      all: summaries.length,
      unpaid: summaries.filter((item) => matchesBoarderFilter(item, "unpaid")).length,
      paid: summaries.filter((item) => matchesBoarderFilter(item, "paid")).length,
      change: summaries.filter((item) => matchesBoarderFilter(item, "change")).length,
    };
  }, [data]);

  return (
    <Screen>
      <View className="pb-1 pt-4">
        <Text className="text-sm font-medium text-muted">Boarding house treasurer</Text>
        <View className="mt-1 flex-row items-center justify-between gap-2">
          <MonthSwitcher
            year={period.year}
            month={period.month}
            onChange={(year, month) => setPeriod({ year, month })}
          />
          <Button
            size="sm"
            onPress={() =>
              router.push({
                pathname: "/bills/new",
                params: { year: String(period.year), month: String(period.month) },
              })
            }
          >
            <Ionicons name="add" size={17} color={accent} />
            <Button.Label>Add bill</Button.Label>
          </Button>
        </View>
      </View>

      {dashboard.isLoading ? (
        <LoadingBlock label="Loading this month’s records…" />
      ) : dashboard.isError ? (
        <ErrorBlock
          message="Your dashboard could not be loaded."
          onRetry={() => void dashboard.refetch()}
        />
      ) : data ? (
        <>
          <View className="mt-4 gap-2">
            <View className="flex-row gap-2">
              <SummaryCard
                label="Expected"
                value={formatMoney(data.totals.expectedMinor, currency)}
                icon="receipt-outline"
              />
              <SummaryCard
                label="Received"
                value={formatMoney(data.totals.appliedMinor, currency)}
                tone="success"
                icon="checkmark-circle-outline"
              />
            </View>
            <View className="flex-row gap-2">
              <SummaryCard
                label="Still owed"
                value={formatMoney(data.totals.outstandingMinor, currency)}
                tone={data.totals.outstandingMinor > 0 ? "danger" : "success"}
                icon="alert-circle-outline"
              />
              <SummaryCard
                label="Change to return"
                value={formatMoney(data.totals.changeOutstandingMinor, currency)}
                tone={data.totals.changeOutstandingMinor > 0 ? "warning" : "default"}
                icon="cash-outline"
              />
            </View>
          </View>

          <View className="mt-3 flex-row items-center justify-between gap-2">
            <Text className="text-xs text-muted">
              {data.totals.paidShareCount} of {data.totals.totalShareCount} shares paid
              {data.totals.partialShareCount > 0
                ? ` • ${data.totals.partialShareCount} partial`
                : ""}
            </Text>
            {data.archivedBillCount > 0 || includeArchived ? (
              <Pressable
                accessibilityRole="switch"
                accessibilityState={{ checked: includeArchived }}
                onPress={() => setIncludeArchived((value) => !value)}
                className="min-h-11 flex-row items-center gap-1.5"
              >
                <Ionicons
                  name={includeArchived ? "checkbox" : "square-outline"}
                  size={16}
                  color={accent}
                />
                <Text className="text-xs font-medium text-accent">
                  Show archived ({data.archivedBillCount})
                </Text>
              </Pressable>
            ) : null}
          </View>
          <Text className="mt-1 text-[11px] text-muted">
            {formatPeriod(period.year, period.month)} totals exclude archived bills.
          </Text>

          <SectionHeader
            title="Bills this month"
            subtitle="Tap a bill to record payments and see every boarder."
          />
          {data.bills.length === 0 ? (
            <EmptyState
              icon="document-text-outline"
              title={`No bills for ${formatPeriod(period.year, period.month)}`}
              description="Create a bill for water, rent, electricity, or another shared expense."
              actionLabel="Create a bill"
              onAction={() =>
                router.push({
                  pathname: "/bills/new",
                  params: { year: String(period.year), month: String(period.month) },
                })
              }
            />
          ) : (
            data.bills.map((item) => (
              <BillCard
                key={item.bill.id}
                item={item}
                currency={currency}
                onPress={() =>
                  router.push({ pathname: "/bills/[id]", params: { id: item.bill.id } })
                }
              />
            ))
          )}

          <SectionHeader
            title="Boarders this month"
            subtitle="Balances combine every bill in the selected month."
          />
          {data.boarderSummaries.length === 0 ? (
            <EmptyState
              icon="people-outline"
              title="No boarder balances yet"
              description="Add boarders, then include them in a bill."
              actionLabel="Go to boarders"
              onAction={() => router.push("/boarders")}
            />
          ) : (
            <>
              <FilterChips value={filter} onChange={setFilter} counts={filterCounts} />
              <View className="mt-3">
                {filteredBoarders.length === 0 ? (
                  <EmptyState
                    icon="filter-outline"
                    title="No boarders match this filter"
                    description="Try another filter or clear it."
                    actionLabel="Show all"
                    onAction={() => setFilter("all")}
                  />
                ) : (
                  filteredBoarders.map((summary) => (
                    <BoarderBalanceRow
                      key={summary.boarderId}
                      name={summary.boarderName}
                      archived={summary.boarderArchived}
                      expectedMinor={summary.expectedMinor}
                      appliedMinor={summary.appliedMinor}
                      outstandingMinor={summary.outstandingMinor}
                      changeOutstandingMinor={summary.changeOutstandingMinor}
                      currency={currency}
                      onPress={() =>
                        router.push({
                          pathname: "/boarders/[id]",
                          params: { id: summary.boarderId },
                        })
                      }
                    />
                  ))
                )}
              </View>
            </>
          )}
        </>
      ) : null}
    </Screen>
  );
}
