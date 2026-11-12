use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, Row};
use uuid::Uuid;

use crate::models::*;

pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub struct SharePaymentContext {
    pub share: ShareWithState,
    pub bill: Bill,
    pub boarder_name: String,
}

pub struct Treasurer {
    pub conn: Connection,
}

fn map_bill_type(row: &Row<'_>) -> rusqlite::Result<BillType> {
    Ok(BillType {
        id: row.get(0)?,
        name: row.get(1)?,
        is_archived: row.get::<_, i64>(2)? != 0,
        sort_order: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn map_boarder(row: &Row<'_>) -> rusqlite::Result<Boarder> {
    let status: String = row.get(2)?;
    Ok(Boarder {
        id: row.get(0)?,
        name: row.get(1)?,
        status: if status == "archived" {
            BoarderStatus::Archived
        } else {
            BoarderStatus::Active
        },
        archived_at: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn map_bill(row: &Row<'_>) -> rusqlite::Result<Bill> {
    Ok(Bill {
        id: row.get(0)?,
        bill_type_id: row.get(1)?,
        bill_type_name_snapshot: row.get(2)?,
        period_year: row.get(3)?,
        period_month: row.get(4)?,
        total_minor: row.get(5)?,
        due_date: row.get(6)?,
        note: row.get(7)?,
        archived_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_share(row: &Row<'_>) -> rusqlite::Result<BillShare> {
    Ok(BillShare {
        id: row.get(0)?,
        bill_id: row.get(1)?,
        boarder_id: row.get(2)?,
        boarder_name_snapshot: row.get(3)?,
        amount_due_minor: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn map_payment(row: &Row<'_>) -> rusqlite::Result<PaymentRecord> {
    Ok(PaymentRecord {
        id: row.get(0)?,
        bill_share_id: row.get(1)?,
        amount_minor: row.get(2)?,
        paid_at: row.get(3)?,
        note: row.get(4)?,
        change_minor: row.get(5)?,
        change_returned_at: row.get(6)?,
        change_return_note: row.get(7)?,
        voided_at: row.get(8)?,
        void_reason: row.get(9)?,
        created_at: row.get(10)?,
    })
}

impl Treasurer {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = super::open(path)?;
        super::initialize(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = super::open_in_memory()?;
        super::initialize(&conn)?;
        Ok(Self { conn })
    }
}

mod backups;
mod bill_types;
mod bills;
mod boarders;
mod loading;
mod payments;
mod reports;
mod settings;
