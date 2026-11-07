import { useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import {
  ArrowLeft,
  Archive,
  Banknote,
  CalendarDays,
  Pencil,
  RotateCcw,
  RotateCcwSquare,
} from "lucide-react";
import { useMemo, useState } from "react";

import { matchesShareFilter } from "@my-better-t-app/api/treasurer/calculations";
import {
  daysOverdue,
  formatDateOnly,
  formatDateTime,
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
  Field,
  FilterChips,
  LoadingBlock,
  Modal,
  PageHeader,
  SectionHeader,
  ShareStatusBadge,
  SummaryCard,
  cn,
} from "@/components/ui";
import {
  useAppSettings,
  useBillDetail,
  useBillMutations,
  usePaymentMutations,
} from "@/queries";

export function BillDetailPage() {
  const { id } = useParams({ from: "/bills/$id" });
  const detail = useBillDetail(id);
  const settings = useAppSettings();
  const { setArchived, updateMeta } = useBillMutations();
  const { voidPayment, markChangeReturned } = usePaymentMutations();
  const queryClient = useQueryClient();

  const [filter, setFilter] = useState<ShareFilter>("all");
  const [editing, setEditing] = useState(false);
  const [dueDate, setDueDate] = useState("");
  const [note, setNote] = useState("");
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [voidTarget, setVoidTarget] = useState<{ id: string; boarder: string } | null>(null);
  const [voidReason, setVoidReason] = useState("");
  const [changeTarget, setChangeTarget] = useState<{ id: string; boarder: string } | null>(null);
  const [changeNote, setChangeNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const currency = settings.data?.currencyCode ?? "PHP";
  const data = detail.data;
  const bill = data?.bill;
  const isArchived = bill?.archivedAt !== null && bill?.archivedAt !== undefined;

  const filteredShares = useMemo(
    () => (data?.shares ?? []).filter((share) => matchesShareFilter(share, filter)),
    [data, filter],
  );

  const counts = useMemo(() => {
    const shares = data?.shares ?? [];
    return {
      all: shares.length,
      unpaid: shares.filter((share) => matchesShareFilter(share, "unpaid")).length,
      paid: shares.filter((share) => matchesShareFilter(share, "paid")).length,
      change: shares.filter((share) => matchesShareFilter(share, "change")).length,
    };
  }, [data]);

  const paymentRows = useMemo(() => {
    if (!data) return [];
    const names = new Map(
      data.shares.map((share) => [
        share.id,
        share.currentBoarderName ?? share.boarderNameSnapshot,
      ]),
    );
    return data.shares
      .flatMap((share) => share.state.payments)
      .sort((a, b) => b.paidAt - a.paidAt)
      .map((payment) => ({
        payment,
        boarder: names.get(payment.billShareId) ?? "Boarder",
      }));
  }, [data]);

  function startEditing() {
    if (!bill) return;
    setDueDate(bill.dueDate ?? "");
    setNote(bill.note ?? "");
    setEditing(true);
    setError(null);
    setNotice(null);
  }

  async function saveMeta() {
    if (!bill) return;
    setError(null);
    setNotice(null);
    try {
      await updateMeta.mutateAsync({
        id: bill.id,
        dueDate: dueDate || null,
        note: note.trim() || null,
      });
      setEditing(false);
      setNotice("Due date and note updated.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill could not be updated.");
    }
  }

  async function handleArchiveToggle() {
    if (!bill) return;
    setError(null);
    setNotice(null);
    try {
      await setArchived.mutateAsync({ id: bill.id, isArchived: !isArchived });
      setArchiveOpen(false);
      setNotice(isArchived ? "Bill restored." : "Bill archived.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill could not be updated.");
      setArchiveOpen(false);
    }
  }

  async function handleVoid() {
    if (!voidTarget) return;
    const reason = voidReason.trim();
    if (!reason) {
      setError("Enter a reason for voiding this payment.");
      return;
    }
    setError(null);
    try {
      await voidPayment.mutateAsync({ paymentId: voidTarget.id, reason });
      setVoidTarget(null);
      setVoidReason("");
      setNotice("Payment voided. Totals were recalculated.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The payment could not be voided.");
    }
  }

  async function handleMarkChange() {
    if (!changeTarget) return;
    setError(null);
    try {
      await markChangeReturned.mutateAsync({
        paymentId: changeTarget.id,
        returnedAt: Date.now(),
        note: changeNote.trim() || null,
      });
      setChangeTarget(null);
      setChangeNote("");
      setNotice("Change marked as returned.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Change could not be marked returned.");
    }
  }

  if (detail.isLoading) return <LoadingBlock label="Loading bill…" />;

  if (detail.isError || !data || !bill) {
    return (
      <ErrorBlock message="This bill could not be loaded." onRetry={() => void detail.refetch()} />
    );
  }

  const overdue = isOverdue(bill.dueDate, data.summary.outstandingMinor > 0);
  const renamed =
    data.currentTypeName.localeCompare(bill.billTypeNameSnapshot, undefined, {
      sensitivity: "base",
    }) !== 0;

  return (
    <div>
      <Link
        to="/"
        className="mb-4 inline-flex items-center gap-1.5 text-xs font-semibold text-muted hover:text-ink"
      >
        <ArrowLeft size={14} />
        Dashboard
      </Link>

      <PageHeader
        eyebrow={formatPeriod(bill.periodYear, bill.periodMonth)}
        title={bill.billTypeNameSnapshot}
        description={
          renamed
            ? `This type is now called ${data.currentTypeName}.`
            : bill.note ?? undefined
        }
        actions={
          <>
            <BillStatusBadge status={data.summary.status} />
            <Button variant="secondary" onClick={startEditing}>
              <Pencil size={15} />
              {data.isLocked ? "Edit details" : "Edit bill"}
            </Button>
            <Button variant={isArchived ? "secondary" : "danger"} onClick={() => setArchiveOpen(true)}>
              {isArchived ? <RotateCcw size={15} /> : <Archive size={15} />}
              {isArchived ? "Unarchive" : "Archive"}
            </Button>
          </>
        }
      />

      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted">
        <span>Total {formatMoney(bill.totalMinor, currency)}</span>
        {bill.dueDate ? (
          <span className="flex items-center gap-1">
            <CalendarDays size={12} />
            Due {formatDateOnly(bill.dueDate)}
          </span>
        ) : null}
        {overdue && bill.dueDate ? (
          <Badge tone="danger">Overdue {daysOverdue(bill.dueDate)}d</Badge>
        ) : null}
        {data.summary.needsReview ? <Badge tone="warning">Review change</Badge> : null}
        {isArchived ? (
          <Badge>
            <Archive size={11} />
            Archived
          </Badge>
        ) : null}
      </div>

      {error ? (
        <div className="mt-4">
          <ErrorBlock message={error} />
        </div>
      ) : null}
      {notice ? (
        <div className="mt-4 rounded-xl border border-success/20 bg-success-soft px-3 py-2 text-sm font-medium text-success">
          {notice}
        </div>
      ) : null}

      {isArchived ? (
        <div className="mt-4 rounded-xl border border-line bg-panel px-4 py-3 text-sm text-muted">
          This bill is archived. Unarchive it to record or correct payments.
        </div>
      ) : null}

      {data.summary.changeOutstandingMinor > 0 ? (
        <div className="mt-4 rounded-xl border border-warning/30 bg-warning-soft px-4 py-3">
          <p className="text-sm font-semibold text-warning">
            {formatMoney(data.summary.changeOutstandingMinor, currency)} change still to return
          </p>
          <p className="mt-0.5 text-xs text-muted">
            The bill can be settled while change is outstanding. Return it from the payment
            history.
          </p>
        </div>
      ) : null}

      <div className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <SummaryCard label="Expected" value={formatMoney(data.summary.expectedMinor, currency)} />
        <SummaryCard
          label="Received"
          value={formatMoney(data.summary.appliedMinor, currency)}
          tone="success"
        />
        <SummaryCard
          label="Still owed"
          value={formatMoney(data.summary.outstandingMinor, currency)}
          tone={data.summary.outstandingMinor > 0 ? "danger" : "success"}
        />
        <SummaryCard
          label="Change"
          value={formatMoney(data.summary.changeOutstandingMinor, currency)}
          tone={data.summary.changeOutstandingMinor > 0 ? "warning" : "neutral"}
        />
      </div>

      <SectionHeader
        title={`Boarders (${data.shares.length})`}
        subtitle="Paid means the share is covered; change is tracked separately."
      />

      <FilterChips value={filter} onChange={setFilter} counts={counts} />

      <div className="mt-3">
        {filteredShares.length === 0 ? (
          <EmptyState
            title="No boarders match this filter"
            description="Try another filter."
            action={
              <Button size="sm" variant="secondary" onClick={() => setFilter("all")}>
                Show all
              </Button>
            }
          />
        ) : (
          <Card className="divide-y divide-line">
            {filteredShares.map((share) => (
              <div key={share.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <Link
                      to="/boarders/$id"
                      params={{ id: share.boarderId }}
                      className="text-sm font-semibold text-ink hover:text-accent"
                    >
                      {share.currentBoarderName ?? share.boarderNameSnapshot}
                    </Link>
                    <ShareStatusBadge status={share.state.status} />
                    {share.boarderArchived ? <Badge>Archived</Badge> : null}
                    {share.amountDueMinor === 0 ? <Badge>Nothing due</Badge> : null}
                    {share.state.needsReview ? <Badge tone="warning">Review change</Badge> : null}
                  </div>
                  <p className="mt-0.5 text-xs text-muted">
                    {formatMoney(share.amountDueMinor, currency)} due ·{" "}
                    {formatMoney(share.state.paidMinor, currency)} paid
                    {share.state.changeOutstandingMinor > 0
                      ? ` · ${formatMoney(share.state.changeOutstandingMinor, currency)} change`
                      : ""}
                  </p>
                </div>

                <div className="text-right">
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

                {!isArchived && share.amountDueMinor > 0 ? (
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
      </div>

      <SectionHeader
        title="Payment history"
        subtitle="Newest first. Voided entries stay visible."
      />

      {paymentRows.length === 0 ? (
        <EmptyState title="No payments recorded yet" />
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

                {!isArchived && payment.voidedAt === null ? (
                  <div className="flex gap-1">
                    {payment.computedChangeMinor > 0 && payment.changeReturnedAt === null ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => {
                          setChangeTarget({ id: payment.id, boarder });
                          setChangeNote("");
                        }}
                      >
                        <RotateCcwSquare size={14} />
                        Return change
                      </Button>
                    ) : null}
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setVoidTarget({ id: payment.id, boarder });
                        setVoidReason("");
                      }}
                    >
                      Void
                    </Button>
                  </div>
                ) : null}
              </div>
            </div>
          ))}
        </Card>
      )}

      <Modal
        open={editing}
        onClose={() => setEditing(false)}
        title="Edit bill details"
        description={
          data.isLocked
            ? "Payments have been recorded, so the type, period, total, and shares are locked. You can still update the due date and note."
            : "Update the due date and note. To change the type, total, or split, archive this bill and create a new one."
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setEditing(false)}>
              Cancel
            </Button>
            <Button onClick={() => void saveMeta()} disabled={updateMeta.isPending}>
              {updateMeta.isPending ? "Saving…" : "Save details"}
            </Button>
          </>
        }
      >
        <div className="space-y-4">
          <Field label="Due date (optional)">
            <input
              type="date"
              className="field"
              value={dueDate}
              onChange={(event) => setDueDate(event.target.value)}
            />
          </Field>
          <Field label="Note (optional)" hint="Up to 500 characters.">
            <textarea
              className="field-area"
              rows={3}
              maxLength={500}
              value={note}
              onChange={(event) => setNote(event.target.value)}
            />
          </Field>
        </div>
      </Modal>

      <Modal
        open={archiveOpen}
        onClose={() => setArchiveOpen(false)}
        title={isArchived ? "Unarchive this bill?" : "Archive this bill?"}
        description={
          isArchived
            ? "It will return to the default period list and totals. All records are unchanged."
            : data.summary.outstandingMinor > 0
              ? `This bill still has ${formatMoney(data.summary.outstandingMinor, currency)} outstanding. Archiving hides it from default totals but keeps the debt in boarder history.`
              : "It will disappear from the default period list and totals. All records stay intact and you can unarchive it later."
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setArchiveOpen(false)}>
              Cancel
            </Button>
            <Button
              variant={isArchived ? "primary" : "danger"}
              onClick={() => void handleArchiveToggle()}
              disabled={setArchived.isPending}
            >
              {isArchived ? "Unarchive" : "Archive"}
            </Button>
          </>
        }
      />

      <Modal
        open={changeTarget !== null}
        onClose={() => setChangeTarget(null)}
        title="Mark change returned"
        description={
          changeTarget
            ? `Record that ${changeTarget.boarder}'s change was handed back. This does not change the payment amount.`
            : undefined
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setChangeTarget(null)}>
              Cancel
            </Button>
            <Button
              onClick={() => void handleMarkChange()}
              disabled={markChangeReturned.isPending}
            >
              {markChangeReturned.isPending ? "Saving…" : "Mark returned"}
            </Button>
          </>
        }
      >
        <Field label="Note (optional)" hint="e.g. Returned in cash">
          <input
            className="field"
            maxLength={200}
            value={changeNote}
            onChange={(event) => setChangeNote(event.target.value)}
          />
        </Field>
      </Modal>

      <Modal
        open={voidTarget !== null}
        onClose={() => setVoidTarget(null)}
        title="Void this payment?"
        description={
          voidTarget
            ? `The ${voidTarget.boarder} payment stays in history with your reason. It is excluded from totals. Record a new payment if needed.`
            : undefined
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setVoidTarget(null)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={() => void handleVoid()} disabled={voidPayment.isPending}>
              {voidPayment.isPending ? "Voiding…" : "Void payment"}
            </Button>
          </>
        }
      >
        <Field label="Reason" required>
          <input
            className="field"
            maxLength={200}
            value={voidReason}
            placeholder="e.g. Entered the wrong amount"
            onChange={(event) => setVoidReason(event.target.value)}
          />
        </Field>
      </Modal>

      <div className="mt-6 flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            void queryClient.invalidateQueries();
          }}
        >
          Refresh
        </Button>
      </div>
    </div>
  );
}
