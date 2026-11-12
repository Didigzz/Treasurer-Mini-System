use std::collections::HashMap;

use anyhow::{bail, Result};
use rusqlite::params;

use crate::calculations::compute_share_state;
use crate::models::*;

use super::{map_boarder, new_id, now_ms};
use super::Treasurer;

impl Treasurer {
    pub(super) fn all_boarders(&self) -> Result<Vec<Boarder>> {
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
}
