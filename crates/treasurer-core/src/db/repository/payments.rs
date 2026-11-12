
use anyhow::{bail, Result};
use rusqlite::params;

use crate::calculations::{
    compute_share_state, recompute_payment_changes,
};
use crate::models::*;

use super::{new_id, now_ms};
use super::{SharePaymentContext, Treasurer};

impl Treasurer {
    pub(super) fn share_by_id(&self, id: &str) -> Result<Option<BillShare>> {
        Ok(self
            .all_shares()?
            .into_iter()
            .find(|share| share.id == id))
    }

    pub(super) fn payments_for_share(&self, share_id: &str) -> Result<Vec<PaymentRecord>> {
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
}
