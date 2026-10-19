import { Ionicons } from "@expo/vector-icons";
import { router } from "expo-router";
import { Button, Surface, cn, useThemeColor } from "heroui-native";
import { useMemo, useState } from "react";
import { Pressable, Text, View } from "react-native";

import {
  AppDialog,
  AppInput,
  EmptyState,
  ErrorBlock,
  FormField,
  LoadingBlock,
  Screen,
} from "@/src/features/treasurer/components/ui";
import { formatMoney } from "@/src/features/treasurer/money";
import {
  useAppSettings,
  useBoarders,
  useCreateBoarder,
} from "@/src/features/treasurer/queries";

type BoarderTab = "active" | "archived";

export default function BoardersScreen() {
  const [tab, setTab] = useState<BoarderTab>("active");
  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | undefined>();

  const boarders = useBoarders();
  const appSettings = useAppSettings();
  const createBoarder = useCreateBoarder();
  const currency = appSettings.data?.currencyCode ?? "PHP";
  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");

  const active = useMemo(
    () => (boarders.data ?? []).filter((boarder) => boarder.status === "active"),
    [boarders.data],
  );
  const archived = useMemo(
    () => (boarders.data ?? []).filter((boarder) => boarder.status === "archived"),
    [boarders.data],
  );
  const visible = tab === "active" ? active : archived;
  const duplicateName = useMemo(() => {
    const trimmed = name.trim().toLowerCase();
    if (!trimmed) return false;
    return (boarders.data ?? []).some(
      (boarder) => boarder.name.trim().toLowerCase() === trimmed,
    );
  }, [name, boarders.data]);

  function resetDialog() {
    setName("");
    setNameError(undefined);
    createBoarder.reset();
  }

  async function handleAdd() {
    const trimmed = name.trim();
    if (!trimmed) {
      setNameError("Enter a boarder name.");
      return;
    }
    if (trimmed.length > 80) {
      setNameError("Keep the name to 80 characters or fewer.");
      return;
    }

    try {
      await createBoarder.mutateAsync(trimmed);
      setAddOpen(false);
      resetDialog();
    } catch (error) {
      setNameError(error instanceof Error ? error.message : "The boarder could not be added.");
    }
  }

  return (
    <Screen>
      <View className="flex-row items-start justify-between gap-3 pt-5">
        <View className="flex-1">
          <Text className="text-2xl font-semibold text-foreground">Boarders</Text>
          <Text className="mt-1 text-sm text-muted">
            Names only. Archive people when they move out; their history stays intact.
          </Text>
        </View>
        <Button
          size="sm"
          onPress={() => {
            resetDialog();
            setAddOpen(true);
          }}
        >
          <Ionicons name="add" size={17} color={accent} />
          <Button.Label>Add</Button.Label>
        </Button>
      </View>

      <View className="mt-5 flex-row rounded-xl bg-surface p-1">
        {(
          [
            ["active", `Active ${active.length}`],
            ["archived", `Archived ${archived.length}`],
          ] as const
        ).map(([value, label]) => (
          <Pressable
            key={value}
            accessibilityRole="tab"
            accessibilityState={{ selected: tab === value }}
            onPress={() => setTab(value)}
            className={cn(
              "min-h-11 flex-1 items-center justify-center rounded-lg",
              tab === value ? "bg-accent" : "bg-transparent",
            )}
          >
            <Text
              className={cn(
                "text-sm font-medium",
                tab === value ? "text-accent-foreground" : "text-foreground",
              )}
            >
              {label}
            </Text>
          </Pressable>
        ))}
      </View>

      <View className="mt-4">
        {boarders.isLoading ? (
          <LoadingBlock label="Loading boarders…" />
        ) : boarders.isError ? (
          <ErrorBlock
            message="The boarder list could not be loaded."
            onRetry={() => void boarders.refetch()}
          />
        ) : visible.length === 0 ? (
          <EmptyState
            icon="people-outline"
            title={tab === "active" ? "No boarders yet" : "No archived boarders"}
            description={
              tab === "active"
                ? "Add the people currently staying in the boarding house."
                : "People you archive will appear here with their history intact."
            }
            actionLabel={tab === "active" ? "Add your first boarder" : undefined}
            onAction={tab === "active" ? () => setAddOpen(true) : undefined}
          />
        ) : (
          visible.map((boarder) => (
            <Pressable
              key={boarder.id}
              accessibilityRole="button"
              accessibilityLabel={`${boarder.name}, ${boarder.status}`}
              className="mb-2"
              onPress={() =>
                router.push({ pathname: "/boarders/[id]", params: { id: boarder.id } })
              }
            >
              <Surface variant="secondary" className="rounded-2xl p-4">
                <View className="flex-row items-center justify-between gap-3">
                  <View className="flex-1">
                    <Text
                      className={cn(
                        "text-base font-semibold",
                        tab === "archived" ? "text-muted" : "text-foreground",
                      )}
                    >
                      {boarder.name}
                    </Text>
                    <Text className="mt-1 text-xs text-muted">
                      {boarder.outstandingMinor > 0
                        ? `${formatMoney(boarder.outstandingMinor, currency)} outstanding`
                        : "No outstanding balance"}
                      {boarder.changeOutstandingMinor > 0
                        ? ` • ${formatMoney(boarder.changeOutstandingMinor, currency)} change`
                        : ""}
                    </Text>
                  </View>
                  <View className="flex-row items-center gap-1">
                    {tab === "archived" ? (
                      <Text className="text-xs text-muted">Archived</Text>
                    ) : null}
                    <Ionicons name="chevron-forward" size={17} color={muted} />
                  </View>
                </View>
              </Surface>
            </Pressable>
          ))
        )}
      </View>

      <AppDialog
        open={addOpen}
        onOpenChange={(open) => {
          setAddOpen(open);
          if (!open) resetDialog();
        }}
        title="Add boarder"
        description="Only a name is needed. Renaming or archiving later will not change old bills."
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onPress={() => {
                setAddOpen(false);
                resetDialog();
              }}
            >
              <Button.Label>Cancel</Button.Label>
            </Button>
            <Button size="sm" isDisabled={createBoarder.isPending} onPress={() => void handleAdd()}>
              <Button.Label>{createBoarder.isPending ? "Adding…" : "Add boarder"}</Button.Label>
            </Button>
          </>
        }
      >
        <FormField label="Name" error={nameError} required>
          <AppInput
            autoFocus
            value={name}
            onChangeText={(value) => {
              setName(value);
              if (nameError) setNameError(undefined);
            }}
            placeholder="e.g. Maria Santos"
            maxLength={80}
            returnKeyType="done"
            onSubmitEditing={() => void handleAdd()}
          />
          {duplicateName ? (
            <Text className="mt-1.5 text-xs text-warning">
              Another boarder already has this name. You can still add them.
            </Text>
          ) : null}
        </FormField>
      </AppDialog>
    </Screen>
  );
}
