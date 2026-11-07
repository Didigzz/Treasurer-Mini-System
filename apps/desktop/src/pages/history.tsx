import { Link } from "@tanstack/react-router";
import { AlertCircle, Banknote, CheckCircle2, ChevronDown, Receipt } from "lucide-react";
import { useMemo, useState } from "react";

import {
  computeHistoryTotals,
  matchesHistoryFilter,
} from "@my-better-t-app/api/treasurer/calculations";
import {
  daysOverdue,
  formatMoney,
  formatPeriod,
  isOverdue,
} from "@my-better-t-app/api/treasurer/money";
import type { HistoryFilter } from "@my-better-t-app/api/treasurer/types";
import {
  Badge,
  BillStatusBadge,
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  LoadingBlock,
  PageHeader,
  ShareStatusBadge,
  SummaryCard,
  cn,
} from "@/components/ui";
import { useAppSettings, useBillHistory } from "@/queries";

const ALL = "all";

export function HistoryPage() {
  const history = useBillHistory();
  const settings = useAppSettings();
  const currency = settings.data?.currencyCode ?? "PHP";

  const [periodKey, setPeriodKey] = useState(ALL);
  const [typeId, setTypeId] = useState(ALL);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());

  const data = history.data;

  const filter = useMemo<HistoryFilter>(() => {
    const [yearText, monthText] = periodKey.split("-");
    return {
      year: periodKey === ALL ? null : Number(yearText),
      month: periodKey === ALL ? null : Number(monthText),
      billTypeId: typeId === ALL ? null : typeId,
    };
  }, [periodKey, typeId]);

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

  function clearFilters() {
    setPeriodKey(ALL);
    setTypeId(ALL);
  }

  if (history.isLoading) return <LoadingBlock label="Loading bill history…" />;

  if (history.isError || !data) {
    return (
      <ErrorBlock
        message="The bill history could not be loaded."
        onRetry={() => void history.refetch()}
      />
    );
  }

  return (
    <div>
      <PageHeader
        eyebrow="Boarding house treasurer"
        title="History"
        description="Every bill recorded on this computer, across all months. Read only."
      />

      <div className="mt-5 flex flex-wrap items-end gap-3">
        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold text-ink">Month</span>
          <select
            className="field min-w-48"
            value={periodKey}
            onChange={(event) => setPeriodKey(event.target.value)}
          >
            <option value={ALL}>All months</option>
            {data.periods.map((period) => (
              <option
                key={`${period.year}-${period.month}`}
                value={`${period.year}-${String(period.month).padStart(2, "0")}`}
              >
                {formatPeriod(period.year, period.month)} ({period.billCount})
              </option>
            ))}
          </select>
        </label>

        <label className="block">
          <span className="mb-1.5 block text-xs font-semibold text-ink">Bill type</span>
          <select
            className="field min-w-44"
            value={typeId}
            onChange={(event) => setTypeId(event.target.value)}
          >
            <option value={ALL}>All types</option>
            {data.billTypes.map((type) => (
              <option key={type.id} value={type.id}>
                {type.name} ({type.billCount})
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <SummaryCard
          label="Expected"
          value={formatMoney(totals.expectedMinor, currency)}
          icon={<Receipt size={16} />}
        />
        <SummaryCard
          label="Received"
          value={formatMoney(totals.appliedMinor, currency)}
          tone="success"
          icon={<CheckCircle2 size={16} />}
        />
        <SummaryCard
          label="Still owed"
          value={formatMoney(totals.outstandingMinor, currency)}
          tone={totals.outstandingMinor > 0 ? "danger" : "success"}
          icon={<AlertCircle size={16} />}
        />
        <SummaryCard
          label="Change to return"
          value={formatMoney(totals.changeOutstandingMinor, currency)}
          tone={totals.changeOutstandingMinor > 0 ? "warning" : "neutral"}
          icon={<Banknote size={16} />}
        />
      </div>

      <p className="mt-3 text-xs text-muted">
        {visible.length} {visible.length === 1 ? "bill" : "bills"} · includes archived bills
      </p>

      <div className="mt-4">
        {visible.length === 0 ? (
          data.bills.length === 0 ? (
            <EmptyState
              title="No bills yet"
              description="Create a bill from the Dashboard to start the history."
              action={
                <Link to="/">
                  <Button size="sm">Go to Dashboard</Button>
                </Link>
              }
            />
          ) : (
            <EmptyState
              title="No bills match this filter"
              description="Try another month or bill type."
              action={
                <Button size="sm" variant="secondary" onClick={clearFilters}>
                  Show all
                </Button>
              }
            />
          )
        ) : (
          <div className="grid gap-2.5">
            {visible.map((item) => {
              const isOpen = expanded.has(item.bill.id);
              const tenantCount = item.shares.length;
              const overdue = isOverdue(
                item.bill.dueDate,
                item.summary.outstandingMinor > 0,
              );

              return (
                <Card key={item.bill.id} className="overflow-hidden">
                  <div className="flex items-start gap-2 p-4">
                    <Link
                      to="/history/$id"
                      params={{ id: item.bill.id }}
                      className="min-w-0 flex-1"
                    >
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <p className="truncate text-sm font-bold text-ink">
                              {item.bill.billTypeNameSnapshot}
                            </p>
                            <BillStatusBadge status={item.summary.status} />
                            {overdue && item.bill.dueDate ? (
                              <Badge tone="danger">
                                Overdue {daysOverdue(item.bill.dueDate)}d
                              </Badge>
                            ) : null}
                            {item.summary.needsReview ? (
                              <Badge tone="warning">Review change</Badge>
                            ) : null}
                          </div>
                          <p className="mt-1 text-xs text-muted">
                            {formatPeriod(item.bill.periodYear, item.bill.periodMonth)} ·{" "}
                            {tenantCount} {tenantCount === 1 ? "tenant" : "tenants"} ·{" "}
                            {item.summary.paidShareCount} of {item.summary.totalShareCount} paid
                          </p>
                        </div>
                        <div className="text-right">
                          <p className="text-sm font-bold tabular-nums text-ink">
                            {formatMoney(item.bill.totalMinor, currency)}
                          </p>
                          <p className="text-[11px] text-muted">total</p>
                        </div>
                      </div>
                    </Link>

                    <button
                      type="button"
                      aria-expanded={isOpen}
                      aria-label={`${isOpen ? "Hide" : "Show"} tenants on ${
                        item.bill.billTypeNameSnapshot
                      } ${formatPeriod(item.bill.periodYear, item.bill.periodMonth)}`}
                      onClick={() => toggleExpanded(item.bill.id)}
                      className="mt-0.5 shrink-0 rounded-lg p-1.5 text-muted transition-colors hover:bg-canvas hover:text-ink"
                    >
                      <ChevronDown
                        size={16}
                        className={cn("transition-transform", isOpen && "rotate-180")}
                      />
                    </button>
                  </div>

                  {isOpen ? (
                    <div className="border-t border-line px-4 py-3">
                      <p className="text-xs font-semibold text-ink">
                        Tenants ({tenantCount})
                      </p>
                      <p className="mt-0.5 text-[11px] text-muted">
                        Snapshot names from the time of the bill.
                      </p>
                      <div className="mt-2 divide-y divide-line">
                        {item.shares.map((share) => (
                          <div
                            key={share.id}
                            className="flex flex-wrap items-center justify-between gap-2 py-2"
                          >
                            <div className="min-w-0">
                              <div className="flex flex-wrap items-center gap-2">
                                <ShareStatusBadge status={share.state.status} />
                                <p className="truncate text-sm font-medium text-ink">
                                  {share.boarderNameSnapshot}
                                </p>
                                {share.boarderArchived ? <Badge>Archived</Badge> : null}
                                {share.amountDueMinor === 0 ? (
                                  <Badge>Nothing due</Badge>
                                ) : null}
                              </div>
                              <p className="mt-0.5 text-xs text-muted">
                                {formatMoney(share.amountDueMinor, currency)} due ·{" "}
                                {formatMoney(share.state.paidMinor, currency)} paid
                                {share.state.changeOutstandingMinor > 0
                                  ? ` · ${formatMoney(
                                      share.state.changeOutstandingMinor,
                                      currency,
                                    )} change`
                                  : ""}
                              </p>
                            </div>
                            <p
                              className={cn(
                                "text-xs font-semibold tabular-nums",
                                share.state.balanceMinor > 0
                                  ? "text-danger"
                                  : "text-success",
                              )}
                            >
                              {share.state.balanceMinor > 0
                                ? `${formatMoney(share.state.balanceMinor, currency)} left`
                                : "Settled"}
                            </p>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </Card>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
