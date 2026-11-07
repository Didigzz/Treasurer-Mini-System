import { Link } from "@tanstack/react-router";
import { Archive, ChevronRight, Pencil, Plus, RotateCcw } from "lucide-react";
import { useState } from "react";

import { formatMoney } from "@my-better-t-app/api/treasurer/money";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  ErrorBlock,
  Field,
  LoadingBlock,
  Modal,
  PageHeader,
  SuccessNote,
  cn,
} from "@/components/ui";
import {
  useAppSettings,
  useBoarderMutations,
  useBoardersWithBalances,
} from "@/queries";

export function BoardersPage() {
  const boarders = useBoardersWithBalances();
  const settings = useAppSettings();
  const { create, rename, setArchived } = useBoarderMutations();
  const currency = settings.data?.currencyCode ?? "PHP";

  const [newName, setNewName] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<{ id: string; name: string } | null>(null);
  const [archiving, setArchiving] = useState<{ id: string; name: string; archived: boolean } | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const rows = (boarders.data ?? []).filter(
    (boarder) => showArchived || boarder.status === "active",
  );
  const archivedCount = (boarders.data ?? []).filter(
    (boarder) => boarder.status === "archived",
  ).length;

  async function handleCreate() {
    const name = newName.trim();
    if (!name) {
      setError("Enter a boarder name.");
      return;
    }
    setError(null);
    setNotice(null);
    try {
      await create.mutateAsync({ name });
      setNewName("");
      setNotice(`${name} added.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The boarder could not be added.");
    }
  }

  async function handleRename() {
    if (!editing) return;
    const name = editing.name.trim();
    if (!name) {
      setError("Enter a boarder name.");
      return;
    }
    setError(null);
    setNotice(null);
    try {
      await rename.mutateAsync({ id: editing.id, name });
      setEditing(null);
      setNotice("Boarder renamed.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The boarder could not be renamed.");
    }
  }

  async function handleArchive() {
    if (!archiving) return;
    setError(null);
    setNotice(null);
    try {
      await setArchived.mutateAsync({ id: archiving.id, isArchived: archiving.archived });
      setNotice(
        archiving.archived
          ? `${archiving.name} archived. Their history stays intact.`
          : `${archiving.name} restored.`,
      );
      setArchiving(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The boarder could not be updated.");
    }
  }

  return (
    <div>
      <PageHeader
        title="Boarders"
        description="People staying in the boarding house. Balances include every bill they appear on."
      />

      <Card className="mt-6 p-4">
        <div className="flex flex-wrap items-end gap-2">
          <Field label="Add a boarder" className="min-w-60 flex-1">
            <input
              className="field"
              value={newName}
              maxLength={80}
              placeholder="Full name"
              onChange={(event) => {
                setNewName(event.target.value);
                if (error) setError(null);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") void handleCreate();
              }}
            />
          </Field>
          <Button onClick={() => void handleCreate()} disabled={create.isPending}>
            <Plus size={16} />
            {create.isPending ? "Adding…" : "Add boarder"}
          </Button>
        </div>
      </Card>

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

      <div className="mt-5 flex items-center justify-between gap-3">
        <p className="text-xs text-muted">
          {rows.length} {rows.length === 1 ? "boarder" : "boarders"}
        </p>
        {archivedCount > 0 ? (
          <label className="flex cursor-pointer items-center gap-2 text-xs font-semibold text-accent">
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(event) => setShowArchived(event.target.checked)}
              className="h-3.5 w-3.5 accent-accent"
            />
            Show archived ({archivedCount})
          </label>
        ) : null}
      </div>

      <div className="mt-3">
        {boarders.isLoading ? (
          <LoadingBlock label="Loading boarders…" />
        ) : boarders.isError ? (
          <ErrorBlock
            message="Boarders could not be loaded."
            onRetry={() => void boarders.refetch()}
          />
        ) : rows.length === 0 ? (
          <EmptyState
            title="No boarders yet"
            description="Add the first boarder above, then include them when creating a bill."
          />
        ) : (
          <Card className="divide-y divide-line">
            {rows.map((boarder) => (
              <div key={boarder.id} className="flex items-center gap-3 px-4 py-3">
                <Link
                  to="/boarders/$id"
                  params={{ id: boarder.id }}
                  className="flex min-w-0 flex-1 items-center gap-3"
                >
                  <div
                    className={cn(
                      "flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-xs font-bold",
                      boarder.status === "active"
                        ? "bg-accent-soft text-accent"
                        : "bg-canvas text-muted",
                    )}
                  >
                    {boarder.name.slice(0, 2).toUpperCase()}
                  </div>
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="truncate text-sm font-semibold text-ink">{boarder.name}</p>
                      {boarder.status === "archived" ? (
                        <Badge>
                          <Archive size={11} />
                          Archived
                        </Badge>
                      ) : null}
                    </div>
                    <p className="mt-0.5 text-xs text-muted">
                      {boarder.outstandingMinor > 0
                        ? `${formatMoney(boarder.outstandingMinor, currency)} outstanding`
                        : "No balance"}
                      {boarder.changeOutstandingMinor > 0
                        ? ` · ${formatMoney(boarder.changeOutstandingMinor, currency)} change`
                        : ""}
                    </p>
                  </div>
                </Link>

                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setEditing({ id: boarder.id, name: boarder.name })}
                  >
                    <Pencil size={14} />
                    Rename
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      setArchiving({
                        id: boarder.id,
                        name: boarder.name,
                        archived: boarder.status === "active",
                      })
                    }
                  >
                    {boarder.status === "active" ? <Archive size={14} /> : <RotateCcw size={14} />}
                    {boarder.status === "active" ? "Archive" : "Restore"}
                  </Button>
                  <Link to="/boarders/$id" params={{ id: boarder.id }}>
                    <Button size="sm" variant="ghost" aria-label={`Open ${boarder.name}`}>
                      <ChevronRight size={16} />
                    </Button>
                  </Link>
                </div>
              </div>
            ))}
          </Card>
        )}
      </div>

      <Modal
        open={editing !== null}
        onClose={() => setEditing(null)}
        title="Rename boarder"
        description="The new name applies to current records. Bill history keeps the name used at the time."
        footer={
          <>
            <Button variant="ghost" onClick={() => setEditing(null)}>
              Cancel
            </Button>
            <Button onClick={() => void handleRename()} disabled={rename.isPending}>
              {rename.isPending ? "Saving…" : "Save name"}
            </Button>
          </>
        }
      >
        <Field label="Name" required>
          <input
            className="field"
            autoFocus
            maxLength={80}
            value={editing?.name ?? ""}
            onChange={(event) =>
              setEditing((current) => (current ? { ...current, name: event.target.value } : null))
            }
            onKeyDown={(event) => {
              if (event.key === "Enter") void handleRename();
            }}
          />
        </Field>
      </Modal>

      <Modal
        open={archiving !== null}
        onClose={() => setArchiving(null)}
        title={archiving?.archived ? "Archive this boarder?" : "Restore this boarder?"}
        description={
          archiving?.archived
            ? "Archived boarders cannot be added to new bills, but their existing bills, payments, and balances stay visible."
            : "Restored boarders can be included in new bills again."
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setArchiving(null)}>
              Cancel
            </Button>
            <Button
              variant={archiving?.archived ? "danger" : "primary"}
              onClick={() => void handleArchive()}
              disabled={setArchived.isPending}
            >
              {setArchived.isPending ? "Saving…" : archiving?.archived ? "Archive" : "Restore"}
            </Button>
          </>
        }
      />
    </div>
  );
}
