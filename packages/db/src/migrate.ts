import { sql } from "drizzle-orm";

import type { Database } from "./index";

/**
 * Startup migrations for the server database.
 *
 * The desktop sidecar ships as a single compiled binary, so migrations cannot
 * be read from `.sql` files at runtime. They are kept here as ordered
 * statements and applied exactly once against the configured `DATABASE_URL`.
 * `drizzle-kit generate` remains the way to produce SQL for external tooling;
 * append a migration here whenever the shared schema changes.
 */
const MIGRATIONS: Array<{ id: string; statements: string[] }> = [
  {
    id: "0001_initial",
    statements: [
      `CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL,
        updated_at INTEGER NOT NULL
      )`,
      `CREATE TABLE IF NOT EXISTS bill_types (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        is_archived INTEGER NOT NULL DEFAULT 0,
        sort_order INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      )`,
      `CREATE TABLE IF NOT EXISTS boarders (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        archived_at INTEGER,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      )`,
      `CREATE TABLE IF NOT EXISTS bills (
        id TEXT PRIMARY KEY NOT NULL,
        bill_type_id TEXT NOT NULL REFERENCES bill_types(id) ON DELETE RESTRICT,
        bill_type_name_snapshot TEXT NOT NULL,
        period_year INTEGER NOT NULL,
        period_month INTEGER NOT NULL,
        total_minor INTEGER NOT NULL,
        due_date TEXT,
        note TEXT,
        archived_at INTEGER,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      )`,
      `CREATE TABLE IF NOT EXISTS bill_shares (
        id TEXT PRIMARY KEY NOT NULL,
        bill_id TEXT NOT NULL REFERENCES bills(id) ON DELETE CASCADE,
        boarder_id TEXT NOT NULL REFERENCES boarders(id) ON DELETE RESTRICT,
        boarder_name_snapshot TEXT NOT NULL,
        amount_due_minor INTEGER NOT NULL,
        created_at INTEGER NOT NULL
      )`,
      `CREATE TABLE IF NOT EXISTS payments (
        id TEXT PRIMARY KEY NOT NULL,
        bill_share_id TEXT NOT NULL REFERENCES bill_shares(id) ON DELETE CASCADE,
        amount_minor INTEGER NOT NULL,
        paid_at INTEGER NOT NULL,
        note TEXT,
        change_minor INTEGER NOT NULL DEFAULT 0,
        change_returned_at INTEGER,
        change_return_note TEXT,
        voided_at INTEGER,
        void_reason TEXT,
        created_at INTEGER NOT NULL
      )`,
      `CREATE INDEX IF NOT EXISTS bill_types_archived_idx ON bill_types (is_archived, sort_order)`,
      `CREATE INDEX IF NOT EXISTS boarders_status_idx ON boarders (status)`,
      `CREATE INDEX IF NOT EXISTS bills_period_idx ON bills (period_year, period_month)`,
      `CREATE INDEX IF NOT EXISTS bills_type_period_idx ON bills (bill_type_id, period_year, period_month)`,
      `CREATE UNIQUE INDEX IF NOT EXISTS bill_shares_bill_boarder_idx ON bill_shares (bill_id, boarder_id)`,
      `CREATE INDEX IF NOT EXISTS bill_shares_boarder_idx ON bill_shares (boarder_id)`,
      `CREATE INDEX IF NOT EXISTS payments_share_idx ON payments (bill_share_id)`,
      `CREATE INDEX IF NOT EXISTS payments_paid_at_idx ON payments (paid_at)`,
    ],
  },
];

export async function migrateDatabase(db: Database): Promise<void> {
  await db.run(
    sql.raw(
      "CREATE TABLE IF NOT EXISTS __app_migrations (id TEXT PRIMARY KEY NOT NULL, applied_at INTEGER NOT NULL)",
    ),
  );

  const appliedRows = (await db.all(
    sql.raw("SELECT id FROM __app_migrations"),
  )) as Array<{ id: string }>;
  const applied = new Set(appliedRows.map((row) => row.id));

  for (const migration of MIGRATIONS) {
    if (applied.has(migration.id)) continue;

    await db.transaction(async (tx) => {
      for (const statement of migration.statements) {
        await tx.run(sql.raw(statement));
      }
      await tx.run(
        sql`INSERT INTO __app_migrations (id, applied_at) VALUES (${migration.id}, ${Date.now()})`,
      );
    });
  }
}
