import { Download, HardDrive, Moon, RefreshCcw, Sun, Upload } from "lucide-react";
import { useEffect, useState } from "react";

import { CURRENCY_OPTIONS } from "@my-better-t-app/api/treasurer/money";
import { formatDateTime } from "@my-better-t-app/api/treasurer/money";
import type { BackupCounts } from "@my-better-t-app/api/treasurer/backup";
import {
  Badge,
  Button,
  Card,
  ErrorBlock,
  Field,
  LoadingBlock,
  Modal,
  PageHeader,
  SectionHeader,
  SuccessNote,
  cn,
} from "@/components/ui";
import { openBackupFile, saveBackupFile } from "@/lib/backup-files";
import { useTheme } from "@/lib/theme";
import {
  useAppSettings,
  useBackupQueries,
  useSettingsMutations,
} from "@/queries";

export function SettingsPage() {
  const settings = useAppSettings();
  const { setCurrency, setAllowCentavos, importBackup } = useSettingsMutations();
  const { exportPayload, inspect } = useBackupQueries();
  const { theme, setTheme } = useTheme();

  const themeOptions = [
    { value: "light", label: "Light", icon: <Sun size={15} /> },
    { value: "dark", label: "Dark", icon: <Moon size={15} /> },
  ] as const;

  const [currency, setCurrencyValue] = useState("PHP");
  const [savedCurrency, setSavedCurrency] = useState("PHP");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<"export" | "inspect" | "import" | null>(null);
  const [pendingImport, setPendingImport] = useState<{
    json: string;
    path: string | null;
    exportedAt: string;
    counts: BackupCounts;
  } | null>(null);

  useEffect(() => {
    if (!settings.data) return;
    setCurrencyValue(settings.data.currencyCode);
    setSavedCurrency(settings.data.currencyCode);
  }, [settings.data]);

  const dirty = currency !== savedCurrency;

  async function handleSaveCurrency() {
    setError(null);
    setNotice(null);
    try {
      await setCurrency.mutateAsync({ currencyCode: currency });
      setSavedCurrency(currency);
      setNotice("Currency updated. Existing amounts keep their numeric values.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The currency could not be saved.");
    }
  }

  async function handleToggleCentavos(next: boolean) {
    setError(null);
    setNotice(null);
    try {
      await setAllowCentavos.mutateAsync({ allowCentavos: next });
      setNotice(
        next
          ? "Centavos are allowed in new amounts."
          : "New amounts are whole pesos only. Existing centavo amounts are unchanged.",
      );
    } catch (cause) {
      setError(
        cause instanceof Error ? cause.message : "The centavos setting could not be saved.",
      );
    }
  }

  async function handleExport() {
    setBusy("export");
    setError(null);
    setNotice(null);
    try {
      const payload = await exportPayload();
      const result = await saveBackupFile(payload);
      if (result) {
        await settings.refetch();
        setNotice(
          result.path
            ? `Backup saved to ${result.path}`
            : "Backup download started.",
        );
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The backup could not be saved.");
    } finally {
      setBusy(null);
    }
  }

  async function handleChooseImport() {
    setBusy("inspect");
    setError(null);
    setNotice(null);
    try {
      const opened = await openBackupFile();
      if (!opened) return;
      const meta = await inspect(opened.json);
      setPendingImport({
        json: opened.json,
        path: opened.path,
        exportedAt: meta.exportedAt,
        counts: meta.counts,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The backup could not be read.");
    } finally {
      setBusy(null);
    }
  }

  async function handleConfirmImport() {
    if (!pendingImport) return;
    setBusy("import");
    setError(null);
    setNotice(null);
    try {
      const result = await importBackup.mutateAsync({ json: pendingImport.json });
      setPendingImport(null);
      setNotice(
        `Backup restored: ${result.counts.boarders} boarders, ${result.counts.bills} bills, ${result.counts.payments} payments.`,
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The backup could not be restored.");
    } finally {
      setBusy(null);
    }
  }

  if (settings.isLoading) return <LoadingBlock label="Loading settings…" />;

  if (settings.isError || !settings.data) {
    return (
      <ErrorBlock
        message="Settings could not be loaded."
        onRetry={() => void settings.refetch()}
      />
    );
  }

  const lastBackup = settings.data.lastBackupAt ? Date.parse(settings.data.lastBackupAt) : null;

  return (
    <div>
      <PageHeader
        title="Settings"
        description="Currency, backups, and where your data lives."
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

      <div className="mt-5 grid gap-5 lg:grid-cols-2">
        <Card className="p-5">
          <SectionHeader
            title="Currency"
            subtitle="Used across the dashboard, bills, and payment screens."
          />
          <Field label="Display currency" hint="Amounts are not converted.">
            <select
              className="field"
              value={currency}
              onChange={(event) => setCurrencyValue(event.target.value)}
            >
              {CURRENCY_OPTIONS.map((option) => (
                <option key={option.code} value={option.code}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>
          <Button
            className="mt-4"
            disabled={!dirty || setCurrency.isPending}
            onClick={() => void handleSaveCurrency()}
          >
            {setCurrency.isPending ? "Saving…" : "Save currency"}
          </Button>
          {dirty ? (
            <p className="mt-2 text-xs text-muted">
              You have unsaved changes.
            </p>
          ) : null}

          <div className="mt-5 flex items-start justify-between gap-3 border-t border-line pt-4">
            <div className="min-w-0">
              <p className="text-sm font-semibold text-ink">Allow centavos</p>
              <p className="mt-0.5 text-xs text-muted">
                Turn on centavos for exact division. When off, bill splits round to whole pesos.
              </p>
            </div>
            <label className="flex shrink-0 cursor-pointer items-center gap-2 text-xs font-semibold text-muted">
              <input
                type="checkbox"
                className="h-4 w-4 accent-accent"
                checked={settings.data.allowCentavos}
                disabled={setAllowCentavos.isPending}
                onChange={(event) => void handleToggleCentavos(event.target.checked)}
              />
              {settings.data.allowCentavos ? "On" : "Off"}
            </label>
          </div>
        </Card>

        <Card className="p-5">
          <SectionHeader title="Storage" subtitle="Local-only, on this computer." />
          <div className="flex items-start gap-3 rounded-xl bg-canvas p-3">
            <HardDrive size={18} className="mt-0.5 shrink-0 text-muted" />
            <div className="text-xs leading-relaxed text-muted">
              <p className="font-semibold text-ink">Desktop app data folder</p>
              <p className="mt-0.5">
                The database lives in the app data directory as{" "}
                <code className="rounded bg-panel px-1 py-0.5">treasurer.db</code>. Use Export
                backup before moving to another computer.
              </p>
            </div>
          </div>
          <div className="mt-4 flex items-center justify-between gap-3 text-xs">
            <span className="text-muted">Last backup</span>
            <span className="font-semibold text-ink">
              {lastBackup ? formatDateTime(lastBackup) : "Never"}
            </span>
          </div>
        </Card>

        <Card className="p-5 lg:col-span-2">
          <SectionHeader
            title="Appearance"
            subtitle="Choose how the app looks. Light is the default."
          />
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-1 rounded-xl border border-line bg-canvas p-1">
              {themeOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => setTheme(option.value)}
                  aria-pressed={theme === option.value}
                  className={cn(
                    "inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors",
                    theme === option.value
                      ? "bg-panel text-ink shadow-sm"
                      : "text-muted hover:text-ink",
                  )}
                >
                  {option.icon}
                  {option.label}
                </button>
              ))}
            </div>
            <p className="text-xs text-muted">
              Saved on this computer and applied immediately.
            </p>
          </div>
        </Card>

        <Card className="p-5 lg:col-span-2">
          <SectionHeader
            title="Backup and restore"
            subtitle="Exports every bill type, boarder, bill, share, payment, and setting as JSON."
          />
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => void handleExport()} disabled={busy !== null}>
              <Download size={16} />
              {busy === "export" ? "Preparing…" : "Export backup"}
            </Button>
            <Button
              variant="secondary"
              onClick={() => void handleChooseImport()}
              disabled={busy !== null}
            >
              <Upload size={16} />
              {busy === "inspect" ? "Reading…" : "Import backup"}
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                void settings.refetch();
                setNotice(null);
                setError(null);
              }}
            >
              <RefreshCcw size={15} />
              Refresh
            </Button>
          </div>

          <div className="mt-4 rounded-xl border border-warning/30 bg-warning-soft px-4 py-3 text-xs leading-relaxed text-warning">
            Importing replaces <strong>all</strong> current data with the file contents. Changes
            are intentional and cannot be undone without another backup. Restoring a backup from
            the Expo app or another computer is the supported migration path.
          </div>

          {!settings.data.lastBackupAt ? (
            <p className="mt-3 text-xs text-muted">
              No backup has been recorded yet. Export one before updating the app or moving
              computers.
            </p>
          ) : null}
        </Card>
      </div>

      <Modal
        open={pendingImport !== null}
        onClose={() => {
          if (busy !== "import") setPendingImport(null);
        }}
        title="Replace all data with this backup?"
        description={
          pendingImport
            ? `Exported ${formatDateTime(Date.parse(pendingImport.exportedAt))}${
                pendingImport.path ? ` · ${pendingImport.path}` : ""
              }`
            : undefined
        }
        footer={
          <>
            <Button
              variant="ghost"
              onClick={() => setPendingImport(null)}
              disabled={busy === "import"}
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={() => void handleConfirmImport()}
              disabled={busy === "import"}
            >
              {busy === "import" ? "Restoring…" : "Replace data"}
            </Button>
          </>
        }
      >
        {pendingImport ? (
          <div>
            <p className="text-xs text-muted">This file contains:</p>
            <div className="mt-3 grid grid-cols-2 gap-2 text-sm sm:grid-cols-3">
              {[
                ["Bill types", pendingImport.counts.billTypes],
                ["Boarders", pendingImport.counts.boarders],
                ["Bills", pendingImport.counts.bills],
                ["Shares", pendingImport.counts.shares],
                ["Payments", pendingImport.counts.payments],
              ].map(([label, count]) => (
                <div key={label} className="rounded-xl bg-canvas px-3 py-2">
                  <p className="text-[11px] text-muted">{label}</p>
                  <p className="font-bold tabular-nums text-ink">{count}</p>
                </div>
              ))}
            </div>
            <div className="mt-4 flex items-center gap-2">
              <Badge tone="warning">Replaces current data</Badge>
              <Badge>Validated</Badge>
            </div>
            <p className="mt-3 text-xs text-muted">
              Export a backup first if you may need the current data later.
            </p>
          </div>
        ) : null}
      </Modal>
    </div>
  );
}
