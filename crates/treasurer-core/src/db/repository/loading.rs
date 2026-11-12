use std::collections::HashMap;

use anyhow::Result;

use crate::calculations::{
    compute_bill_summary, compute_share_state,
};
use crate::models::*;

use super::{map_bill, map_payment, map_share};
use super::Treasurer;

impl Treasurer {
    pub(super) fn all_shares(&self) -> Result<Vec<BillShare>> {
        let mut statement = self.conn.prepare(
            "SELECT id, bill_id, boarder_id, boarder_name_snapshot, amount_due_minor, created_at
             FROM bill_shares",
        )?;
        let rows = statement
            .query_map([], map_share)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(super) fn payments_by_share(&self) -> Result<HashMap<String, Vec<PaymentRecord>>> {
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

    pub(super) fn all_bills(&self) -> Result<Vec<Bill>> {
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

    pub(super) fn shares_with_states(&self) -> Result<Vec<ShareWithState>> {
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

    pub(super) fn bill_with_details(&self, bill: &Bill, shares: &[ShareWithState]) -> BillWithDetails {
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
}
