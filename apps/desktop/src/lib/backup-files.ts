import type { BackupPayload } from "@my-better-t-app/api/treasurer/backup";

import { isTauri } from "@/env";

export interface OpenedBackup {
  path: string | null;
  json: string;
}

function timestampForFilename(date = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}-${pad(
    date.getHours(),
  )}${pad(date.getMinutes())}`;
}

function browserDownload(contents: string, filename: string) {
  const blob = new Blob([contents], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function browserOpen(): Promise<OpenedBackup | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "application/json,.json";

    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      file
        .text()
        .then((json) => resolve({ path: null, json }))
        .catch(() => reject(new Error("The backup file could not be read.")));
    };

    // Cancelling a file picker fires no event; the promise simply stays open
    // until the user picks a file, which is acceptable for the browser fallback.
    input.click();
  });
}

export async function saveBackupFile(
  payload: BackupPayload,
): Promise<{ path: string | null } | null> {
  const filename = `boarding-house-backup-${timestampForFilename()}.json`;
  const contents = JSON.stringify(payload, null, 2);

  if (!isTauri()) {
    browserDownload(contents, filename);
    return { path: filename };
  }

  const { save } = await import("@tauri-apps/plugin-dialog");
  const { invoke } = await import("@tauri-apps/api/core");
  const path = await save({
    title: "Save treasurer backup",
    defaultPath: filename,
    filters: [{ name: "JSON backup", extensions: ["json"] }],
  });

  if (!path) return null;

  await invoke("write_text_file", { path, contents });
  return { path };
}

export async function openBackupFile(): Promise<OpenedBackup | null> {
  if (!isTauri()) return browserOpen();

  const { open } = await import("@tauri-apps/plugin-dialog");
  const { invoke } = await import("@tauri-apps/api/core");
  const selected = await open({
    title: "Open treasurer backup",
    multiple: false,
    directory: false,
    filters: [{ name: "JSON backup", extensions: ["json"] }],
  });

  if (!selected) return null;
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return null;

  const json = await invoke<string>("read_text_file", { path });
  return { path, json };
}
