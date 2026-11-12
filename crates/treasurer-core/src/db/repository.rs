use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::backup::{self, BackupPayload};
use crate::calculations::{
    compute_bill_summary, compute_boarder_period_summaries, compute_share_state,
    list_history_periods, recompute_payment_changes, sort_history_bills, BoarderPeriodSummaryInput,
};
use crate::models::*;
use crate::money::is_currency_code;

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

    // ----- settings -------------------------------------------------------

    pub fn app_settings(&self) -> Result<AppSettings> {
        let currency = self
            .setting("currency_code")?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "PHP".to_string());
        let last_backup_at = self
            .setting("last_backup_at")?
            .filter(|value| !value.is_empty());
        let allow_centavos = self.setting("allow_centavos")?.as_deref() != Some("false");

        Ok(AppSettings {
            currency_code: currency,
            allow_centavos,
            last_backup_at,
        })
    }

    fn setting(&self, key: &str) -> Result<Option<String>> {
        backup::get_setting(&self.conn, key)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now_ms()],
        )?;
        Ok(())
    }

    pub fn set_currency_code(&self, currency_code: &str) -> Result<()> {
        if !is_currency_code(currency_code) {
            bail!("Choose a supported currency.");
        }
        self.set_setting("currency_code", currency_code)
    }

    pub fn set_allow_centavos(&self, allow: bool) -> Result<()> {
        self.set_setting("allow_centavos", if allow { "true" } else { "false" })
    }

    pub fn set_last_backup_at(&self, value: &str) -> Result<()> {
        self.set_setting("last_backup_at", value)
    }

    // ----- bill types -----------------------------------------------------

    pub fn list_bill_types(&self) -> Result<Vec<BillTypeWithCount>> {
        let mut statement = self.conn.prepare(
            "SELECT id, name, is_archived, sort_order, created_at, updated_at
             FROM bill_types ORDER BY sort_order, name",
        )?;
        let types = statement
            .query_map([], map_bill_type)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut counts: HashMap<String, i64> = HashMap::new();
        let mut bills = self.conn.prepare("SELECT bill_type_id FROM bills")?;
        let rows = bills.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            *counts.entry(row?).or_insert(0) += 1;
        }

        Ok(types
            .into_iter()
            .map(|bill_type| BillTypeWithCount {
                bill_count: counts.get(&bill_type.id).copied().unwrap_or(0),
                bill_type,
            })
            .collect())
    }

    fn bill_type(&self, id: &str) -> Result<Option<BillType>> {
        let mut statement = self.conn.prepare(
            "SELECT id, name, is_archived, sort_order, created_at, updated_at
             FROM bill_types WHERE id = ?1",
        )?;
        let mut rows = statement.query_map([id], map_bill_type)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn create_bill_type(&self, name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("Enter a bill type name.");
        }
        if trimmed.chars().count() > 40 {
            bail!("Bill type names can be at most 40 characters.");
        }

        let existing = self.list_bill_types()?;
        if existing.iter().any(|row| {
            !row.bill_type.is_archived && row.bill_type.name.eq_ignore_ascii_case(trimmed)
        }) {
            bail!("An active bill type with that name already exists.");
        }

        let sort_order = existing
            .iter()
            .map(|row| row.bill_type.sort_order)
            .max()
            .unwrap_or(0)
            + 1;
        let id = new_id();
        let now = now_ms();

        self.conn.execute(
            "INSERT INTO bill_types (id, name, is_archived, sort_order, created_at, updated_at)
             VALUES (?1, ?2, 0, ?3, ?4, ?4)",
            params![id, trimmed, sort_order, now],
        )?;
        Ok(id)
    }

    pub fn rename_bill_type(&self, id: &str, name: &str) -> Result<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("Enter a bill type name.");
        }

        let existing = self.list_bill_types()?;
        if existing.iter().any(|row| {
            row.bill_type.id != id
                && !row.bill_type.is_archived
                && row.bill_type.name.eq_ignore_ascii_case(trimmed)
        }) {
            bail!("An active bill type with that name already exists.");
        }

        self.conn.execute(
            "UPDATE bill_types SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![trimmed, now_ms(), id],
        )?;
        Ok(())
    }

    pub fn set_bill_type_archived(&self, id: &str, is_archived: bool) -> Result<()> {
        if !is_archived {
            let existing = self.list_bill_types()?;
            let target = existing
                .iter()
                .find(|row| row.bill_type.id == id)
                .map(|row| row.bill_type.clone());
            let Some(target) = target else {
                bail!("Bill type not found.");
            };
            if existing.iter().any(|row| {
                row.bill_type.id != id
                    && !row.bill_type.is_archived
                    && row.bill_type.name.eq_ignore_ascii_case(&target.name)
            }) {
                bail!("Rename this type before restoring it; another active type uses that name.");
            }
        }

        self.conn.execute(
            "UPDATE bill_types SET is_archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![is_archived as i64, now_ms(), id],
        )?;
        Ok(())
    }

    // ----- boarders -------------------------------------------------------

    fn all_boarders(&self) -> Result<Vec<Boarder>> {
        let mut statement = self.conn.prepare(
            "SELECT id, name, status, archived_at, created_at, updated_at
             FROM boarders ORDER BY name",
        )?;
        let rows = statement
            .query_map([], map_boarder)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn active_boarders(&self) -> Result<Vec<Boarder>> {
        Ok(self
            .all_boarders()?
            .into_iter()
            .filter(|boarder| boarder.status == BoarderStatus::Active)
            .collect())
    }

    pub fn list_boarders_with_balances(&self) -> Result<Vec<BoarderWithBalance>> {
        let boarders = self.all_boarders()?;
        let shares = self.all_shares()?;
        let payments = self.payments_by_share()?;

        let mut totals: HashMap<String, (i64, i64)> = HashMap::new();
        for share in &shares {
            let state = compute_share_state(
                share.amount_due_minor,
                payments.get(&share.id).map(Vec::as_slice).unwrap_or(&[]),
            );
            let entry = totals.entry(share.boarder_id.clone()).or_insert((0, 0));
            entry.0 += state.balance_minor;
            entry.1 += state.change_outstanding_minor;
        }

        Ok(boarders
            .into_iter()
            .map(|boarder| {
                let (outstanding, change) =
                    totals.get(&boarder.id).copied().unwrap_or((0, 0));
                BoarderWithBalance {
                    boarder,
                    outstanding_minor: outstanding,
                    change_outstanding_minor: change,
                }
            })
            .collect())
    }

    pub fn get_boarder(&self, id: &str) -> Result<Option<Boarder>> {
        Ok(self
            .all_boarders()?
            .into_iter()
            .find(|boarder| boarder.id == id))
    }

    pub fn create_boarder(&self, name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("Enter a boarder name.");
        }
        if trimmed.chars().count() > 80 {
            bail!("Boarder names can be at most 80 characters.");
        }

        let id = new_id();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO boarders (id, name, status, archived_at, created_at, updated_at)
             VALUES (?1, ?2, 'active', NULL, ?3, ?3)",
            params![id, trimmed, now],
        )?;
        Ok(id)
    }

    pub fn rename_boarder(&self, id: &str, name: &str) -> Result<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            bail!("Enter a boarder name.");
        }
        self.conn.execute(
            "UPDATE boarders SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![trimmed, now_ms(), id],
        )?;
        Ok(())
    }

    pub fn set_boarder_archived(&self, id: &str, is_archived: bool) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE boarders SET status = ?1, archived_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                if is_archived { "archived" } else { "active" },
                if is_archived { Some(now) } else { None },
                now,
                id
            ],
        )?;
        Ok(())
    }

    // ----- loading helpers ------------------------------------------------

    fn all_shares(&self) -> Result<Vec<BillShare>> {
        let mut statement = self.conn.prepare(
            "SELECT id, bill_id, boarder_id, boarder_name_snapshot, amount_due_minor, created_at
             FROM bill_shares",
        )?;
        let rows = statement
            .query_map([], map_share)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn payments_by_share(&self) -> Result<HashMap<String, Vec<PaymentRecord>>> {
        let mut statement = self.conn.prepare(
            "SELECT id, bill_share_id, amount_minor, paid_at, note, change_minor,
                    change_returned_at, change_return_note, voided_at, void_reason, created_at
             FROM payments",
        )?;
        let payments = statement
            .query_map([], map_payment)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut grouped: HashMap<String, Vec<PaymentRecord>> = HashMap::new();
        for payment in payments {
            grouped
                .entry(payment.bill_share_id.clone())
                .or_default()
                .push(payment);
        }
        Ok(grouped)
    }

    fn all_bills(&self) -> Result<Vec<Bill>> {
        let mut statement = self.conn.prepare(
            "SELECT id, bill_type_id, bill_type_name_snapshot, period_year, period_month,
                    total_minor, due_date, note, archived_at, created_at, updated_at
             FROM bills",
        )?;
        let rows = statement
            .query_map([], map_bill)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn shares_with_states(&self) -> Result<Vec<ShareWithState>> {
        let shares = self.all_shares()?;
        let payments = self.payments_by_share()?;
        let boarders: HashMap<String, Boarder> = self
            .all_boarders()?
            .into_iter()
            .map(|boarder| (boarder.id.clone(), boarder))
            .collect();

        Ok(shares
            .into_iter()
            .map(|share| {
                let boarder = boarders.get(&share.boarder_id);
                ShareWithState {
                    state: compute_share_state(
                        share.amount_due_minor,
                        payments.get(&share.id).map(Vec::as_slice).unwrap_or(&[]),
                    ),
                    current_boarder_name: boarder.map(|boarder| boarder.name.clone()),
                    boarder_archived: boarder
                        .map(|boarder| boarder.status == BoarderStatus::Archived)
                        .unwrap_or(false),
                    share,
                }
            })
            .collect())
    }

    fn bill_with_details(&self, bill: &Bill, shares: &[ShareWithState]) -> BillWithDetails {
        BillWithDetails {
            bill: bill.clone(),
            shares: shares.to_vec(),
            summary: compute_bill_summary(shares, bill.archived_at),
            current_type_name: self
                .bill_type(&bill.bill_type_id)
                .ok()
                .flatten()
                .map(|bill_type| bill_type.name)
                .unwrap_or_else(|| bill.bill_type_name_snapshot.clone()),
            is_locked: shares.iter().any(|share| {
                share
                    .state
                    .payments
                    .iter()
                    .any(|payment| payment.payment.is_active())
            }),
        }
    }

    // ----- bills ----------------------------------------------------------

    pub fn find_duplicate_bills(
        &self,
        bill_type_id: &str,
        period_year: i32,
        period_month: i32,
        exclude_bill_id: Option<&str>,
    ) -> Result<Vec<Bill>> {
        Ok(self
            .all_bills()?
            .into_iter()
            .filter(|bill| {
                bill.bill_type_id == bill_type_id
                    && bill.period_year == period_year
                    && bill.period_month == period_month
                    && Some(bill.id.as_str()) != exclude_bill_id
            })
            .collect())
    }

    pub fn dashboard(
        &self,
        period_year: i32,
        period_month: i32,
        include_archived: bool,
    ) -> Result<DashboardData> {
        let all_bills = self.all_bills()?;
        let shares = self.shares_with_states()?;
        let types = self.list_bill_types()?;
        let sort_order: HashMap<&str, i64> = types
            .iter()
            .map(|row| (row.bill_type.id.as_str(), row.bill_type.sort_order))
            .collect();

        let mut bills: Vec<BillWithDetails> = all_bills
            .iter()
            .filter(|bill| {
                bill.period_year == period_year
                    && bill.period_month == period_month
                    && (include_archived || bill.archived_at.is_none())
            })
            .map(|bill| {
                let bill_shares: Vec<ShareWithState> = shares
                    .iter()
                    .filter(|share| share.share.bill_id == bill.id)
                    .cloned()
                    .collect();
                self.bill_with_details(bill, &bill_shares)
            })
            .collect();

        bills.sort_by(|a, b| {
            let rank = |status: BillStatus| match status {
                BillStatus::Open => 0,
                BillStatus::Settled => 1,
                BillStatus::Archived => 2,
            };
            rank(a.summary.status)
                .cmp(&rank(b.summary.status))
                .then_with(|| {
                    sort_order
                        .get(a.bill.bill_type_id.as_str())
                        .copied()
                        .unwrap_or(0)
                        .cmp(&sort_order.get(b.bill.bill_type_id.as_str()).copied().unwrap_or(0))
                })
                .then(a.bill.created_at.cmp(&b.bill.created_at))
        });

        let active_bills: Vec<&BillWithDetails> = bills
            .iter()
            .filter(|item| item.bill.archived_at.is_none())
            .collect();

        let summaries_input: Vec<BoarderPeriodSummaryInput<'_>> = active_bills
            .iter()
            .flat_map(|item| item.shares.iter())
            .map(|share| BoarderPeriodSummaryInput {
                boarder_id: &share.share.boarder_id,
                boarder_name: share.display_name(),
                boarder_archived: share.boarder_archived,
                amount_due_minor: share.share.amount_due_minor,
                state: &share.state,
            })
            .collect();
        let boarder_summaries = compute_boarder_period_summaries(&summaries_input);

        let totals = active_bills.iter().fold(DashboardTotals::default(), |mut acc, item| {
            acc.expected_minor += item.summary.expected_minor;
            acc.applied_minor += item.summary.applied_minor;
            acc.outstanding_minor += item.summary.outstanding_minor;
            acc.change_outstanding_minor += item.summary.change_outstanding_minor;
            acc.paid_share_count += item.summary.paid_share_count;
            acc.partial_share_count += item.summary.partial_share_count;
            acc.unpaid_share_count += item.summary.unpaid_share_count;
            acc.total_share_count += item.summary.total_share_count;
            acc
        });

        Ok(DashboardData {
            archived_bill_count: all_bills
                .iter()
                .filter(|bill| {
                    bill.period_year == period_year
                        && bill.period_month == period_month
                        && bill.archived_at.is_some()
                })
                .count(),
            bills,
            boarder_summaries,
            totals,
        })
    }

    pub fn bill_detail(&self, id: &str) -> Result<Option<BillWithDetails>> {
        let bills = self.all_bills()?;
        let Some(bill) = bills.iter().find(|bill| bill.id == id) else {
            return Ok(None);
        };
        let shares: Vec<ShareWithState> = self
            .shares_with_states()?
            .into_iter()
            .filter(|share| share.share.bill_id == id)
            .collect();
        Ok(Some(self.bill_with_details(bill, &shares)))
    }

    pub fn bill_history(&self) -> Result<BillHistoryData> {
        let all_bills = self.all_bills()?;
        let shares = self.shares_with_states()?;
        let types = self.list_bill_types()?;
        let type_names: HashMap<&str, &str> = types
            .iter()
            .map(|row| (row.bill_type.id.as_str(), row.bill_type.name.as_str()))
            .collect();

        let mut details: Vec<BillWithDetails> = all_bills
            .iter()
            .map(|bill| {
                let bill_shares: Vec<ShareWithState> = shares
                    .iter()
                    .filter(|share| share.share.bill_id == bill.id)
                    .cloned()
                    .collect();
                self.bill_with_details(bill, &bill_shares)
            })
            .collect();

        let refs: Vec<HistoryBillRef> = details
            .iter()
            .map(|item| HistoryBillRef {
                id: item.bill.id.clone(),
                bill_type_id: item.bill.bill_type_id.clone(),
                period_year: item.bill.period_year,
                period_month: item.bill.period_month,
                created_at: item.bill.created_at,
            })
            .collect();
        let sorted_refs = sort_history_bills(&refs);
        let positions: HashMap<&str, usize> = sorted_refs
            .iter()
            .enumerate()
            .map(|(index, reference)| (reference.id.as_str(), index))
            .collect();
        details.sort_by_key(|item| positions.get(item.bill.id.as_str()).copied().unwrap_or(0));

        let periods = list_history_periods(&refs);

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for bill in &all_bills {
            *counts.entry(bill.bill_type_id.as_str()).or_insert(0) += 1;
        }
        let mut bill_types: Vec<HistoryBillType> = counts
            .into_iter()
            .map(|(id, bill_count)| HistoryBillType {
                id: id.to_string(),
                name: type_names
                    .get(id)
                    .copied()
                    .unwrap_or(id)
                    .to_string(),
                bill_count,
            })
            .collect();
        bill_types.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.name.cmp(&b.name))
        });

        Ok(BillHistoryData {
            bills: details,
            periods,
            bill_types,
        })
    }

    pub fn boarder_ledger(&self, id: &str) -> Result<Option<BoarderLedger>> {
        let Some(boarder) = self.get_boarder(id)? else {
            return Ok(None);
        };

        let bills = self.all_bills()?;
        let bills_by_id: HashMap<&str, &Bill> =
            bills.iter().map(|bill| (bill.id.as_str(), bill)).collect();

        let shares: Vec<ShareWithState> = self
            .shares_with_states()?
            .into_iter()
            .filter(|share| share.share.boarder_id == id)
            .collect();

        let mut items: Vec<BoarderLedgerItem> = shares
            .iter()
            .filter_map(|share| {
                let bill = bills_by_id.get(share.share.bill_id.as_str())?;
                Some(BoarderLedgerItem {
                    bill: (*bill).clone(),
                    share: share.clone(),
                    state: share.state.clone(),
                })
            })
            .collect();
        items.sort_by(|a, b| {
            b.bill
                .period_year
                .cmp(&a.bill.period_year)
                .then(b.bill.period_month.cmp(&a.bill.period_month))
                .then(b.bill.created_at.cmp(&a.bill.created_at))
        });

        let summary = LedgerSummary {
            expected_minor: items.iter().map(|item| item.share.share.amount_due_minor).sum(),
            applied_minor: items.iter().map(|item| item.state.applied_minor).sum(),
            outstanding_minor: items.iter().map(|item| item.state.balance_minor).sum(),
            change_outstanding_minor: items
                .iter()
                .map(|item| item.state.change_outstanding_minor)
                .sum(),
        };

        let mut payments: Vec<BoarderLedgerPayment> = items
            .iter()
            .flat_map(|item| {
                item.state.payments.iter().map(|payment| BoarderLedgerPayment {
                    payment: payment.clone(),
                    bill_id: item.bill.id.clone(),
                    bill_type_name: item.bill.bill_type_name_snapshot.clone(),
                    period_year: item.bill.period_year,
                    period_month: item.bill.period_month,
                })
            })
            .collect();
        payments.sort_by(|a, b| b.payment.payment.paid_at.cmp(&a.payment.payment.paid_at));

        Ok(Some(BoarderLedger {
            boarder,
            items,
            summary,
            payments,
        }))
    }

    pub fn create_bill(&self, input: &CreateBillInput) -> Result<String> {
        if input.total_minor <= 0 {
            bail!("Enter a bill total greater than zero.");
        }
        if input.shares.is_empty() {
            bail!("Select at least one boarder.");
        }

        let share_total: i64 = input.shares.iter().map(|share| share.amount_minor).sum();
        if share_total != input.total_minor {
            bail!("The assigned shares must equal the bill total.");
        }

        let mut seen = std::collections::HashSet::new();
        if !input
            .shares
            .iter()
            .all(|share| seen.insert(share.boarder_id.as_str()))
        {
            bail!("A boarder can only appear once on a bill.");
        }

        let Some(bill_type) = self.bill_type(&input.bill_type_id)? else {
            bail!("Bill type not found.");
        };
        if bill_type.is_archived {
            bail!("That bill type is archived.");
        }

        let boarders: HashMap<String, Boarder> = self
            .all_boarders()?
            .into_iter()
            .map(|boarder| (boarder.id.clone(), boarder))
            .collect();

        for share in &input.shares {
            let Some(boarder) = boarders.get(&share.boarder_id) else {
                bail!("One of the selected boarders no longer exists.");
            };
            if boarder.status != BoarderStatus::Active {
                bail!("Archived boarders cannot be added to a new bill.");
            }
        }

        let now = now_ms();
        let bill_id = new_id();
        let transaction = self.conn.unchecked_transaction()?;

        transaction.execute(
            "INSERT INTO bills (id, bill_type_id, bill_type_name_snapshot, period_year,
                                period_month, total_minor, due_date, note, archived_at,
                                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?9)",
            params![
                bill_id,
                bill_type.id,
                bill_type.name,
                input.period_year,
                input.period_month,
                input.total_minor,
                input.due_date,
                input.note,
                now
            ],
        )?;

        for share in &input.shares {
            let boarder_name = boarders
                .get(&share.boarder_id)
                .map(|boarder| boarder.name.clone())
                .unwrap_or_default();
            transaction.execute(
                "INSERT INTO bill_shares (id, bill_id, boarder_id, boarder_name_snapshot,
                                          amount_due_minor, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_id(),
                    bill_id,
                    share.boarder_id,
                    boarder_name,
                    share.amount_minor,
                    now
                ],
            )?;
        }

        transaction.commit()?;
        Ok(bill_id)
    }

    pub fn update_bill(&self, id: &str, input: &CreateBillInput) -> Result<()> {
        let Some(existing) = self.bill_detail(id)? else {
            bail!("Bill not found.");
        };
        if existing.bill.archived_at.is_some() {
            bail!("Unarchive the bill before editing it.");
        }
        if existing.is_locked {
            bail!("Money fields are locked because payments have been recorded.");
        }
        if input.shares.is_empty() {
            bail!("Select at least one boarder.");
        }
        let share_total: i64 = input.shares.iter().map(|share| share.amount_minor).sum();
        if share_total != input.total_minor {
            bail!("The assigned shares must equal the bill total.");
        }

        let Some(bill_type) = self.bill_type(&input.bill_type_id)? else {
            bail!("Bill type not found.");
        };
        if bill_type.is_archived {
            bail!("That bill type is archived.");
        }

        let boarders: HashMap<String, Boarder> = self
            .all_boarders()?
            .into_iter()
            .map(|boarder| (boarder.id.clone(), boarder))
            .collect();
        let existing_boarders: std::collections::HashSet<&str> = existing
            .shares
            .iter()
            .map(|share| share.share.boarder_id.as_str())
            .collect();
        let old_snapshots: HashMap<&str, &str> = existing
            .shares
            .iter()
            .map(|share| {
                (
                    share.share.boarder_id.as_str(),
                    share.share.boarder_name_snapshot.as_str(),
                )
            })
            .collect();

        for share in &input.shares {
            let Some(boarder) = boarders.get(&share.boarder_id) else {
                bail!("One of the selected boarders no longer exists.");
            };
            if boarder.status != BoarderStatus::Active
                && !existing_boarders.contains(boarder.id.as_str())
            {
                bail!("Archived boarders cannot be added to a bill.");
            }
        }

        let now = now_ms();
        let snapshot = if bill_type.id == existing.bill.bill_type_id {
            existing.bill.bill_type_name_snapshot.clone()
        } else {
            bill_type.name.clone()
        };

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE bills SET bill_type_id = ?1, bill_type_name_snapshot = ?2,
                              period_year = ?3, period_month = ?4, total_minor = ?5,
                              due_date = ?6, note = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                bill_type.id,
                snapshot,
                input.period_year,
                input.period_month,
                input.total_minor,
                input.due_date,
                input.note,
                now,
                id
            ],
        )?;
        transaction.execute("DELETE FROM bill_shares WHERE bill_id = ?1", [id])?;
        for share in &input.shares {
            let boarder_name = old_snapshots
                .get(share.boarder_id.as_str())
                .copied()
                .map(str::to_string)
                .or_else(|| boarders.get(&share.boarder_id).map(|boarder| boarder.name.clone()))
                .unwrap_or_default();
            transaction.execute(
                "INSERT INTO bill_shares (id, bill_id, boarder_id, boarder_name_snapshot,
                                          amount_due_minor, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_id(),
                    id,
                    share.boarder_id,
                    boarder_name,
                    share.amount_minor,
                    now
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn set_bill_archived(&self, id: &str, is_archived: bool) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE bills SET archived_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![if is_archived { Some(now) } else { None }, now, id],
        )?;
        Ok(())
    }

    pub fn update_bill_meta(
        &self,
        id: &str,
        due_date: Option<&str>,
        note: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE bills SET due_date = ?1, note = ?2, updated_at = ?3 WHERE id = ?4",
            params![due_date, note, now_ms(), id],
        )?;
        Ok(())
    }

    // ----- payments -------------------------------------------------------

    fn share_by_id(&self, id: &str) -> Result<Option<BillShare>> {
        Ok(self
            .all_shares()?
            .into_iter()
            .find(|share| share.id == id))
    }

    fn payments_for_share(&self, share_id: &str) -> Result<Vec<PaymentRecord>> {
        let mut grouped = self.payments_by_share()?;
        Ok(grouped.remove(share_id).unwrap_or_default())
    }

    pub fn share_payment_context(&self, share_id: &str) -> Result<Option<SharePaymentContext>> {
        let Some(share) = self.share_by_id(share_id)? else {
            return Ok(None);
        };
        let Some(bill) = self
            .all_bills()?
            .into_iter()
            .find(|bill| bill.id == share.bill_id)
        else {
            return Ok(None);
        };
        let boarder = self.get_boarder(&share.boarder_id)?;
        let state = compute_share_state(
            share.amount_due_minor,
            &self.payments_for_share(share_id)?,
        );

        Ok(Some(SharePaymentContext {
            boarder_name: share.boarder_name_snapshot.clone(),
            share: ShareWithState {
                current_boarder_name: boarder.as_ref().map(|boarder| boarder.name.clone()),
                boarder_archived: boarder
                    .as_ref()
                    .map(|boarder| boarder.status == BoarderStatus::Archived)
                    .unwrap_or(false),
                share,
                state,
            },
            bill,
        }))
    }

    pub fn record_payment(&self, input: &CreatePaymentInput) -> Result<String> {
        if input.amount_minor <= 0 {
            bail!("Enter a payment amount greater than zero.");
        }

        let Some(share) = self.share_by_id(&input.bill_share_id)? else {
            bail!("Bill share not found.");
        };

        let payment_id = new_id();
        let now = now_ms();
        let draft = PaymentRecord {
            id: payment_id.clone(),
            bill_share_id: input.bill_share_id.clone(),
            amount_minor: input.amount_minor,
            paid_at: input.paid_at,
            note: input.note.clone(),
            change_minor: 0,
            change_returned_at: None,
            change_return_note: None,
            voided_at: None,
            void_reason: None,
            created_at: now,
        };

        let mut payments = self.payments_for_share(&input.bill_share_id)?;
        payments.push(draft);
        let computed = recompute_payment_changes(share.amount_due_minor, &payments);
        let Some(new_payment) = computed.iter().find(|item| item.payment.id == payment_id) else {
            bail!("The payment could not be calculated.");
        };
        let change_minor = new_payment.computed_change_minor;

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO payments (id, bill_share_id, amount_minor, paid_at, note, change_minor,
                                   change_returned_at, change_return_note, voided_at, void_reason,
                                   created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL, NULL, ?7)",
            params![
                payment_id,
                input.bill_share_id,
                input.amount_minor,
                input.paid_at,
                input.note,
                change_minor,
                now
            ],
        )?;

        for payment in &computed {
            if payment.payment.id == payment_id || !payment.payment.is_active() {
                continue;
            }
            if payment.payment.change_minor != payment.computed_change_minor {
                transaction.execute(
                    "UPDATE payments SET change_minor = ?1 WHERE id = ?2",
                    params![payment.computed_change_minor, payment.payment.id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(payment_id)
    }

    pub fn void_payment(&self, payment_id: &str, reason: &str) -> Result<()> {
        let trimmed = reason.trim();
        if trimmed.is_empty() {
            bail!("Enter a reason for voiding this payment.");
        }

        let payments = self.payments_by_share()?;
        let Some(payment) = payments
            .values()
            .flatten()
            .find(|payment| payment.id == payment_id)
            .cloned()
        else {
            bail!("Payment not found.");
        };
        if payment.voided_at.is_some() {
            bail!("This payment is already voided.");
        }

        let Some(share) = self.share_by_id(&payment.bill_share_id)? else {
            bail!("Bill share not found.");
        };

        let now = now_ms();
        let mut updated: Vec<PaymentRecord> = payments
            .get(&payment.bill_share_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                if item.id == payment_id {
                    PaymentRecord {
                        voided_at: Some(now),
                        void_reason: Some(trimmed.to_string()),
                        ..item
                    }
                } else {
                    item
                }
            })
            .collect();
        if !updated.iter().any(|item| item.id == payment_id) {
            updated.push(payment.clone());
        }

        let computed = recompute_payment_changes(share.amount_due_minor, &updated);

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE payments SET voided_at = ?1, void_reason = ?2 WHERE id = ?3",
            params![now, trimmed, payment_id],
        )?;
        for item in &computed {
            if !item.payment.is_active() {
                continue;
            }
            if item.payment.change_minor != item.computed_change_minor {
                transaction.execute(
                    "UPDATE payments SET change_minor = ?1 WHERE id = ?2",
                    params![item.computed_change_minor, item.payment.id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn mark_change_returned(
        &self,
        payment_id: &str,
        returned_at: i64,
        note: Option<&str>,
    ) -> Result<()> {
        let payments = self.payments_by_share()?;
        let Some(payment) = payments
            .values()
            .flatten()
            .find(|payment| payment.id == payment_id)
            .cloned()
        else {
            bail!("Payment not found.");
        };
        if payment.voided_at.is_some() {
            bail!("Voided payments cannot return change.");
        }
        if payment.change_returned_at.is_some() {
            bail!("This change was already marked returned.");
        }

        let Some(share) = self.share_by_id(&payment.bill_share_id)? else {
            bail!("Bill share not found.");
        };
        let computed = recompute_payment_changes(
            share.amount_due_minor,
            payments
                .get(&payment.bill_share_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        let Some(current) = computed.iter().find(|item| item.payment.id == payment_id) else {
            bail!("There is no change to return for this payment.");
        };
        if current.computed_change_minor <= 0 {
            bail!("There is no change to return for this payment.");
        }

        self.conn.execute(
            "UPDATE payments SET change_minor = ?1, change_returned_at = ?2, change_return_note = ?3
             WHERE id = ?4",
            params![current.computed_change_minor, returned_at, note, payment_id],
        )?;
        Ok(())
    }

    // ----- backups --------------------------------------------------------

    pub fn create_backup(&self) -> Result<BackupPayload> {
        backup::create_backup_payload(&self.conn)
    }

    pub fn export_backup_json(&self) -> Result<String> {
        backup::export_backup_json(&self.conn)
    }

    pub fn import_backup_json(&self, json: &str) -> Result<()> {
        let payload = backup::parse_backup_json(json)?;
        backup::restore_backup(&self.conn, payload)
    }
}
