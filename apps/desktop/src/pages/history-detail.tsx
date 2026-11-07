import { Link, useParams } from "@tanstack/react-router";
import { AlertCircle, ArrowLeft, Banknote, CheckCircle2, Receipt } from "lucide-react";
import { useMemo } from "react";

import {
  daysOverdue,
  formatDateOnly,
  formatDateTime,
  formatMoney,
  formatPeriod,
  isOverdue,
} from "@my-better-t-app/api/treasurer/money";
import {
  Badge,
  BillStatusBadge,
  Card,
  EmptyState,
  ErrorBlock,
  LoadingBlock,
  PageHeader,
  ShareStatusBadge,
  SummaryCard,
  cn,
} from "@/components/ui";
import { useAppSettings, useBillDetail } from "@/queries";

export function HistoryBillPage() {
  const { id } = useParams({ from: "/history/$id" });
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
        boarder: names.get(payment.billShareId) ?? "Tenant",
      }));
  }, [data]);

  if (detail.isLoading) return <LoadingBlock label="Loading bill…" />;

  if (detail.isError || !data || !bill) {
    return (
      <ErrorBlock
        message="This bill could not be loaded."
        onRetry={() => void detail.refetch()}
      />
    );
  }

  const overdue = isOverdue(bill.dueDate, data.summary.outstandingMinor > 0);
  const renamed =
    data.currentTypeName.localeCompare(bill.billTypeNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;
  const isArchived = bill.archivedAt !== null;

  return (
    <div>
      <Link
        to="/history"
        className="mb-4 inline-flex items-center gap-1.5 text-xs font-semibold text-muted hover:text-ink"
      >
        <ArrowLeft size={14} />
        History
      </Link>

      <PageHeader
        eyebrow={formatPeriod(bill.periodYear, bill.periodMonth)}
        title={bill.billTypeNameSnapshot}
        description={
          renamed ? `This type is now called ${data.currentTypeName}.` : bill.note ?? undefined
        }
        actions={
          <>
            <BillStatusBadge status={data.summary.status} />
            <Badge>Read-only</Badge>
          </>
        }
      />

      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted">
        <span>Total {formatMoney(bill.totalMinor, currency)}</span>
        {bill.dueDate ? <span>Due {formatDateOnly(bill.dueDate)}</span> : null}
        {overdue && bill.dueDate ? (
          <Badge tone="danger">Overdue {daysOverdue(bill.dueDate)}d</Badge>
        ) : null}
        {data.summary.needsReview ? <Badge tone="warning">Review change</Badge> : null}
        {isArchived ? <Badge>Archived</Badge> : null}
      </div>

      {data.summary.changeOutstandingMinor > 0 ? (
        <div className="mt-4 rounded-xl border border-warning/30 bg-warning-soft px-4 py-3">
          <p className="text-sm font-semibold text-warning">
            {formatMoney(data.summary.changeOutstandingMinor, currency)} change still to return
          </p>
        </div>
      ) : null}

      <div className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <SummaryCard
          label="Expected"
          value={formatMoney(data.summary.expectedMinor, currency)}
          icon={<Receipt size={16} />}
        />
        <SummaryCard
          label="Received"
          value={formatMoney(data.summary.appliedMinor, currency)}
          tone="success"
          icon={<CheckCircle2 size={16} />}
        />
        <SummaryCard
          label="Still owed"
          value={formatMoney(data.summary.outstandingMinor, currency)}
          tone={data.summary.outstandingMinor > 0 ? "danger" : "success"}
          icon={<AlertCircle size={16} />}
        />
        <SummaryCard
          label="Change to return"
          value={formatMoney(data.summary.changeOutstandingMinor, currency)}
          tone={data.summary.changeOutstandingMinor > 0 ? "warning" : "neutral"}
          icon={<Banknote size={16} />}
        />
      </div>

      <div className="mt-7 mb-3">
        <h2 className="text-sm font-bold text-ink">Tenants ({data.shares.length})</h2>
        <p className="mt-0.5 text-xs text-muted">
          Snapshot names from the time the bill was created.
        </p>
      </div>

      {data.shares.length === 0 ? (
        <EmptyState title="No tenants on this bill" />
      ) : (
        <Card className="divide-y divide-line">
          {data.shares.map((share) => (
            <div key={share.id} className="px-4 py-3">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <ShareStatusBadge status={share.state.status} />
                    <p className="truncate text-sm font-semibold text-ink">
                      {share.boarderNameSnapshot}
                    </p>
                    {share.boarderArchived ? <Badge>Archived</Badge> : null}
                    {share.amountDueMinor === 0 ? <Badge>Nothing due</Badge> : null}
                    {share.state.needsReview ? (
                      <Badge tone="warning">Review change</Badge>
                    ) : null}
                  </div>
                  <p className="mt-0.5 text-xs text-muted">
                    {formatMoney(share.amountDueMinor, currency)} due ·{" "}
                    {formatMoney(share.state.paidMinor, currency)} paid
                    {share.state.changeOutstandingMinor > 0
                      ? ` · ${formatMoney(share.state.changeOutstandingMinor, currency)} change`
                      : ""}
                  </p>
                </div>
                <p
                  className={cn(
                    "text-sm font-semibold tabular-nums",
                    share.state.balanceMinor > 0 ? "text-danger" : "text-success",
                  )}
                >
                  {share.state.balanceMinor > 0
                    ? `${formatMoney(share.state.balanceMinor, currency)} left`
                    : "Settled"}
                </p>
              </div>
            </div>
          ))}
        </Card>
      )}

      <div className="mt-7 mb-3">
        <h2 className="text-sm font-bold text-ink">Payment history</h2>
        <p className="mt-0.5 text-xs text-muted">
          Newest first. Voided entries stay visible.
        </p>
      </div>

      {paymentRows.length === 0 ? (
        <EmptyState title="No payments recorded" />
      ) : (
        <Card className="divide-y divide-line">
          {paymentRows.map(({ payment, boarder }) => (
            <div key={payment.id} className="px-4 py-3">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="text-sm font-semibold text-ink">{boarder}</p>
                    <p className="text-sm font-semibold tabular-nums text-ink">
                      {formatMoney(payment.amountMinor, currency)}
                    </p>
                    {payment.voidedAt !== null ? (
                      <Badge tone="danger">Voided</Badge>
                    ) : payment.computedChangeMinor > 0 ? (
                      <Badge tone={payment.changeReturnedAt ? "success" : "warning"}>
                        {payment.changeReturnedAt ? "Change returned" : "Change to return"}
                      </Badge>
                    ) : null}
                  </div>
                  <p className="mt-0.5 text-xs text-muted">
                    {formatDateTime(payment.paidAt)}
                    {payment.note ? ` · ${payment.note}` : ""}
                  </p>
                  {payment.voidedAt !== null ? (
                    <p className="mt-1 text-xs text-danger">
                      Voided {formatDateTime(payment.voidedAt)}
                      {payment.voidReason ? ` · ${payment.voidReason}` : ""}
                    </p>
                  ) : null}
                  {payment.computedChangeMinor > 0 ? (
                    <p className="mt-1 text-xs text-warning">
                      {formatMoney(payment.computedChangeMinor, currency)} change
                      {payment.changeReturnedAt
                        ? ` returned ${formatDateTime(payment.changeReturnedAt)}`
                        : " still to return"}
                      {payment.changeReturnNote ? ` · ${payment.changeReturnNote}` : ""}
                    </p>
                  ) : null}
                </div>
              </div>
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
