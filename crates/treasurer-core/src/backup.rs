use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::calculations::recompute_payment_changes;
use crate::models::PaymentRecord;
use crate::money::MAX_AMOUNT_MINOR;

pub const MAX_BACKUP_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub currency_code: String,
    /// Optional so version 1 backups written before this setting still import.
    pub allow_centavos: Option<bool>,
    pub last_backup_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBillType {
    pub id: String,
    pub name: String,
    pub is_archived: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBoarder {
    pub id: String,
    pub name: String,
    pub status: String,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBill {
    pub id: String,
    pub bill_type_id: String,
    pub bill_type_name_snapshot: String,
    pub period_year: i32,
    pub period_month: i32,
    pub total_minor: i64,
    pub due_date: Option<String>,
    pub note: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBillShare {
    pub id: String,
    pub bill_id: String,
    pub boarder_id: String,
    pub boarder_name_snapshot: String,
    pub amount_due_minor: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPayment {
    pub id: String,
    pub bill_share_id: String,
    pub amount_minor: i64,
    pub paid_at: String,
    pub note: Option<String>,
    pub change_minor: i64,
    pub change_returned_at: Option<String>,
    pub change_return_note: Option<String>,
    pub voided_at: Option<String>,
    pub void_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupData {
    pub settings: BackupSettings,
    pub bill_types: Vec<BackupBillType>,
    pub boarders: Vec<BackupBoarder>,
    pub bills: Vec<BackupBill>,
    pub bill_shares: Vec<BackupBillShare>,
    pub payments: Vec<BackupPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPayload {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub data: BackupData,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BackupCounts {
    pub bill_types: usize,
    pub boarders: usize,
    pub bills: usize,
    pub shares: usize,
    pub payments: usize,
}

pub fn backup_counts(payload: &BackupPayload) -> BackupCounts {
    BackupCounts {
        bill_types: payload.data.bill_types.len(),
        boarders: payload.data.boarders.len(),
        bills: payload.data.bills.len(),
        shares: payload.data.bill_shares.len(),
        payments: payload.data.payments.len(),
    }
}

fn to_iso(epoch_ms: Option<i64>) -> Option<String> {
    epoch_ms.map(|ms| {
        DateTime::<Utc>::from_timestamp_millis(ms)
            .map(|date| date.to_rfc3339_opts(SecondsFormat::Millis, true))
            .unwrap_or_default()
    })
}

fn to_ms(iso: &str) -> Result<i64> {
    let parsed = DateTime::parse_from_rfc3339(iso)
        .with_context(|| format!("Invalid timestamp: {iso}"))?;
    Ok(parsed.timestamp_millis())
}

pub fn create_backup_payload(conn: &Connection) -> Result<BackupPayload> {
    let mut settings_map = std::collections::HashMap::new();
    {
        let mut statement = conn.prepare("SELECT key, value FROM settings")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (key, value) = row?;
            settings_map.insert(key, value);
        }
    }

    let last_backup_at = settings_map
        .get("last_backup_at")
        .filter(|value| !value.is_empty())
        .cloned();

    let bill_types = {
        let mut statement = conn.prepare(
            "SELECT id, name, is_archived, sort_order, created_at, updated_at FROM bill_types ORDER BY sort_order, name",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(BackupBillType {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    is_archived: row.get::<_, i64>(2)? != 0,
                    sort_order: row.get(3)?,
                    created_at: to_iso(Some(row.get(4)?)).unwrap_or_default(),
                    updated_at: to_iso(Some(row.get(5)?)).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let boarders = {
        let mut statement = conn.prepare(
            "SELECT id, name, status, archived_at, created_at, updated_at FROM boarders ORDER BY name",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(BackupBoarder {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    archived_at: to_iso(row.get(3)?),
                    created_at: to_iso(Some(row.get(4)?)).unwrap_or_default(),
                    updated_at: to_iso(Some(row.get(5)?)).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let bills = {
        let mut statement = conn.prepare(
            "SELECT id, bill_type_id, bill_type_name_snapshot, period_year, period_month,
                    total_minor, due_date, note, archived_at, created_at, updated_at
             FROM bills",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(BackupBill {
                    id: row.get(0)?,
                    bill_type_id: row.get(1)?,
                    bill_type_name_snapshot: row.get(2)?,
                    period_year: row.get(3)?,
                    period_month: row.get(4)?,
                    total_minor: row.get(5)?,
                    due_date: row.get(6)?,
                    note: row.get(7)?,
                    archived_at: to_iso(row.get(8)?),
                    created_at: to_iso(Some(row.get(9)?)).unwrap_or_default(),
                    updated_at: to_iso(Some(row.get(10)?)).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let bill_shares = {
        let mut statement = conn.prepare(
            "SELECT id, bill_id, boarder_id, boarder_name_snapshot, amount_due_minor, created_at
             FROM bill_shares",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(BackupBillShare {
                    id: row.get(0)?,
                    bill_id: row.get(1)?,
                    boarder_id: row.get(2)?,
                    boarder_name_snapshot: row.get(3)?,
                    amount_due_minor: row.get(4)?,
                    created_at: to_iso(Some(row.get(5)?)).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let payments = {
        let mut statement = conn.prepare(
            "SELECT id, bill_share_id, amount_minor, paid_at, note, change_minor,
                    change_returned_at, change_return_note, voided_at, void_reason, created_at
             FROM payments",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(BackupPayment {
                    id: row.get(0)?,
                    bill_share_id: row.get(1)?,
                    amount_minor: row.get(2)?,
                    paid_at: to_iso(Some(row.get(3)?)).unwrap_or_default(),
                    note: row.get(4)?,
                    change_minor: row.get(5)?,
                    change_returned_at: to_iso(row.get(6)?),
                    change_return_note: row.get(7)?,
                    voided_at: to_iso(row.get(8)?),
                    void_reason: row.get(9)?,
                    created_at: to_iso(Some(row.get(10)?)).unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    Ok(BackupPayload {
        format: "boarding-house-treasurer".to_string(),
        version: 1,
        exported_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        data: BackupData {
            settings: BackupSettings {
                currency_code: settings_map
                    .get("currency_code")
                    .cloned()
                    .unwrap_or_else(|| "PHP".to_string()),
                allow_centavos: Some(
                    settings_map.get("allow_centavos").map(|v| v != "false").unwrap_or(true),
                ),
                last_backup_at,
            },
            bill_types,
            boarders,
            bills,
            bill_shares,
            payments,
        },
    })
}

pub fn export_backup_json(conn: &Connection) -> Result<String> {
    let payload = create_backup_payload(conn)?;
    Ok(serde_json::to_string_pretty(&payload)?)
}

pub fn parse_backup_json(json: &str) -> Result<BackupPayload> {
    if json.len() > MAX_BACKUP_BYTES {
        bail!("That backup is too large. Choose a file under 10 MB.");
    }

    let parsed: BackupPayload = serde_json::from_str(json)
        .map_err(|_| anyhow::anyhow!("This file is not a valid boarding house backup."))?;

    validate_backup(&parsed)?;
    Ok(parsed)
}

fn timestamp_is_valid(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
        && NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

fn check_name(value: &str, max: usize, message: &str) -> Result<()> {
    if value.is_empty() || value.chars().count() > max {
        bail!("{message}");
    }
    Ok(())
}

fn check_optional_text(value: &Option<String>, max: usize) -> Result<()> {
    if let Some(value) = value {
        if value.chars().count() > max {
            bail!("This file is not a valid boarding house backup.");
        }
    }
    Ok(())
}

fn ensure_unique(values: &[String], label: &str) -> Result<()> {
    let unique: HashSet<&String> = values.iter().collect();
    if unique.len() != values.len() {
        bail!("The backup contains duplicate {label}.");
    }
    Ok(())
}

pub fn validate_backup(payload: &BackupPayload) -> Result<()> {
    if payload.format != "boarding-house-treasurer" || payload.version != 1 {
        bail!("This file is not a valid boarding house backup.");
    }
    if !timestamp_is_valid(&payload.exported_at) {
        bail!("This file is not a valid boarding house backup.");
    }

    let data = &payload.data;

    check_name(&data.settings.currency_code, 3, "This file is not a valid boarding house backup.")?;
    if data.settings.currency_code.chars().count() != 3 {
        bail!("This file is not a valid boarding house backup.");
    }
    if let Some(last_backup_at) = &data.settings.last_backup_at {
        if !timestamp_is_valid(last_backup_at) {
            bail!("This file is not a valid boarding house backup.");
        }
    }

    ensure_unique(
        &data.bill_types.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        "bill type IDs",
    )?;
    ensure_unique(
        &data.boarders.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        "boarder IDs",
    )?;
    ensure_unique(
        &data.bills.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        "bill IDs",
    )?;
    ensure_unique(
        &data.bill_shares.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        "share IDs",
    )?;
    ensure_unique(
        &data.payments.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        "payment IDs",
    )?;

    for bill_type in &data.bill_types {
        if bill_type.id.is_empty() {
            bail!("This file is not a valid boarding house backup.");
        }
        check_name(&bill_type.name, 40, "This file is not a valid boarding house backup.")?;
        if !timestamp_is_valid(&bill_type.created_at) || !timestamp_is_valid(&bill_type.updated_at) {
            bail!("This file is not a valid boarding house backup.");
        }
    }

    for boarder in &data.boarders {
        if boarder.id.is_empty() || (boarder.status != "active" && boarder.status != "archived") {
            bail!("This file is not a valid boarding house backup.");
        }
        check_name(&boarder.name, 80, "This file is not a valid boarding house backup.")?;
        if !timestamp_is_valid(&boarder.created_at) || !timestamp_is_valid(&boarder.updated_at) {
            bail!("This file is not a valid boarding house backup.");
        }
        if let Some(archived_at) = &boarder.archived_at {
            if !timestamp_is_valid(archived_at) {
                bail!("This file is not a valid boarding house backup.");
            }
        }
    }

    let type_ids: HashSet<&String> = data.bill_types.iter().map(|row| &row.id).collect();
    let boarder_ids: HashSet<&String> = data.boarders.iter().map(|row| &row.id).collect();
    let bill_ids: HashSet<&String> = data.bills.iter().map(|row| &row.id).collect();
    let share_ids: HashSet<&String> = data.bill_shares.iter().map(|row| &row.id).collect();

    for bill in &data.bills {
        if !(2000..=2100).contains(&bill.period_year) || !(1..=12).contains(&bill.period_month) {
            bail!("This file is not a valid boarding house backup.");
        }
        if bill.total_minor < 1 || bill.total_minor > MAX_AMOUNT_MINOR {
            bail!("This file is not a valid boarding house backup.");
        }
        check_name(
            &bill.bill_type_name_snapshot,
            40,
            "This file is not a valid boarding house backup.",
        )?;
        if let Some(due_date) = &bill.due_date {
            if !valid_iso_date(due_date) {
                bail!("This file is not a valid boarding house backup.");
            }
        }
        check_optional_text(&bill.note, 500)?;
        if !timestamp_is_valid(&bill.created_at) || !timestamp_is_valid(&bill.updated_at) {
            bail!("This file is not a valid boarding house backup.");
        }
        if let Some(archived_at) = &bill.archived_at {
            if !timestamp_is_valid(archived_at) {
                bail!("This file is not a valid boarding house backup.");
            }
        }
        if !type_ids.contains(&bill.bill_type_id) {
            bail!("The backup contains a bill with a missing bill type.");
        }
    }

    let mut share_keys = HashSet::new();
    for share in &data.bill_shares {
        if share.amount_due_minor < 0 || share.amount_due_minor > MAX_AMOUNT_MINOR {
            bail!("This file is not a valid boarding house backup.");
        }
        if !bill_ids.contains(&share.bill_id) {
            bail!("The backup contains a share with a missing bill.");
        }
        if !boarder_ids.contains(&share.boarder_id) {
            bail!("The backup contains a share with a missing boarder.");
        }
        if !share_keys.insert((share.bill_id.clone(), share.boarder_id.clone())) {
            bail!("The backup contains a duplicate boarder share.");
        }
        check_name(
            &share.boarder_name_snapshot,
            80,
            "This file is not a valid boarding house backup.",
        )?;
        if !timestamp_is_valid(&share.created_at) {
            bail!("This file is not a valid boarding house backup.");
        }
    }

    for bill in &data.bills {
        let assigned: i64 = data
            .bill_shares
            .iter()
            .filter(|share| share.bill_id == bill.id)
            .map(|share| share.amount_due_minor)
            .sum();
        if assigned != bill.total_minor {
            bail!("The backup contains a bill whose shares do not equal its total.");
        }
    }

    for payment in &data.payments {
        if payment.amount_minor < 1 || payment.amount_minor > MAX_AMOUNT_MINOR {
            bail!("This file is not a valid boarding house backup.");
        }
        if payment.change_minor < 0 || payment.change_minor > MAX_AMOUNT_MINOR {
            bail!("This file is not a valid boarding house backup.");
        }
        if !share_ids.contains(&payment.bill_share_id) {
            bail!("The backup contains a payment with a missing share.");
        }
        if !timestamp_is_valid(&payment.paid_at) || !timestamp_is_valid(&payment.created_at) {
            bail!("This file is not a valid boarding house backup.");
        }
        for timestamp in [
            &payment.change_returned_at,
            &payment.voided_at,
        ]
        .into_iter()
        .flatten()
        {
            if !timestamp_is_valid(timestamp) {
                bail!("This file is not a valid boarding house backup.");
            }
        }
        check_optional_text(&payment.note, 500)?;
        check_optional_text(&payment.change_return_note, 200)?;
        check_optional_text(&payment.void_reason, 200)?;
    }

    Ok(())
}

pub fn restore_backup(conn: &Connection, payload: BackupPayload) -> Result<()> {
    validate_backup(&payload)?;
    let data = payload.data;

    let mut payments_by_share: std::collections::HashMap<String, Vec<PaymentRecord>> =
        std::collections::HashMap::new();
    for payment in &data.payments {
        let record = PaymentRecord {
            id: payment.id.clone(),
            bill_share_id: payment.bill_share_id.clone(),
            amount_minor: payment.amount_minor,
            paid_at: to_ms(&payment.paid_at)?,
            note: payment.note.clone(),
            change_minor: payment.change_minor,
            change_returned_at: payment.change_returned_at.as_deref().map(to_ms).transpose()?,
            change_return_note: payment.change_return_note.clone(),
            voided_at: payment.voided_at.as_deref().map(to_ms).transpose()?,
            void_reason: payment.void_reason.clone(),
            created_at: to_ms(&payment.created_at)?,
        };
        payments_by_share
            .entry(record.bill_share_id.clone())
            .or_default()
            .push(record);
    }

    let mut computed_payments: Vec<(String, i64)> = Vec::new();
    for share in &data.bill_shares {
        let share_payments = payments_by_share
            .get(&share.id)
            .cloned()
            .unwrap_or_default();
        for payment in recompute_payment_changes(share.amount_due_minor, &share_payments) {
            computed_payments.push((payment.payment.id.clone(), payment.computed_change_minor));
        }
    }
    let computed_by_id: std::collections::HashMap<String, i64> =
        computed_payments.into_iter().collect();

    let transaction = conn.unchecked_transaction()?;

    transaction.execute("DELETE FROM payments", [])?;
    transaction.execute("DELETE FROM bill_shares", [])?;
    transaction.execute("DELETE FROM bills", [])?;
    transaction.execute("DELETE FROM boarders", [])?;
    transaction.execute("DELETE FROM bill_types", [])?;
    transaction.execute("DELETE FROM settings", [])?;

    let now = Utc::now().timestamp_millis();
    for (key, value) in [
        ("currency_code", data.settings.currency_code.clone()),
        (
            "last_backup_at",
            data.settings.last_backup_at.clone().unwrap_or_default(),
        ),
        (
            "allow_centavos",
            if data.settings.allow_centavos.unwrap_or(true) {
                "true".to_string()
            } else {
                "false".to_string()
            },
        ),
    ] {
        transaction.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
    }

    for bill_type in &data.bill_types {
        transaction.execute(
            "INSERT INTO bill_types (id, name, is_archived, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                bill_type.id,
                bill_type.name,
                bill_type.is_archived as i64,
                bill_type.sort_order,
                to_ms(&bill_type.created_at)?,
                to_ms(&bill_type.updated_at)?,
            ],
        )?;
    }

    for boarder in &data.boarders {
        transaction.execute(
            "INSERT INTO boarders (id, name, status, archived_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                boarder.id,
                boarder.name,
                boarder.status,
                boarder.archived_at.as_deref().map(to_ms).transpose()?,
                to_ms(&boarder.created_at)?,
                to_ms(&boarder.updated_at)?,
            ],
        )?;
    }

    for bill in &data.bills {
        transaction.execute(
            "INSERT INTO bills (id, bill_type_id, bill_type_name_snapshot, period_year,
                                period_month, total_minor, due_date, note, archived_at,
                                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                bill.id,
                bill.bill_type_id,
                bill.bill_type_name_snapshot,
                bill.period_year,
                bill.period_month,
                bill.total_minor,
                bill.due_date,
                bill.note,
                bill.archived_at.as_deref().map(to_ms).transpose()?,
                to_ms(&bill.created_at)?,
                to_ms(&bill.updated_at)?,
            ],
        )?;
    }

    for share in &data.bill_shares {
        transaction.execute(
            "INSERT INTO bill_shares (id, bill_id, boarder_id, boarder_name_snapshot,
                                      amount_due_minor, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                share.id,
                share.bill_id,
                share.boarder_id,
                share.boarder_name_snapshot,
                share.amount_due_minor,
                to_ms(&share.created_at)?,
            ],
        )?;
    }

    for payment in &data.payments {
        transaction.execute(
            "INSERT INTO payments (id, bill_share_id, amount_minor, paid_at, note, change_minor,
                                   change_returned_at, change_return_note, voided_at, void_reason,
                                   created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                payment.id,
                payment.bill_share_id,
                payment.amount_minor,
                to_ms(&payment.paid_at)?,
                payment.note,
                computed_by_id.get(&payment.id).copied().unwrap_or(0),
                payment.change_returned_at.as_deref().map(to_ms).transpose()?,
                payment.change_return_note,
                payment.voided_at.as_deref().map(to_ms).transpose()?,
                payment.void_reason,
                to_ms(&payment.created_at)?,
            ],
        )?;
    }

    transaction.commit()?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let value = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    Ok(value)
}
