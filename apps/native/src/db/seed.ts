import { billTypes, settings } from "@my-better-t-app/db/schema";
import { eq } from "drizzle-orm";

import { db } from "./index";

export const DEFAULT_BILL_TYPES = [
  { id: "type-water", name: "Water", sortOrder: 0 },
  { id: "type-electricity", name: "Electricity", sortOrder: 1 },
  { id: "type-rent", name: "Rent", sortOrder: 2 },
  { id: "type-wifi", name: "Wi-Fi", sortOrder: 3 },
  { id: "type-other", name: "Other", sortOrder: 4 },
] as const;

const DEFAULT_SETTINGS = [
  { key: "currency_code", value: "PHP" },
  { key: "last_backup_at", value: "" },
] as const;

export async function seedDatabase(): Promise<void> {
  const now = Date.now();

  await db
    .insert(billTypes)
    .values(
      DEFAULT_BILL_TYPES.map((type) => ({
        ...type,
        isArchived: false,
        createdAt: now,
        updatedAt: now,
      })),
    )
    .onConflictDoNothing();

  await db
    .insert(settings)
    .values(
      DEFAULT_SETTINGS.map((setting) => ({
        ...setting,
        updatedAt: now,
      })),
    )
    .onConflictDoNothing();
}

export async function getSetting(key: string): Promise<string | undefined> {
  const rows = await db
    .select({ value: settings.value })
    .from(settings)
    .where(eq(settings.key, key))
    .limit(1);

  return rows[0]?.value;
}

export async function setSetting(key: string, value: string): Promise<void> {
  await db
    .insert(settings)
    .values({ key, value, updatedAt: Date.now() })
    .onConflictDoUpdate({
      target: settings.key,
      set: { value, updatedAt: Date.now() },
    });
}
