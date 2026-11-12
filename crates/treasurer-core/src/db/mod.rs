use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub mod repository;

pub const DATABASE_FILE: &str = "treasurer.db";

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS bill_types (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  is_archived INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS bill_types_archived_idx ON bill_types (is_archived, sort_order);

CREATE TABLE IF NOT EXISTS boarders (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  archived_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS boarders_status_idx ON boarders (status);

CREATE TABLE IF NOT EXISTS bills (
  id TEXT PRIMARY KEY,
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
);
CREATE INDEX IF NOT EXISTS bills_period_idx ON bills (period_year, period_month);
CREATE INDEX IF NOT EXISTS bills_type_period_idx ON bills (bill_type_id, period_year, period_month);

CREATE TABLE IF NOT EXISTS bill_shares (
  id TEXT PRIMARY KEY,
  bill_id TEXT NOT NULL REFERENCES bills(id) ON DELETE CASCADE,
  boarder_id TEXT NOT NULL REFERENCES boarders(id) ON DELETE RESTRICT,
  boarder_name_snapshot TEXT NOT NULL,
  amount_due_minor INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS bill_shares_bill_boarder_idx ON bill_shares (bill_id, boarder_id);
CREATE INDEX IF NOT EXISTS bill_shares_boarder_idx ON bill_shares (boarder_id);

CREATE TABLE IF NOT EXISTS payments (
  id TEXT PRIMARY KEY,
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
);
CREATE INDEX IF NOT EXISTS payments_share_idx ON payments (bill_share_id);
CREATE INDEX IF NOT EXISTS payments_paid_at_idx ON payments (paid_at);
"#;

pub const DEFAULT_BILL_TYPES: [(&str, &str, i64); 5] = [
    ("type-water", "Water", 0),
    ("type-electricity", "Electricity", 1),
    ("type-rent", "Rent", 2),
    ("type-wifi", "Wi-Fi", 3),
    ("type-other", "Other", 4),
];

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Could not create {}", parent.display()))?;
        }
    }

    let conn = Connection::open(path)
        .with_context(|| format!("Could not open the database at {}", path.display()))?;
    configure(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

pub fn initialize(conn: &Connection) -> Result<()> {
    ensure_schema(conn)?;
    seed(conn)?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// The Expo app already created this database with Drizzle. When the bills
/// table is present the schema is adopted as-is and no DDL is run.
fn ensure_schema(conn: &Connection) -> Result<()> {
    if table_exists(conn, "bills")? {
        return Ok(());
    }

    conn.execute_batch(SCHEMA_SQL)?;
    conn.pragma_update(None, "user_version", 1)?;
    Ok(())
}

fn seed(conn: &Connection) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();

    for (id, name, sort_order) in DEFAULT_BILL_TYPES {
        conn.execute(
            "INSERT INTO bill_types (id, name, is_archived, sort_order, created_at, updated_at)
             VALUES (?1, ?2, 0, ?3, ?4, ?4)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![id, name, sort_order, now],
        )?;
    }

    for (key, value) in [("currency_code", "PHP"), ("last_backup_at", "")] {
        conn.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO NOTHING",
            rusqlite::params![key, value, now],
        )?;
    }

    conn.execute(
        "INSERT INTO settings (key, value, updated_at)
         VALUES ('allow_centavos', 'true', ?1)
         ON CONFLICT(key) DO NOTHING",
        rusqlite::params![now],
    )?;

    Ok(())
}
