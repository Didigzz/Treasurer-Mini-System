import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { CalendarDays, Plus, Users } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { equalSplit, splitWithOverrides } from "@my-better-t-app/api/treasurer/calculations";
import {
  currentPeriod,
  formatMoney,
  minorToInputValue,
  parseAmountInput,
  parseMoneyToMinor,
} from "@my-better-t-app/api/treasurer/money";
import type { SplitResult } from "@my-better-t-app/api/treasurer/types";
import {
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  Field,
  LoadingBlock,
  Modal,
  MonthSwitcher,
  PageHeader,
  SectionHeader,
  SuccessNote,
  cn,
} from "@/components/ui";
import { useActiveBoarders, useAppSettings, useBillMutations, useBillTypes } from "@/queries";
import { useTRPC } from "@/trpc";

type SplitMethod = "equal" | "custom";

const WHOLE_PESO_MESSAGE = "Whole pesos only. Turn on centavos in Settings to use decimals.";

export function NewBillPage() {
  const search = useSearch({ from: "/bills/new" });
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const trpc = useTRPC();

  const types = useBillTypes();
  const boarders = useActiveBoarders();
  const settings = useAppSettings();
  const { create } = useBillMutations();
  const currency = settings.data?.currencyCode ?? "PHP";

  const [period, setPeriod] = useState(() => {
    const fallback = currentPeriod();
    const year = search.year ?? fallback.year;
    const month = search.month ?? fallback.month;
    return {
      year: year >= 2000 && year <= 2100 ? year : fallback.year,
      month: month >= 1 && month <= 12 ? month : fallback.month,
    };
  });
  const [billTypeId, setBillTypeId] = useState("");
  const [totalInput, setTotalInput] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [note, setNote] = useState("");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [method, setMethod] = useState<SplitMethod>("equal");
  const [customInputs, setCustomInputs] = useState<Record<string, string>>({});
  const [pinnedIds, setPinnedIds] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [duplicates, setDuplicates] = useState<Array<{ id: string; totalMinor: number }> | null>(
    null,
  );
  const [checkingDuplicates, setCheckingDuplicates] = useState(false);
  const initialized = useRef(false);

  const activeTypes = useMemo(
    () => (types.data ?? []).filter((type) => !type.isArchived),
    [types.data],
  );

  useEffect(() => {
    if (billTypeId || activeTypes.length === 0) return;
    setBillTypeId(activeTypes[0]!.id);
  }, [activeTypes, billTypeId]);

  useEffect(() => {
    if (initialized.current || boarders.isLoading) return;
    setSelectedIds((boarders.data ?? []).map((boarder) => boarder.id));
    initialized.current = true;
  }, [boarders.data, boarders.isLoading]);

  const selectedBoarders = selectedIds
    .map((id) => (boarders.data ?? []).find((boarder) => boarder.id === id))
    .filter((boarder): boarder is NonNullable<typeof boarder> => Boolean(boarder));

  const allowCentavos = settings.data?.allowCentavos ?? true;

  const parsedTotal = parseAmountInput(totalInput, { allowCentavos });

  const equalResult = useMemo(() => {
    if (parsedTotal === null || parsedTotal <= 0) return null;
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
    invalidPin || parsedTotal === null || parsedTotal <= 0
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

  const sharesToSave: SplitResult[] | null =
    method === "equal"
      ? equalResult && !("error" in equalResult)
        ? equalResult.shares
        : null
      : customResult && !("error" in customResult)
        ? customResult.shares
        : null;

  const splitError =
    parsedTotal === null || parsedTotal <= 0
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
    setSelectedIds((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
    // Unselecting drops the pin; the typed value stays so it can be shown as a reminder.
    setPinnedIds((current) => current.filter((value) => value !== id));
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

  async function submit(allowDuplicate: boolean) {
    if (!billTypeId) {
      setError("Choose a bill type.");
      return;
    }
    if (parsedTotal === null || parsedTotal <= 0) {
      setError("Enter a valid bill total greater than zero.");
      return;
    }
    if (!sharesToSave) {
      setError(splitError ?? "Complete the split before saving.");
      return;
    }
    if (note.length > 500) {
      setError("Keep the note to 500 characters or fewer.");
      return;
    }

    setError(null);
    setNotice(null);

    if (!allowDuplicate) {
      setCheckingDuplicates(true);
      try {
        const matches = await queryClient.fetchQuery(
          trpc.bills.findDuplicates.queryOptions({
            billTypeId,
            periodYear: period.year,
            periodMonth: period.month,
          }),
        );
        if (matches.length > 0) {
          setDuplicates(matches);
          return;
        }
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Duplicates could not be checked.");
        return;
      } finally {
        setCheckingDuplicates(false);
      }
    }

    try {
      const id = await create.mutateAsync({
        billTypeId,
        periodYear: period.year,
        periodMonth: period.month,
        totalMinor: parsedTotal,
        dueDate: dueDate || null,
        note: note.trim() || null,
        shares: sharesToSave,
      });
      await navigate({ to: "/bills/$id", params: { id } });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill could not be saved.");
    }
  }

  if (types.isLoading) return <LoadingBlock label="Loading bill types…" />;

  if (activeTypes.length === 0) {
    return (
      <EmptyState
        title="No active bill types"
        description="Add or restore a bill type before creating a bill."
        action={
          <Link to="/bill-types">
            <Button size="sm">Manage bill types</Button>
          </Link>
        }
      />
    );
  }

  if (!boarders.isLoading && (boarders.data ?? []).length === 0) {
    return (
      <EmptyState
        title="No active boarders"
        description="Add the people staying in the boarding house first."
        action={
          <Link to="/boarders">
            <Button size="sm">
              <Users size={15} />
              Go to boarders
            </Button>
          </Link>
        }
      />
    );
  }

  return (
    <div>
      <PageHeader
        eyebrow="New bill"
        title="Create a bill"
        description="Choose a type and month, then split the total among boarders."
        actions={
          <Link to="/">
            <Button variant="secondary">Cancel</Button>
          </Link>
        }
      />

      {error ? (
        <div className="mt-4">
          <ErrorBlock message={error} />
        </div>
      ) : null}
      {notice ? (
        <div className="mt-4">
          <SuccessNote>{notice}</SuccessNote>
        </div>
      ) : null}

      <div className="mt-5 grid items-start gap-5 lg:grid-cols-[1fr_320px]">
        <div className="space-y-5">
          <Card className="p-5">
            <SectionHeader title="Bill type" subtitle="One type per bill." />
            <div className="flex flex-wrap gap-1.5">
              {activeTypes.map((type) => (
                <button
                  key={type.id}
                  type="button"
                  onClick={() => setBillTypeId(type.id)}
                  className={cn(
                    "rounded-full border px-3.5 py-1.5 text-xs font-semibold transition-colors",
                    type.id === billTypeId
                      ? "border-accent bg-accent text-accent-ink"
                      : "border-line bg-panel text-ink hover:border-accent/40",
                  )}
                >
                  {type.name}
                </button>
              ))}
            </div>

            <SectionHeader title="Billing period" />
            <MonthSwitcher
              year={period.year}
              month={period.month}
              onChange={(year, month) => setPeriod({ year, month })}
            />
          </Card>

          <Card className="p-5">
            <SectionHeader title="Amount and details" />
            <div className="grid gap-4 sm:grid-cols-2">
              <Field
                label={`Total bill (${currency})`}
                hint={allowCentavos ? "e.g. 1234.50" : "Whole pesos only."}
                required
              >
                <input
                  className="field"
                  inputMode={allowCentavos ? "decimal" : "numeric"}
                  placeholder="0.00"
                  maxLength={16}
                  value={totalInput}
                  onChange={(event) => {
                    setTotalInput(event.target.value);
                    if (error) setError(null);
                  }}
                />
              </Field>

              <Field label="Due date (optional)">
                <div className="relative">
                  <input
                    type="date"
                    className="field pr-9"
                    value={dueDate}
                    onChange={(event) => setDueDate(event.target.value)}
                  />
                  <CalendarDays
                    size={15}
                    className="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-muted"
                  />
                </div>
              </Field>
            </div>

            <Field label="Note (optional)" hint="Up to 500 characters." className="mt-4">
              <textarea
                className="field-area"
                maxLength={500}
                rows={3}
                placeholder="e.g. November water bill"
                value={note}
                onChange={(event) => setNote(event.target.value)}
              />
            </Field>
          </Card>

          <Card className="p-5">
            <SectionHeader
              title={`Boarders (${selectedIds.length} selected)`}
              subtitle="All active boarders start selected."
              action={
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      setSelectedIds((boarders.data ?? []).map((boarder) => boarder.id))
                    }
                  >
                    All
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setSelectedIds([])}>
                    None
                  </Button>
                </div>
              }
            />

            <div className="grid gap-1 sm:grid-cols-2">
              {(boarders.data ?? []).map((boarder) => {
                const selected = selectedIds.includes(boarder.id);
                return (
                  <label
                    key={boarder.id}
                    className={cn(
                      "flex cursor-pointer items-center gap-2.5 rounded-xl border px-3 py-2 text-sm transition-colors",
                      selected
                        ? "border-accent/30 bg-accent-soft/50 text-ink"
                        : "border-line bg-panel text-muted hover:text-ink",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={() => toggleBoarder(boarder.id)}
                      className="h-4 w-4 accent-accent"
                    />
                    <span className="truncate font-medium">{boarder.name}</span>
                  </label>
                );
              })}
            </div>
          </Card>

          <Card className="p-5">
            <SectionHeader title="How to divide" subtitle="Shares must add up to the total." />

            <div className="inline-flex rounded-xl border border-line bg-canvas p-1">
              {(
                [
                  ["equal", "Equal split"],
                  ["custom", "Custom amounts"],
                ] as const
              ).map(([value, label]) => (
                <button
                  key={value}
                  type="button"
                  onClick={() => chooseMethod(value)}
                  className={cn(
                    "rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors",
                    method === value ? "bg-panel text-ink shadow-sm" : "text-muted hover:text-ink",
                  )}
                >
                  {label}
                </button>
              ))}
            </div>

            <div className="mt-4">
              {selectedBoarders.length === 0 ? (
                <p className="text-sm text-muted">Select at least one boarder to see shares.</p>
              ) : method === "equal" ? (
                equalResult && "error" in equalResult ? (
                  <p className="text-sm text-danger">{equalResult.error}</p>
                ) : equalResult ? (
                  <div className="divide-y divide-line">
                    {equalResult.shares.map((share) => {
                      const boarder = selectedBoarders.find(
                        (item) => item.id === share.boarderId,
                      );
                      return (
                        <div
                          key={share.boarderId}
                          className="flex items-center justify-between py-2"
                        >
                          <span className="text-sm text-ink">{boarder?.name}</span>
                          <span className="text-sm font-semibold tabular-nums text-ink">
                            {formatMoney(share.amountMinor, currency)}
                          </span>
                        </div>
                      );
                    })}
                    {parsedTotal !== null &&
                    selectedBoarders.length > 0 &&
                    parsedTotal % selectedBoarders.length !== 0 ? (
                      <p className="pt-2 text-[11px] text-muted">
                        Leftover centavos are assigned one at a time in selection order.
                      </p>
                    ) : null}
                  </div>
                ) : null
              ) : (
                <div className="space-y-2.5">
                  {pinnedBoarders.length === 0 ? (
                    <p className="text-xs text-muted">
                      Turn on Custom for any boarder to override their amount.
                    </p>
                  ) : null}
                  {selectedBoarders.map((boarder) => {
                    const pinned = pinnedIds.includes(boarder.id);
                    const computed =
                      customResult && !("error" in customResult)
                        ? customResult.shares.find((share) => share.boarderId === boarder.id)
                        : undefined;
                    const stale = pinned ? undefined : customInputs[boarder.id];

                    return (
                      <div
                        key={boarder.id}
                        className="rounded-xl border border-line bg-panel px-3 py-2.5"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <div className="min-w-0 flex-1">
                            <p className="truncate text-sm font-medium text-ink">{boarder.name}</p>
                            {!pinned ? (
                              <p className="mt-0.5 text-[11px] leading-4 text-muted">
                                Auto · split equally
                                {stale ? (
                                  <span className="ml-1.5 line-through opacity-60">{stale}</span>
                                ) : null}
                              </p>
                            ) : null}
                          </div>
                          <div className="flex shrink-0 items-center gap-3">
                            <label className="flex min-h-9 cursor-pointer items-center gap-2 rounded-lg px-1.5 text-xs font-medium text-muted hover:text-ink">
                              <input
                                type="checkbox"
                                checked={pinned}
                                onChange={() => togglePinned(boarder.id)}
                                className="h-4 w-4 accent-accent"
                              />
                              Custom
                            </label>
                            {pinned ? (
                              <input
                                className="field h-10 w-28 text-right font-medium tabular-nums"
                                inputMode={allowCentavos ? "decimal" : "numeric"}
                                placeholder="0.00"
                                value={customInputs[boarder.id] ?? ""}
                                onChange={(event) =>
                                  setCustomInputs((current) => ({
                                    ...current,
                                    [boarder.id]: event.target.value,
                                  }))
                                }
                              />
                            ) : (
                              <span className="w-28 text-right text-sm font-semibold tabular-nums text-ink">
                                {computed ? formatMoney(computed.amountMinor, currency) : "—"}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                  <div className="flex items-center justify-between border-t border-line pt-2.5">
                    <span className="text-xs font-semibold text-muted">Custom amounts</span>
                    <span className="text-sm font-semibold tabular-nums text-ink">
                      {formatMoney(customAssignedMinor, currency)}
                    </span>
                  </div>
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-xs font-semibold text-muted">Leftover</span>
                    <span className="text-right text-xs text-muted">
                      {poolCount === 0
                        ? "every boarder is custom"
                        : `${formatMoney(Math.max(0, leftoverMinor), currency)} split equally among ${poolCount} ${poolCount === 1 ? "boarder" : "boarders"}`}
                    </span>
                  </div>
                  {customResult && !("error" in customResult) && customResult.rounded ? (
                    <p className="text-xs text-muted">
                      Rounded to whole pesos. Turn on centavos in Settings for exact division.
                    </p>
                  ) : null}
                </div>
              )}
            </div>
          </Card>
        </div>

        <Card className="sticky top-6 p-5">
          <p className="label">Summary</p>
          <p className="mt-2 text-2xl font-bold tabular-nums text-ink">
            {parsedTotal === null ? "—" : formatMoney(parsedTotal, currency)}
          </p>
          <p className="mt-1 text-xs text-muted">
            {activeTypes.find((type) => type.id === billTypeId)?.name ?? "No type"} ·{" "}
            {selectedIds.length} {selectedIds.length === 1 ? "boarder" : "boarders"}
          </p>

          <div className="mt-4 space-y-1.5 border-t border-line pt-4 text-xs">
            <div className="flex justify-between gap-2">
              <span className="text-muted">Period</span>
              <span className="font-semibold text-ink">
                {period.year}-{String(period.month).padStart(2, "0")}
              </span>
            </div>
            <div className="flex justify-between gap-2">
              <span className="text-muted">Shares assigned</span>
              <span className="font-semibold text-ink">
                {sharesToSave ? formatMoney(parsedTotal ?? 0, currency) : "Not balanced"}
              </span>
            </div>
          </div>

          {splitError ? <p className="mt-3 text-xs text-danger">{splitError}</p> : null}

          <Button
            className="mt-5 w-full"
            disabled={!sharesToSave || !billTypeId || create.isPending || checkingDuplicates}
            onClick={() => void submit(false)}
          >
            <Plus size={16} />
            {create.isPending
              ? "Creating…"
              : checkingDuplicates
                ? "Checking…"
                : "Create bill"}
          </Button>
        </Card>
      </div>

      <Modal
        open={duplicates !== null}
        onClose={() => setDuplicates(null)}
        title="A bill already exists for this type and month"
        description="You can still create another one if you need a separate bill, but check that this is not accidental."
        footer={
          <>
            <Button variant="ghost" onClick={() => setDuplicates(null)}>
              Cancel
            </Button>
            <Button
              disabled={create.isPending}
              onClick={() => {
                setDuplicates(null);
                void submit(true);
              }}
            >
              Create anyway
            </Button>
          </>
        }
      >
        <div className="space-y-2">
          {(duplicates ?? []).map((duplicate) => (
            <div key={duplicate.id} className="rounded-xl border border-line bg-canvas px-3 py-2">
              <p className="text-sm font-semibold text-ink">
                {formatMoney(duplicate.totalMinor, currency)}
              </p>
              <Link
                to="/bills/$id"
                params={{ id: duplicate.id }}
                className="text-xs font-semibold text-accent"
              >
                Open existing bill
              </Link>
            </div>
          ))}
        </div>
      </Modal>
    </div>
  );
}
