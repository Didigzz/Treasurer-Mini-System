import { Link, useParams } from "@tanstack/react-router";
import { ArrowLeft, Archive, Banknote, Receipt } from "lucide-react";

import {
  formatDateOnly,
  formatDateTime,
  formatMoney,
  formatPeriod,
} from "@my-better-t-app/api/treasurer/money";
import {
  Badge,
  BillStatusBadge,
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  LoadingBlock,
  PageHeader,
  SectionHeader,
  ShareStatusBadge,
  SummaryCard,
} from "@/components/ui";
import { useAppSettings, useBoarderLedger } from "@/queries";

export function BoarderDetailPage() {
  const { id } = useParams({ from: "/boarders/$id" });
  const ledger = useBoarderLedger(id);
  const settings = useAppSettings();
  const currency = settings.data?.currencyCode ?? "PHP";

  if (ledger.isLoading) return <LoadingBlock label="Loading boarder…" />;

  if (ledger.isError || !ledger.data) {
    return (
      <ErrorBlock
        message="This boarder could not be loaded."
        onRetry={() => void ledger.refetch()}
      />
    );
  }

  const { boarder, items, payments, summary } = ledger.data;

  return (
    <div>
      <Link
        to="/boarders"
        className="mb-4 inline-flex items-center gap-1.5 text-xs font-semibold text-muted hover:text-ink"
      >
        <ArrowLeft size={14} />
        All boarders
      </Link>

      <PageHeader
        eyebrow="Boarder ledger"
        title={boarder.name}
        description="Every share and payment recorded for this boarder, across all periods."
        actions={
          boarder.status === "archived" ? (
            <Badge>
              <Archive size={12} />
              Archived
            </Badge>
          ) : null
        }
      />

      <div className="mt-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <SummaryCard label="Expected" value={formatMoney(summary.expectedMinor, currency)} />
        <SummaryCard
          label="Received"
          value={formatMoney(summary.appliedMinor, currency)}
          tone="success"
        />
        <SummaryCard
          label="Still owed"
          value={formatMoney(summary.outstandingMinor, currency)}
          tone={summary.outstandingMinor > 0 ? "danger" : "success"}
        />
        <SummaryCard
          label="Change to return"
          value={formatMoney(summary.changeOutstandingMinor, currency)}
          tone={summary.changeOutstandingMinor > 0 ? "warning" : "neutral"}
        />
      </div>

      <SectionHeader
        title="Shares"
        subtitle="Newest billing period first. Paid means the share is covered; change is tracked separately."
      />

      {items.length === 0 ? (
        <EmptyState
          title="No bills yet"
          description="This boarder has not been included in a bill."
          action={
            <Link to="/bills/new" search={{ year: undefined, month: undefined }}>
              <Button size="sm">
                <Receipt size={15} />
                Create a bill
              </Button>
            </Link>
          }
        />
      ) : (
        <Card className="divide-y divide-line">
          {items.map(({ bill, share, state }) => (
            <div key={share.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    to="/bills/$id"
                    params={{ id: bill.id }}
                    className="text-sm font-semibold text-ink hover:text-accent"
                  >
                    {bill.billTypeNameSnapshot}
                  </Link>
                  <BillStatusBadge
                    status={bill.archivedAt !== null ? "archived" : state.balanceMinor === 0 ? "settled" : "open"}
                  />
                  <ShareStatusBadge status={state.status} />
                </div>
                <p className="mt-0.5 text-xs text-muted">
                  {formatPeriod(bill.periodYear, bill.periodMonth)} ·{" "}
                  {formatMoney(share.amountDueMinor, currency)} due
                  {bill.dueDate ? ` · due ${formatDateOnly(bill.dueDate)}` : ""}
                </p>
              </div>

              <div className="text-right">
                <p className="text-sm font-semibold tabular-nums text-ink">
                  {formatMoney(state.appliedMinor, currency)} paid
                </p>
                <p
                  className={
                    state.balanceMinor > 0
                      ? "text-xs font-medium text-danger"
                      : "text-xs font-medium text-success"
                  }
                >
                  {state.balanceMinor > 0
                    ? `${formatMoney(state.balanceMinor, currency)} left`
                    : "Settled"}
                  {state.changeOutstandingMinor > 0
                    ? ` · ${formatMoney(state.changeOutstandingMinor, currency)} change`
                    : ""}
                </p>
              </div>

              {boarder.status === "active" && bill.archivedAt === null ? (
                <Link to="/payments/new" search={{ shareId: share.id }}>
                  <Button size="sm" variant="secondary">
                    <Banknote size={14} />
                    Record payment
                  </Button>
                </Link>
              ) : null}
            </div>
          ))}
        </Card>
      )}

      <SectionHeader title="Payment history" subtitle="Newest first. Voided entries stay visible." />

      {payments.length === 0 ? (
        <EmptyState title="No payments recorded" />
      ) : (
        <Card className="divide-y divide-line">
          {payments.map(({ payment, bill }) => (
            <div key={payment.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <p className="text-sm font-semibold tabular-nums text-ink">
                    {formatMoney(payment.amountMinor, currency)}
                  </p>
                  {payment.voidedAt !== null ? (
                    <Badge tone="danger">Voided</Badge>
                  ) : payment.computedChangeMinor > 0 ? (
                    <Badge tone={payment.changeReturnedAt ? "success" : "warning"}>
                      {payment.changeReturnedAt ? "Change returned" : "Change due"}
                    </Badge>
                  ) : null}
                </div>
                <p className="mt-0.5 text-xs text-muted">
                  {bill.billTypeNameSnapshot} · {formatPeriod(bill.periodYear, bill.periodMonth)} ·{" "}
                  {formatDateTime(payment.paidAt)}
                </p>
                {payment.note ? <p className="mt-1 text-xs text-muted">{payment.note}</p> : null}
                {payment.voidedAt !== null && payment.voidReason ? (
                  <p className="mt-1 text-xs text-danger">Voided: {payment.voidReason}</p>
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
              <Link to="/bills/$id" params={{ id: bill.id }}>
                <Button size="sm" variant="ghost">
                  Open bill
                </Button>
              </Link>
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
