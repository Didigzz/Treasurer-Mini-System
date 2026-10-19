import { Ionicons } from "@expo/vector-icons";
import { Button, Surface, useThemeColor } from "heroui-native";
import { useState } from "react";
import { Text, View } from "react-native";

import {
  AppDialog,
  AppInput,
  ConfirmDialog,
  EmptyState,
  ErrorBlock,
  FormField,
  LoadingBlock,
  Screen,
} from "@/src/features/treasurer/components/ui";
import {
  useArchiveBillType,
  useBillTypes,
  useCreateBillType,
  useRenameBillType,
} from "@/src/features/treasurer/queries";
import type { BillTypeWithCount } from "@/src/features/treasurer/repository";

type EditorState =
  | { mode: "add" }
  | { mode: "rename"; type: BillTypeWithCount }
  | null;

export default function BillTypesScreen() {
  const types = useBillTypes();
  const createType = useCreateBillType();
  const renameType = useRenameBillType();
  const archiveType = useArchiveBillType();

  const [editor, setEditor] = useState<EditorState>(null);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | undefined>();
  const [archiveTarget, setArchiveTarget] = useState<BillTypeWithCount | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");

  const active = (types.data ?? []).filter((type) => !type.isArchived);
  const archived = (types.data ?? []).filter((type) => type.isArchived);

  function openAdd() {
    setEditor({ mode: "add" });
    setName("");
    setNameError(undefined);
    setActionError(null);
  }

  function openRename(type: BillTypeWithCount) {
    setEditor({ mode: "rename", type });
    setName(type.name);
    setNameError(undefined);
    setActionError(null);
  }

  function closeEditor() {
    setEditor(null);
    setName("");
    setNameError(undefined);
  }

  async function handleSave() {
    const trimmed = name.trim();
    if (!trimmed) {
      setNameError("Enter a bill type name.");
      return;
    }
    if (trimmed.length > 40) {
      setNameError("Keep the name to 40 characters or fewer.");
      return;
    }

    try {
      if (editor?.mode === "add") {
        await createType.mutateAsync(trimmed);
      } else if (editor?.mode === "rename") {
        await renameType.mutateAsync({ id: editor.type.id, name: trimmed });
      }
      closeEditor();
    } catch (error) {
      setNameError(error instanceof Error ? error.message : "The bill type could not be saved.");
    }
  }

  async function handleArchive() {
    if (!archiveTarget) return;
    setActionError(null);
    try {
      await archiveType.mutateAsync({ id: archiveTarget.id, archived: true });
      setArchiveTarget(null);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The bill type could not be archived.");
      setArchiveTarget(null);
    }
  }

  async function handleRestore(type: BillTypeWithCount) {
    setActionError(null);
    try {
      await archiveType.mutateAsync({ id: type.id, archived: false });
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "The bill type could not be restored.");
    }
  }

  const pending = createType.isPending || renameType.isPending;

  return (
    <Screen>
      <View className="flex-row items-start justify-between gap-3 pt-4">
        <View className="flex-1">
          <Text className="text-2xl font-semibold text-foreground">Bill types</Text>
          <Text className="mt-1 text-sm text-muted">
            Archive a type to stop using it without changing old bills.
          </Text>
        </View>
        <Button size="sm" onPress={openAdd}>
          <Ionicons name="add" size={17} color={accent} />
          <Button.Label>Add</Button.Label>
        </Button>
      </View>

      {actionError ? (
        <View className="mt-4">
          <ErrorBlock message={actionError} />
        </View>
      ) : null}

      <View className="mt-5">
        {types.isLoading ? (
          <LoadingBlock label="Loading bill types…" />
        ) : types.isError ? (
          <ErrorBlock message="Bill types could not be loaded." onRetry={() => void types.refetch()} />
        ) : (types.data ?? []).length === 0 ? (
          <EmptyState
            icon="pricetags-outline"
            title="No bill types"
            description="Add a type such as Water, Rent, or Electricity."
            actionLabel="Add a bill type"
            onAction={openAdd}
          />
        ) : (
          <>
            <Text className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted">
              Active
            </Text>
            {active.length === 0 ? (
              <EmptyState
                icon="archive-outline"
                title="No active bill types"
                description="Restore a type below or add a new one."
              />
            ) : (
              active.map((type) => (
                <Surface key={type.id} variant="secondary" className="mb-2 rounded-2xl p-3.5">
                  <View className="flex-row items-center justify-between gap-3">
                    <View className="flex-1">
                      <Text className="text-sm font-semibold text-foreground">{type.name}</Text>
                      <Text className="mt-0.5 text-xs text-muted">
                        {type.billCount} {type.billCount === 1 ? "bill" : "bills"}
                      </Text>
                    </View>
                    <View className="flex-row items-center gap-1">
                      <Button size="sm" variant="ghost" onPress={() => openRename(type)}>
                        <Button.Label>Rename</Button.Label>
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onPress={() => setArchiveTarget(type)}
                      >
                        <Button.Label>Archive</Button.Label>
                      </Button>
                    </View>
                  </View>
                </Surface>
              ))
            )}

            {archived.length > 0 ? (
              <>
                <Text className="mb-2 mt-5 text-xs font-semibold uppercase tracking-wide text-muted">
                  Archived
                </Text>
                {archived.map((type) => (
                  <Surface key={type.id} variant="secondary" className="mb-2 rounded-2xl p-3.5">
                    <View className="flex-row items-center justify-between gap-3">
                      <View className="flex-1">
                        <Text className="text-sm font-semibold text-muted">{type.name}</Text>
                        <Text className="mt-0.5 text-xs text-muted">
                          {type.billCount} {type.billCount === 1 ? "bill" : "bills"} kept
                        </Text>
                      </View>
                      <Button size="sm" variant="secondary" onPress={() => void handleRestore(type)}>
                        <Ionicons name="refresh" size={14} color={muted} />
                        <Button.Label>Restore</Button.Label>
                      </Button>
                    </View>
                  </Surface>
                ))}
              </>
            ) : null}
          </>
        )}
      </View>

      <AppDialog
        open={editor !== null}
        onOpenChange={(open) => {
          if (!open) closeEditor();
        }}
        title={editor?.mode === "rename" ? "Rename bill type" : "Add bill type"}
        description={
          editor?.mode === "rename"
            ? "Existing bills keep the name they were created with."
            : "Examples: Water, Electricity, Rent, Wi-Fi, Other."
        }
        actions={
          <>
            <Button variant="ghost" size="sm" onPress={closeEditor}>
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button size="sm" isDisabled={pending} onPress={() => void handleSave()}>
              <Button.Label>{pending ? "Saving…" : "Save"}</Button.Label>
            </Button>
          </>
        }
      >
        <FormField label="Bill type name" error={nameError} required>
          <AppInput
            autoFocus
            value={name}
            onChangeText={(value) => {
              setName(value);
              if (nameError) setNameError(undefined);
            }}
            maxLength={40}
            placeholder="e.g. Water"
            returnKeyType="done"
            onSubmitEditing={() => void handleSave()}
          />
        </FormField>
      </AppDialog>

      <ConfirmDialog
        open={archiveTarget !== null}
        onOpenChange={(open) => {
          if (!open) setArchiveTarget(null);
        }}
        title={`Archive ${archiveTarget?.name ?? "bill type"}?`}
        description="It will disappear from new bills, but existing bills keep the type name and all records stay intact. You can restore it later."
        confirmLabel="Archive"
        destructive
        pending={archiveType.isPending}
        onConfirm={() => void handleArchive()}
      />
    </Screen>
  );
}
