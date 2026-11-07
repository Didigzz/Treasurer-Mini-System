import { settings } from "@my-better-t-app/db/schema";
import { eq } from "drizzle-orm";

import type { Database } from "@my-better-t-app/db";

export interface AppSettings {
  currencyCode: string;
  lastBackupAt: string | null;
}

export async function getSetting(db: Database, key: string): Promise<string | undefined> {
  const rows = await db
    .select({ value: settings.value })
    .from(settings)
    .where(eq(settings.key, key))
    .limit(1);

  return rows[0]?.value;
}

export async function setSetting(db: Database, key: string, value: string): Promise<void> {
  await db
    .insert(settings)
    .values({ key, value, updatedAt: Date.now() })
    .onConflictDoUpdate({
      target: settings.key,
      set: { value, updatedAt: Date.now() },
    });
}

export async function getAppSettings(db: Database): Promise<AppSettings> {
  const [currencyCode, lastBackupAt] = await Promise.all([
    getSetting(db, "currency_code"),
    getSetting(db, "last_backup_at"),
  ]);

  return {
    currencyCode: currencyCode || "PHP",
    lastBackupAt: lastBackupAt || null,
  };
}

export async function setCurrencyCode(db: Database, currencyCode: string): Promise<void> {
  await setSetting(db, "currency_code", currencyCode);
}

export async function setLastBackupAt(db: Database, value: string): Promise<void> {
  await setSetting(db, "last_backup_at", value);
}
