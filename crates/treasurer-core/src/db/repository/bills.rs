use std::collections::HashMap;

use anyhow::{bail, Result};
use rusqlite::params;

use crate::models::*;

use super::{new_id, now_ms};
use super::Treasurer;

impl Treasurer {
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
}
