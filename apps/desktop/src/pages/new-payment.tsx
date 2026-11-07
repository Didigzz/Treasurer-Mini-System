import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { ArrowLeft, Banknote, CheckCircle2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { previewPayment } from "@my-better-t-app/api/treasurer/calculations";
import {
  formatDateOnly,
  formatDateTime,
  formatMoney,
  formatPeriod,
  minorToInputValue,
  parseMoneyToMinor,
  todayIsoDate,
} from "@my-better-t-app/api/treasurer/money";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  Field,
  LoadingBlock,
  PageHeader,
  SectionHeader,
  ShareStatusBadge,
  SummaryCard,
  cn,
} from "@/components/ui";
import { useAppSettings, usePaymentContext, usePaymentMutations } from "@/queries";

function toDateTimeLocal(epochMs: number): string {
  const date = new Date(epochMs);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}

function fromDateTimeLocal(value: string): number | null {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed.getTime();
}

export function NewPaymentPage() {
  const { shareId } = useSearch({ from: "/payments/new" });
  const navigate = useNavigate();
  const context = usePaymentContext(shareId);
  const settings = useAppSettings();
  const { record } = usePaymentMutations();
  const currency = settings.data?.currencyCode ?? "PHP";

  const [amountInput, setAmountInput] = useState("");
  const [paidAt, setPaidAt] = useState(() => Date.now());
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<{
    amountMinor: number;
    changeMinor: number;
    balanceMinor: number;
    boarder: string;
    billId: string;
  } | null>(null);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current || !context.data) return;
    if (context.data.share.state.balanceMinor > 0) {
      setAmountInput(minorToInputValue(context.data.share.state.balanceMinor));
    }
    initialized.current = true;
  }, [context.data]);

  if (!shareId) {
    return (
      <EmptyState
        title="Choose a boarder first"
        description="Open a bill or boarder and use Record payment."
        action={
          <Link to="/">
            <Button size="sm">Go to dashboard</Button>
          </Link>
        }
      />
    );
  }

  if (context.isLoading) return <LoadingBlock label="Loading payment details…" />;

  if (context.isError || !context.data) {
    return (
      <ErrorBlock
        message="This share could not be loaded."
        onRetry={() => void context.refetch()}
      />
    );
  }

  const { share, bill, boarderName } = context.data;
  const amountMinor = parseMoneyToMinor(amountInput);
  const preview =
    amountMinor !== null && amountMinor > 0
      ? previewPayment(share.amountDueMinor, share.state.payments, amountMinor, paidAt)
      : null;
  const alreadyPaid = share.state.balanceMinor === 0;

  async function handleSave() {
    if (amountMinor === null || amountMinor <= 0) {
      setError("Enter a payment amount greater than zero.");
      return;
    }
    if (note.length > 500) {
      setError("Keep the note to 500 characters or fewer.");
      return;
    }

    setError(null);
    try {
      await record.mutateAsync({
        billShareId: share.id,
        amountMinor,
        paidAt,
        note: note.trim() || null,
      });
      setSaved({
        amountMinor,
        changeMinor: preview?.changeFromThisPaymentMinor ?? 0,
        balanceMinor: preview?.balanceAfterMinor ?? 0,
        boarder: boarderName,
        billId: bill.id,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The payment could not be saved.");
    }
  }

  if (saved) {
    return (
      <div className="mx-auto max-w-lg pt-10">
        <Card className="p-8 text-center">
          <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-success-soft text-success">
            <CheckCircle2 size={30} />
          </div>
          <h1 className="mt-4 text-2xl font-bold text-ink">Payment recorded</h1>
          <p className="mt-1 text-sm text-muted">
            {saved.boarder} · {bill.billTypeNameSnapshot} ·{" "}
            {formatPeriod(bill.periodYear, bill.periodMonth)}
          </p>
          <p className="mt-5 text-3xl font-bold tabular-nums text-ink">
            {formatMoney(saved.amountMinor, currency)}
          </p>
          {saved.balanceMinor > 0 ? (
            <p className="mt-2 text-sm font-semibold text-danger">
              {formatMoney(saved.balanceMinor, currency)} still owed
            </p>
          ) : saved.changeMinor > 0 ? (
            <p className="mt-2 text-sm font-semibold text-warning">
              Paid · {formatMoney(saved.changeMinor, currency)} change to return
            </p>
          ) : (
            <p className="mt-2 text-sm font-semibold text-success">Paid in full</p>
          )}
          <p className="mt-2 text-xs text-muted">{formatDateTime(paidAt)}</p>

          <div className="mt-6 flex justify-center gap-2">
            <Button
              onClick={() =>
                void navigate({ to: "/bills/$id", params: { id: saved.billId } })
              }
            >
              Back to bill
            </Button>
            <Link to="/boarders/$id" params={{ id: share.boarderId }}>
              <Button variant="secondary">Open boarder</Button>
            </Link>
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <Link
        to="/bills/$id"
        params={{ id: bill.id }}
        className="mb-4 inline-flex items-center gap-1.5 text-xs font-semibold text-muted hover:text-ink"
      >
        <ArrowLeft size={14} />
        Back to bill
      </Link>

      <PageHeader
        eyebrow="Record payment"
        title={boarderName}
        description={`${bill.billTypeNameSnapshot} · ${formatPeriod(bill.periodYear, bill.periodMonth)}`}
        actions={<ShareStatusBadge status={share.state.status} />}
      />

      {error ? (
        <div className="mt-4">
          <ErrorBlock message={error} />
        </div>
      ) : null}

      <div className="mt-5 grid items-start gap-5 lg:grid-cols-[1fr_320px]">
        <div className="space-y-5">
          <div className="grid grid-cols-2 gap-3">
            <SummaryCard label="Amount due" value={formatMoney(share.amountDueMinor, currency)} />
            <SummaryCard
              label="Balance"
              value={formatMoney(share.state.balanceMinor, currency)}
              tone={share.state.balanceMinor > 0 ? "danger" : "success"}
            />
          </div>

          {share.state.changeOutstandingMinor > 0 ? (
            <div className="rounded-xl border border-warning/30 bg-warning-soft px-4 py-3 text-xs font-medium text-warning">
              {formatMoney(share.state.changeOutstandingMinor, currency)} change is still outstanding
              from an earlier payment. It is not netted automatically.
            </div>
          ) : null}

          <Card className="p-5">
            <SectionHeader title="Payment" />
            <div className="grid gap-4 sm:grid-cols-2">
              <Field
                label={`Amount received (${currency})`}
                hint={alreadyPaid ? "This share is already paid." : "Prefilled with the balance."}
                required
              >
                <input
                  className="field tabular-nums"
                  autoFocus={!alreadyPaid}
                  inputMode="decimal"
                  placeholder="0.00"
                  maxLength={16}
                  value={amountInput}
                  onChange={(event) => {
                    setAmountInput(event.target.value);
                    if (error) setError(null);
                  }}
                />
              </Field>

              <Field label="Payment date and time" required>
                <input
                  type="datetime-local"
                  className="field"
                  value={toDateTimeLocal(paidAt)}
                  max={`${todayIsoDate()}T23:59`}
                  onChange={(event) => {
                    const next = fromDateTimeLocal(event.target.value);
                    if (next !== null) setPaidAt(next);
                  }}
                />
              </Field>
            </div>
            <p className="mt-1 text-[11px] text-muted">
              Chosen date: {formatDateOnly(todayIsoDate(new Date(paidAt)))}
            </p>

            <Field label="Note (optional)" hint="Up to 500 characters." className="mt-4">
              <textarea
                className="field-area"
                rows={3}
                maxLength={500}
                placeholder="e.g. Cash handed over at the gate"
                value={note}
                onChange={(event) => setNote(event.target.value)}
              />
            </Field>
          </Card>
        </div>

        <Card className="sticky top-6 p-5">
          <p className="label">After saving</p>
          {preview ? (
            <>
              <p
                className={cn(
                  "mt-2 text-lg font-bold",
                  preview.balanceAfterMinor > 0
                    ? "text-danger"
                    : preview.changeFromThisPaymentMinor > 0
                      ? "text-warning"
                      : "text-success",
                )}
              >
                {preview.balanceAfterMinor > 0
                  ? `${formatMoney(preview.balanceAfterMinor, currency)} still owed`
                  : preview.changeFromThisPaymentMinor > 0
                    ? `Paid · ${formatMoney(preview.changeFromThisPaymentMinor, currency)} change`
                    : "Paid in full"}
              </p>
              <p className="mt-1 text-xs text-muted">
                Total paid after this entry: {formatMoney(preview.paidAfterMinor, currency)}
                {preview.changeOutstandingAfterMinor > 0
                  ? ` · change outstanding: ${formatMoney(
                      preview.changeOutstandingAfterMinor,
                      currency,
                    )}`
                  : ""}
              </p>
            </>
          ) : (
            <p className="mt-2 text-sm text-muted">
              Enter an amount to see the remaining balance or change.
            </p>
          )}

          <Button
            className="mt-5 w-full"
            disabled={amountMinor === null || amountMinor <= 0 || record.isPending}
            onClick={() => void handleSave()}
          >
            <Banknote size={16} />
            {record.isPending ? "Saving…" : "Save payment"}
          </Button>

          {alreadyPaid ? (
            <Badge tone="warning" className="mt-3">
              New amounts will create change to return
            </Badge>
          ) : null}
        </Card>
      </div>
    </div>
  );
}
