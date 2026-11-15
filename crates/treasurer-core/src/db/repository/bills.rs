use std::collections::HashMap;

use anyhow::{bail, Result};
use rusqlite::params;

use crate::calculations::{
    equal_split, unique_boarder_ids, validate_additional_charge, SplitOutcome,
};
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
                                          amount_due_minor, created_at, additional_charge_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
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
        // Update existing shares in place so their IDs, snapshots, creation
        // times, and manual charges survive the edit.
        let existing_shares: HashMap<&str, &BillShare> = existing
            .shares
            .iter()
            .map(|share| (share.share.boarder_id.as_str(), &share.share))
            .collect();
        let keep: std::collections::HashSet<&str> = input
            .shares
            .iter()
            .map(|share| share.boarder_id.as_str())
            .collect();

        for share in &input.shares {
            if existing_shares.contains_key(share.boarder_id.as_str()) {
                transaction.execute(
                    "UPDATE bill_shares SET amount_due_minor = ?1
                     WHERE bill_id = ?2 AND boarder_id = ?3",
                    params![share.amount_minor, id, share.boarder_id],
                )?;
                continue;
            }

            let boarder_name = old_snapshots
                .get(share.boarder_id.as_str())
                .copied()
                .map(str::to_string)
                .or_else(|| boarders.get(&share.boarder_id).map(|boarder| boarder.name.clone()))
                .unwrap_or_default();
            transaction.execute(
                "INSERT INTO bill_shares (id, bill_id, boarder_id, boarder_name_snapshot,
                                          amount_due_minor, created_at, additional_charge_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
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
        for boarder_id in existing_shares.keys() {
            if !keep.contains(boarder_id) {
                transaction.execute(
                    "DELETE FROM bill_shares WHERE bill_id = ?1 AND boarder_id = ?2",
                    params![id, boarder_id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Saves one share-edit operation: the new base split for the whole bill
    /// plus the edited boarder's additional charge. Atomic.
    pub fn save_share_edit(
        &self,
        bill_id: &str,
        split: &[SplitResult],
        edited_boarder_id: &str,
        charge_minor: i64,
    ) -> Result<()> {
        let Some(detail) = self.bill_detail(bill_id)? else {
            bail!("Bill not found.");
        };

        let allow_centavos = self.app_settings()?.allow_centavos;
        if let Some(error) = validate_additional_charge(charge_minor, allow_centavos) {
            bail!("{error}");
        }

        let share_total: i64 = split.iter().map(|share| share.amount_minor).sum();
        if split.is_empty()
            || !unique_boarder_ids(split)
            || split.iter().any(|share| share.amount_minor < 0)
            || share_total != detail.bill.total_minor
        {
            bail!("The assigned shares must equal the bill total.");
        }

        let existing: HashMap<&str, &BillShare> = detail
            .shares
            .iter()
            .map(|share| (share.share.boarder_id.as_str(), &share.share))
            .collect();
        if split.len() != existing.len()
            || !split
                .iter()
                .all(|share| existing.contains_key(share.boarder_id.as_str()))
            || !existing.contains_key(edited_boarder_id)
        {
            bail!("The assigned shares must equal the bill total.");
        }

        let changed_amounts = split.iter().any(|share| {
            existing
                .get(share.boarder_id.as_str())
                .map(|row| row.amount_due_minor != share.amount_minor)
                .unwrap_or(false)
        });
        if changed_amounts && detail.is_locked {
            bail!("Share amounts are locked because payments have been recorded. You can still change additional charges.");
        }

        let charge_changed = existing
            .get(edited_boarder_id)
            .map(|row| row.additional_charge_minor != charge_minor)
            .unwrap_or(false);

        let now = now_ms();
        let transaction = self.conn.unchecked_transaction()?;
        if changed_amounts {
            for share in split {
                let changed = existing
                    .get(share.boarder_id.as_str())
                    .map(|row| row.amount_due_minor != share.amount_minor)
                    .unwrap_or(false);
                if changed {
                    transaction.execute(
                        "UPDATE bill_shares SET amount_due_minor = ?1
                         WHERE bill_id = ?2 AND boarder_id = ?3",
                        params![share.amount_minor, bill_id, share.boarder_id],
                    )?;
                }
            }
        }
        transaction.execute(
            "UPDATE bill_shares SET additional_charge_minor = ?1
             WHERE bill_id = ?2 AND boarder_id = ?3",
            params![charge_minor, bill_id, edited_boarder_id],
        )?;
        if changed_amounts || charge_changed {
            transaction.execute(
                "UPDATE bills SET updated_at = ?1 WHERE id = ?2",
                params![now, bill_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Removes one boarder from a bill and re-splits the bill total equally
    /// among the remaining shares. The bill total never changes.
    pub fn exclude_share(&self, bill_id: &str, boarder_id: &str) -> Result<()> {
        let Some(detail) = self.bill_detail(bill_id)? else {
            bail!("Bill not found.");
        };
        if detail.is_locked {
            bail!("Share amounts are locked because payments have been recorded. Void the payments first to change who is on this bill.");
        }
        if detail.shares.len() <= 1 {
            bail!("A bill needs at least one boarder. Archive the bill instead.");
        }
        let Some(target) = detail
            .shares
            .iter()
            .find(|share| share.share.boarder_id == boarder_id)
        else {
            bail!("That boarder is not on this bill.");
        };
        if target
            .state
            .payments
            .iter()
            .any(|payment| payment.payment.is_active())
        {
            bail!("Void this boarder's payment before excluding them.");
        }

        let allow_centavos = self.app_settings()?.allow_centavos;
        let remaining: Vec<SplitBoarder> = detail
            .shares
            .iter()
            .filter(|share| share.share.boarder_id != boarder_id)
            .map(|share| SplitBoarder {
                boarder_id: share.share.boarder_id.clone(),
                name: share.display_name().to_string(),
                created_at: share.share.created_at,
            })
            .collect();
        let split = match equal_split(detail.bill.total_minor, &remaining, allow_centavos) {
            SplitOutcome::Ok(success) => success.shares,
            SplitOutcome::Err(failure) => bail!("{}", failure.error),
        };

        let now = now_ms();
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM bill_shares WHERE bill_id = ?1 AND boarder_id = ?2",
            params![bill_id, boarder_id],
        )?;
        for share in &split {
            transaction.execute(
                "UPDATE bill_shares SET amount_due_minor = ?1
                 WHERE bill_id = ?2 AND boarder_id = ?3",
                params![share.amount_minor, bill_id, share.boarder_id],
            )?;
        }
        transaction.execute(
            "UPDATE bills SET updated_at = ?1 WHERE id = ?2",
            params![now, bill_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Adds an active boarder who is not on the bill yet and re-splits the bill
    /// total equally across everyone. Existing additional charges are kept.
    pub fn add_share_to_bill(&self, bill_id: &str, boarder_id: &str) -> Result<()> {
        let Some(detail) = self.bill_detail(bill_id)? else {
            bail!("Bill not found.");
        };
        if detail.is_locked {
            bail!("Share amounts are locked because payments have been recorded. Void the payments first to change who is on this bill.");
        }
        let Some(boarder) = self.get_boarder(boarder_id)? else {
            bail!("Boarder not found.");
        };
        if boarder.status == BoarderStatus::Archived {
            bail!("Archived boarders cannot be added to a bill.");
        }
        if detail
            .shares
            .iter()
            .any(|share| share.share.boarder_id == boarder_id)
        {
            bail!("That boarder is already on this bill.");
        }

        let allow_centavos = self.app_settings()?.allow_centavos;
        let mut boarders: Vec<SplitBoarder> = detail
            .shares
            .iter()
            .map(|share| SplitBoarder {
                boarder_id: share.share.boarder_id.clone(),
                name: share.display_name().to_string(),
                created_at: share.share.created_at,
            })
            .collect();
        boarders.push(SplitBoarder {
            boarder_id: boarder.id.clone(),
            name: boarder.name.clone(),
            created_at: boarder.created_at,
        });

        let split = match equal_split(detail.bill.total_minor, &boarders, allow_centavos) {
            SplitOutcome::Ok(success) => success.shares,
            SplitOutcome::Err(failure) => bail!("{}", failure.error),
        };

        let now = now_ms();
        let transaction = self.conn.unchecked_transaction()?;
        for share in &split {
            if share.boarder_id == boarder_id {
                transaction.execute(
                    "INSERT INTO bill_shares (id, bill_id, boarder_id, boarder_name_snapshot,
                                              amount_due_minor, created_at, additional_charge_minor)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                    params![
                        new_id(),
                        bill_id,
                        share.boarder_id,
                        boarder.name,
                        share.amount_minor,
                        now
                    ],
                )?;
            } else {
                transaction.execute(
                    "UPDATE bill_shares SET amount_due_minor = ?1
                     WHERE bill_id = ?2 AND boarder_id = ?3",
                    params![share.amount_minor, bill_id, share.boarder_id],
                )?;
            }
        }
        transaction.execute(
            "UPDATE bills SET updated_at = ?1 WHERE id = ?2",
            params![now, bill_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Permanently deletes a bill together with every share and payment
    /// recorded against it.
    pub fn delete_bill(&self, id: &str) -> Result<()> {
        let exists: i64 = self.conn.query_row(
            "SELECT count(*) FROM bills WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            bail!("Bill not found.");
        }

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM payments
             WHERE bill_share_id IN (SELECT id FROM bill_shares WHERE bill_id = ?1)",
            params![id],
        )?;
        transaction.execute("DELETE FROM bill_shares WHERE bill_id = ?1", params![id])?;
        transaction.execute("DELETE FROM bills WHERE id = ?1", params![id])?;
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
