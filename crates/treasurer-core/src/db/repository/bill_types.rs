use std::collections::HashMap;

use anyhow::{bail, Result};
use rusqlite::params;

use crate::models::*;

use super::{map_bill_type, new_id, now_ms};
use super::Treasurer;

impl Treasurer {
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

    pub(super) fn bill_type(&self, id: &str) -> Result<Option<BillType>> {
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
}
