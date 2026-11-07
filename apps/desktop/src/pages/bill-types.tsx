import { Archive, Pencil, Plus, RotateCcw, Tags } from "lucide-react";
import { useState } from "react";

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
import { useBillTypeMutations, useBillTypes } from "@/queries";

export function BillTypesPage() {
  const types = useBillTypes();
  const { create, rename, setArchived } = useBillTypeMutations();

  const [newName, setNewName] = useState("");
  const [editing, setEditing] = useState<{ id: string; name: string } | null>(null);
  const [archiving, setArchiving] = useState<{ id: string; name: string; archived: boolean } | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function handleCreate() {
    const name = newName.trim();
    if (!name) {
      setError("Enter a bill type name.");
      return;
    }
    setError(null);
    setNotice(null);
    try {
      await create.mutateAsync({ name });
      setNewName("");
      setNotice(`${name} added.`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill type could not be added.");
    }
  }

  async function handleRename() {
    if (!editing) return;
    const name = editing.name.trim();
    if (!name) {
      setError("Enter a bill type name.");
      return;
    }
    setError(null);
    setNotice(null);
    try {
      await rename.mutateAsync({ id: editing.id, name });
      setEditing(null);
      setNotice("Bill type renamed.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill type could not be renamed.");
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
          ? `${archiving.name} archived. Existing bills keep their snapshot name.`
          : `${archiving.name} restored.`,
      );
      setArchiving(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The bill type could not be updated.");
    }
  }

  return (
    <div>
      <PageHeader
        title="Bill types"
        description="Reusable categories such as water, rent, or electricity. Archived types cannot be added to new bills."
      />

      <Card className="mt-6 p-4">
        <div className="flex flex-wrap items-end gap-2">
          <Field label="Add a bill type" className="min-w-60 flex-1">
            <input
              className="field"
              value={newName}
              maxLength={40}
              placeholder="e.g. Internet"
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
            {create.isPending ? "Adding…" : "Add type"}
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

      <div className="mt-5">
        {types.isLoading ? (
          <LoadingBlock label="Loading bill types…" />
        ) : types.isError ? (
          <ErrorBlock message="Bill types could not be loaded." onRetry={() => void types.refetch()} />
        ) : (types.data ?? []).length === 0 ? (
          <EmptyState title="No bill types yet" description="Add the first category above." />
        ) : (
          <Card className="divide-y divide-line">
            {(types.data ?? []).map((type) => (
              <div key={type.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
                <div
                  className={cn(
                    "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl",
                    type.isArchived ? "bg-canvas text-muted" : "bg-accent-soft text-accent",
                  )}
                >
                  <Tags size={16} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="text-sm font-semibold text-ink">{type.name}</p>
                    {type.isArchived ? (
                      <Badge>
                        <Archive size={11} />
                        Archived
                      </Badge>
                    ) : null}
                  </div>
                  <p className="mt-0.5 text-xs text-muted">
                    {type.billCount} {type.billCount === 1 ? "bill" : "bills"} · sort order{" "}
                    {type.sortOrder}
                  </p>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setEditing({ id: type.id, name: type.name })}
                  >
                    <Pencil size={14} />
                    Rename
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      setArchiving({ id: type.id, name: type.name, archived: !type.isArchived })
                    }
                  >
                    {type.isArchived ? <RotateCcw size={14} /> : <Archive size={14} />}
                    {type.isArchived ? "Restore" : "Archive"}
                  </Button>
                </div>
              </div>
            ))}
          </Card>
        )}
      </div>

      <Modal
        open={editing !== null}
        onClose={() => setEditing(null)}
        title="Rename bill type"
        description="Existing bills keep the name they were created with; new bills use the new name."
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
            maxLength={40}
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
        title={archiving?.archived ? "Archive this bill type?" : "Restore this bill type?"}
        description={
          archiving?.archived
            ? "It disappears from the new-bill picker. Existing bills and their totals are unchanged."
            : "It becomes available when creating bills again. Names must stay unique among active types."
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
