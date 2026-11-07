import { billTypes, settings } from "@my-better-t-app/db/schema";

import type { Database } from "@my-better-t-app/db";

export const DEFAULT_BILL_TYPES = [
  { id: "type-water", name: "Water", sortOrder: 0 },
  { id: "type-electricity", name: "Electricity", sortOrder: 1 },
  { id: "type-rent", name: "Rent", sortOrder: 2 },
  { id: "type-wifi", name: "Wi-Fi", sortOrder: 3 },
  { id: "type-other", name: "Other", sortOrder: 4 },
] as const;

export const DEFAULT_SETTINGS = [
  { key: "currency_code", value: "PHP" },
  { key: "last_backup_at", value: "" },
] as const;

/**
 * Idempotently insert the defaults a fresh database needs. Runs after the
 * schema has been created, both in dev and inside the packaged sidecar.
 */
export async function seedDatabase(db: Database): Promise<void> {
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
