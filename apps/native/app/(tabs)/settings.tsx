import { Ionicons } from "@expo/vector-icons";
import { useQueryClient } from "@tanstack/react-query";
import Constants from "expo-constants";
import { router } from "expo-router";
import { Button, Surface, cn, useThemeColor } from "heroui-native";
import { useState } from "react";
import { Pressable, Text, View } from "react-native";

import { ThemeToggle } from "@/components/theme-toggle";
import {
  exportBackup,
  pickBackupFile,
  restoreBackup,
  type BackupPreview,
} from "@/src/features/treasurer/backup";
import {
  AppDialog,
  ConfirmDialog,
  Screen,
  SectionHeader,
} from "@/src/features/treasurer/components/ui";
import {
  CURRENCY_OPTIONS,
  formatDateTime,
  formatMoney,
} from "@/src/features/treasurer/money";
import {
  useAppSettings,
  useBillTypes,
  useSetCurrency,
} from "@/src/features/treasurer/queries";

export default function SettingsScreen() {
  const settings = useAppSettings();
  const billTypes = useBillTypes();
  const setCurrency = useSetCurrency();
  const queryClient = useQueryClient();

  const accent = useThemeColor("accent");
  const muted = useThemeColor("muted");

  const [currencyOpen, setCurrencyOpen] = useState(false);
  const [restorePreview, setRestorePreview] = useState<BackupPreview | null>(null);
  const [busy, setBusy] = useState<"export" | "pick" | "restore" | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const currency = settings.data?.currencyCode ?? "PHP";
  const activeTypes = (billTypes.data ?? []).filter((type) => !type.isArchived).length;
  const archivedTypes = (billTypes.data ?? []).filter((type) => type.isArchived).length;

  async function handleExport() {
    setBusy("export");
    setError(null);
    setMessage(null);
    try {
      const result = await exportBackup();
      setMessage(`Backup created: ${result.fileName}`);
      await queryClient.invalidateQueries({ queryKey: ["treasurer"] });
    } catch (err) {
      setError(err instanceof Error ? err.message : "The backup could not be exported.");
    } finally {
      setBusy(null);
    }
  }

  async function handlePickRestore() {
    setBusy("pick");
    setError(null);
    setMessage(null);
    try {
      const preview = await pickBackupFile();
      if (preview) setRestorePreview(preview);
    } catch (err) {
      setError(err instanceof Error ? err.message : "The backup could not be read.");
    } finally {
      setBusy(null);
    }
  }

  async function handleRestore() {
    if (!restorePreview) return;
    setBusy("restore");
    setError(null);
    try {
      await restoreBackup(restorePreview.payload);
      setRestorePreview(null);
      setMessage("Backup restored. All current records were replaced.");
      await queryClient.invalidateQueries({ queryKey: ["treasurer"] });
    } catch (err) {
      setError(err instanceof Error ? err.message : "The backup could not be restored.");
    } finally {
      setBusy(null);
    }
  }

  async function handleCurrency(code: string) {
    setError(null);
    try {
      await setCurrency.mutateAsync(code);
      setCurrencyOpen(false);
      setMessage(`Currency changed to ${code}. Existing amounts were not converted.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "The currency could not be changed.");
    }
  }

  return (
    <Screen>
      <View className="pt-5">
        <Text className="text-2xl font-semibold text-foreground">Settings</Text>
        <Text className="mt-1 text-sm text-muted">
          Keep the app simple, local, and easy to back up.
        </Text>
      </View>

      {message ? (
        <View className="mt-4 rounded-xl border border-success/30 bg-success/10 p-3">
          <Text className="text-sm text-success">{message}</Text>
        </View>
      ) : null}
      {error ? (
        <View className="mt-4 rounded-xl border border-danger/30 bg-danger/10 p-3">
          <Text className="text-sm text-danger">{error}</Text>
        </View>
      ) : null}

      <SectionHeader title="Currency" />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`Currency ${currency}. Change currency.`}
        onPress={() => setCurrencyOpen(true)}
        disabled={settings.isLoading}
      >
        <Surface variant="secondary" className="rounded-2xl p-4">
          <View className="flex-row items-center justify-between gap-3">
            <View className="flex-1">
              <Text className="text-sm font-semibold text-foreground">
                {currency} · {formatMoney(123456, currency)}
              </Text>
              <Text className="mt-1 text-xs text-muted">
                Changing currency changes display only; records are not converted.
              </Text>
            </View>
            <Ionicons name="chevron-forward" size={18} color={muted} />
          </View>
        </Surface>
      </Pressable>

      <SectionHeader title="Bill types" />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Manage bill types"
        onPress={() => router.push("/bill-types")}
      >
        <Surface variant="secondary" className="rounded-2xl p-4">
          <View className="flex-row items-center justify-between gap-3">
            <View className="flex-1">
              <Text className="text-sm font-semibold text-foreground">Manage bill types</Text>
              <Text className="mt-1 text-xs text-muted">
                {activeTypes} active
                {archivedTypes > 0 ? `, ${archivedTypes} archived` : ""}. Renaming does not
                change old bills.
              </Text>
            </View>
            <Ionicons name="chevron-forward" size={18} color={muted} />
          </View>
        </Surface>
      </Pressable>

      <SectionHeader title="Backup and restore" subtitle="Manual, complete, and unencrypted." />
      <Surface variant="secondary" className="rounded-2xl p-4">
        <Text className="text-sm font-semibold text-foreground">Last backup</Text>
        <Text className="mt-1 text-xs text-muted">
          {settings.data?.lastBackupAt
            ? formatDateTime(Date.parse(settings.data.lastBackupAt))
            : "No backup yet. Export one regularly."}
        </Text>

        <Button
          className="mt-4"
          isDisabled={busy !== null}
          onPress={() => void handleExport()}
        >
          <Ionicons name="share-outline" size={17} color={accent} />
          <Button.Label>{busy === "export" ? "Preparing backup…" : "Export backup"}</Button.Label>
        </Button>
        <Button
          variant="secondary"
          className="mt-2"
          isDisabled={busy !== null}
          onPress={() => void handlePickRestore()}
        >
          <Ionicons name="download-outline" size={17} color={muted} />
          <Button.Label>{busy === "pick" ? "Opening files…" : "Restore from backup"}</Button.Label>
        </Button>
        <Text className="mt-3 text-xs text-muted">
          The JSON file contains boarder names and payment records. Restoring replaces every
          record on this device. Amounts and notes are not encrypted.
        </Text>
      </Surface>

      <SectionHeader title="Data storage" />
      <Surface variant="secondary" className="rounded-2xl p-4">
        <View className="flex-row items-start gap-2">
          <Ionicons name="phone-portrait-outline" size={18} color={accent} />
          <View className="flex-1">
            <Text className="text-sm font-semibold text-foreground">Only on this phone</Text>
            <Text className="mt-1 text-xs text-muted">
              Records live in the app-private SQLite database treasurer.db. There are no
              accounts, cloud services, analytics, or sync.
            </Text>
            <Text className="mt-2 text-xs font-medium text-danger">
              Uninstalling the app or clearing app data permanently deletes every record unless
              you exported a backup.
            </Text>
            <Text className="mt-2 text-xs text-muted">
              The app does not encrypt its local database. Use your device lock screen and keep
              backups somewhere trusted.
            </Text>
          </View>
        </View>
      </Surface>

      <SectionHeader title="Appearance" />
      <Surface variant="secondary" className="rounded-2xl p-4">
        <View className="flex-row items-center justify-between gap-3">
          <View className="flex-1">
            <Text className="text-sm font-semibold text-foreground">Theme</Text>
            <Text className="mt-1 text-xs text-muted">Switch between light and dark.</Text>
          </View>
          <ThemeToggle />
        </View>
      </Surface>

      <SectionHeader title="About" />
      <Surface variant="secondary" className="rounded-2xl p-4">
        <Text className="text-sm font-semibold text-foreground">Boarding House Treasurer</Text>
        <Text className="mt-1 text-xs text-muted">
          Version {Constants.expoConfig?.version ?? "1.0.0"} · Local record keeping for one
          boarding house.
        </Text>
      </Surface>

      <AppDialog
        open={currencyOpen}
        onOpenChange={setCurrencyOpen}
        title="Choose currency"
        description="Existing amounts are not converted. They will simply be displayed in the new currency."
      >
        <View className="gap-1">
          {CURRENCY_OPTIONS.map((option) => {
            const selected = option.code === currency;
            return (
              <Pressable
                key={option.code}
                accessibilityRole="radio"
                accessibilityState={{ selected }}
                onPress={() => void handleCurrency(option.code)}
                className={cn(
                  "min-h-12 flex-row items-center justify-between rounded-xl px-3",
                  selected ? "bg-accent/15" : "bg-transparent",
                )}
              >
                <View>
                  <Text className="text-sm font-medium text-foreground">{option.code}</Text>
                  <Text className="text-xs text-muted">{option.label}</Text>
                </View>
                {selected ? <Ionicons name="checkmark" size={18} color={accent} /> : null}
              </Pressable>
            );
          })}
        </View>
      </AppDialog>

      <ConfirmDialog
        open={Boolean(restorePreview)}
        onOpenChange={(open) => {
          if (!open) setRestorePreview(null);
        }}
        title="Replace all data?"
        description={
          restorePreview
            ? `Restore ${restorePreview.fileName}? It contains ${restorePreview.counts.boarders} boarders, ${restorePreview.counts.bills} bills, ${restorePreview.counts.shares} shares, and ${restorePreview.counts.payments} payments. All current records on this phone will be replaced.`
            : undefined
        }
        confirmLabel={busy === "restore" ? "Restoring…" : "Replace all data"}
        destructive
        pending={busy === "restore"}
        onConfirm={() => void handleRestore()}
      />
    </Screen>
  );
}
