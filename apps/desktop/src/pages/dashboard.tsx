import { Link } from "@tanstack/react-router";
import {
  AlertCircle,
  Archive,
  Banknote,
  CheckCircle2,
  ChevronRight,
  Plus,
  Receipt,
} from "lucide-react";
import { useMemo, useState } from "react";

import {
  matchesBoarderFilter,
} from "@my-better-t-app/api/treasurer/calculations";
import {
  currentPeriod,
  daysOverdue,
  formatDateOnly,
  formatMoney,
  formatPeriod,
  isOverdue,
} from "@my-better-t-app/api/treasurer/money";
import type { ShareFilter } from "@my-better-t-app/api/treasurer/types";
import {
  Badge,
  BillStatusBadge,
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  FilterChips,
  LoadingBlock,
  MonthSwitcher,
  PageHeader,
  SectionHeader,
  SummaryCard,
  cn,
} from "@/components/ui";
import { useAppSettings, useDashboard } from "@/queries";

export function DashboardPage() {
  const [period, setPeriod] = useState(() => currentPeriod());
  const [includeArchived, setIncludeArchived] = useState(false);
  const [filter, setFilter] = useState<ShareFilter>("all");

  const dashboard = useDashboard(period.year, period.month, includeArchived);
  const settings = useAppSettings();
  const currency = settings.data?.currencyCode ?? "PHP";
  const data = dashboard.data;

  const filteredBoarders = useMemo(
    () =>
      (data?.boarderSummaries ?? []).filter((summary) => matchesBoarderFilter(summary, filter)),
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
    <div>
      <PageHeader
        eyebrow="Boarding house treasurer"
        title="Dashboard"
        description="Every bill, payment, and boarder balance for the selected month."
        actions={
          <>
            <MonthSwitcher
              year={period.year}
              month={period.month}
              onChange={(year, month) => setPeriod({ year, month })}
            />
            <Link to="/bills/new" search={{ year: period.year, month: period.month }}>
              <Button>
                <Plus size={16} />
                Add bill
              </Button>
            </Link>
          </>
        }
      />

      {dashboard.isLoading ? (
        <LoadingBlock label="Loading this month's records…" />
      ) : dashboard.isError ? (
        <div className="mt-6">
          <ErrorBlock
            message="Your dashboard could not be loaded."
            onRetry={() => void dashboard.refetch()}
          />
        </div>
      ) : data ? (
        <>
          <div className="mt-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <SummaryCard
              label="Expected"
              value={formatMoney(data.totals.expectedMinor, currency)}
              icon={<Receipt size={16} />}
            />
            <SummaryCard
              label="Received"
              value={formatMoney(data.totals.appliedMinor, currency)}
              tone="success"
              icon={<CheckCircle2 size={16} />}
            />
            <SummaryCard
              label="Still owed"
              value={formatMoney(data.totals.outstandingMinor, currency)}
              tone={data.totals.outstandingMinor > 0 ? "danger" : "success"}
              icon={<AlertCircle size={16} />}
            />
            <SummaryCard
              label="Change to return"
              value={formatMoney(data.totals.changeOutstandingMinor, currency)}
              tone={data.totals.changeOutstandingMinor > 0 ? "warning" : "neutral"}
              icon={<Banknote size={16} />}
            />
          </div>

          <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
            <p className="text-xs text-muted">
              {data.totals.paidShareCount} of {data.totals.totalShareCount} shares paid
              {data.totals.partialShareCount > 0
                ? ` · ${data.totals.partialShareCount} partial`
                : ""}
              {" · "}Totals exclude archived bills.
            </p>
            {data.archivedBillCount > 0 || includeArchived ? (
              <label className="flex cursor-pointer items-center gap-2 text-xs font-semibold text-accent">
                <input
                  type="checkbox"
                  checked={includeArchived}
                  onChange={(event) => setIncludeArchived(event.target.checked)}
                  className="h-3.5 w-3.5 accent-accent"
                />
                Show archived ({data.archivedBillCount})
              </label>
            ) : null}
          </div>

          <SectionHeader
            title="Bills this month"
            subtitle="Open a bill to record payments and see every boarder."
          />

          {data.bills.length === 0 ? (
            <EmptyState
              title={`No bills for ${formatPeriod(period.year, period.month)}`}
              description="Create a bill for water, rent, electricity, or another shared expense."
              action={
                <Link to="/bills/new" search={{ year: period.year, month: period.month }}>
                  <Button size="sm">
                    <Plus size={15} />
                    Create a bill
                  </Button>
                </Link>
              }
            />
          ) : (
            <div className="grid gap-2.5">
              {data.bills.map((item) => {
                const overdue = isOverdue(
                  item.bill.dueDate,
                  item.summary.outstandingMinor > 0,
                );
                return (
                  <Link
                    key={item.bill.id}
                    to="/bills/$id"
                    params={{ id: item.bill.id }}
                    className="card block p-4 transition-shadow hover:shadow-sm"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-2">
                          <p className="text-sm font-bold text-ink">{item.bill.billTypeNameSnapshot}</p>
                          <BillStatusBadge status={item.summary.status} />
                          {overdue && item.bill.dueDate ? (
                            <Badge tone="danger">Overdue {daysOverdue(item.bill.dueDate)}d</Badge>
                          ) : null}
                          {item.summary.needsReview ? <Badge tone="warning">Review change</Badge> : null}
                        </div>
                        <p className="mt-1 text-xs text-muted">
                          {formatMoney(item.bill.totalMinor, currency)} total ·{" "}
                          {item.summary.paidShareCount}/{item.summary.totalShareCount} paid
                          {item.bill.dueDate
                            ? ` · due ${formatDateOnly(item.bill.dueDate)}`
                            : ""}
                        </p>
                      </div>
                      <div className="text-right">
                        <p className="text-sm font-bold tabular-nums text-ink">
                          {formatMoney(item.summary.outstandingMinor, currency)}
                        </p>
                        <p className="text-[11px] text-muted">still owed</p>
                      </div>
                    </div>

                    <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-canvas">
                      <div
                        className={cn(
                          "h-full rounded-full transition-all",
                          item.summary.status === "archived"
                            ? "bg-muted/40"
                            : item.summary.progress >= 1
                              ? "bg-success"
                              : "bg-accent",
                        )}
                        style={{ width: `${Math.round(item.summary.progress * 100)}%` }}
                      />
                    </div>

                    {item.bill.note ? (
                      <p className="mt-2.5 line-clamp-1 text-xs text-muted">{item.bill.note}</p>
                    ) : null}
                  </Link>
                );
              })}
            </div>
          )}

          <SectionHeader
            title="Boarders this month"
            subtitle="Balances combine every non-archived bill in the selected month."
          />

          {data.boarderSummaries.length === 0 ? (
            <EmptyState
              title="No boarder balances yet"
              description="Add boarders, then include them in a bill."
              action={
                <Link to="/boarders">
                  <Button size="sm" variant="secondary">
                    Go to boarders
                  </Button>
                </Link>
              }
            />
          ) : (
            <>
              <FilterChips value={filter} onChange={setFilter} counts={filterCounts} />
              <Card className="mt-3 divide-y divide-line">
                {filteredBoarders.length === 0 ? (
                  <div className="px-4 py-8">
                    <EmptyState
                      title="No boarders match this filter"
                      description="Try another filter."
                      action={
                        <Button size="sm" variant="secondary" onClick={() => setFilter("all")}>
                          Show all
                        </Button>
                      }
                    />
                  </div>
                ) : (
                  filteredBoarders.map((summary) => (
                    <Link
                      key={summary.boarderId}
                      to="/boarders/$id"
                      params={{ id: summary.boarderId }}
                      className="flex items-center justify-between gap-4 px-4 py-3 transition-colors hover:bg-canvas/60"
                    >
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-2">
                          <p className="truncate text-sm font-semibold text-ink">
                            {summary.boarderName}
                          </p>
                          {summary.boarderArchived ? (
                            <Badge>
                              <Archive size={11} />
                              Archived
                            </Badge>
                          ) : null}
                        </div>
                        <p className="mt-0.5 text-xs text-muted">
                          {formatMoney(summary.appliedMinor, currency)} of{" "}
                          {formatMoney(summary.expectedMinor, currency)} · {summary.shareCount}{" "}
                          {summary.shareCount === 1 ? "share" : "shares"}
                        </p>
                      </div>
                      <div className="flex items-center gap-4 text-right">
                        <div>
                          <p
                            className={cn(
                              "text-sm font-bold tabular-nums",
                              summary.outstandingMinor > 0 ? "text-danger" : "text-success",
                            )}
                          >
                            {formatMoney(summary.outstandingMinor, currency)}
                          </p>
                          <p className="text-[11px] text-muted">
                            {summary.outstandingMinor > 0 ? "owed" : "settled"}
                          </p>
                        </div>
                        {summary.changeOutstandingMinor > 0 ? (
                          <div>
                            <p className="text-sm font-bold tabular-nums text-warning">
                              {formatMoney(summary.changeOutstandingMinor, currency)}
                            </p>
                            <p className="text-[11px] text-muted">change</p>
                          </div>
                        ) : null}
                        <ChevronRight size={16} className="text-muted" />
                      </div>
                    </Link>
                  ))
                )}
              </Card>
            </>
          )}
        </>
      ) : null}
    </div>
  );
}
