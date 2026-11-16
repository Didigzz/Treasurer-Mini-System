
use anyhow::{bail, Result};
use rusqlite::params;

use crate::calculations::{
    compute_share_state, recompute_payment_changes,
};
use crate::models::*;
use crate::money;

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
            share.total_due_minor(),
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

        // A payment settles the share or overpays it. The minimum is always the
        // remaining balance, so a legacy partial share can only be completed.
        let mut payments = self.payments_for_share(&input.bill_share_id)?;
        let minimum_minor =
            compute_share_state(share.total_due_minor(), &payments).balance_minor;
        if input.amount_minor < minimum_minor {
            let currency = self
                .app_settings()
                .map(|settings| settings.currency_code)
                .unwrap_or_else(|_| "PHP".to_string());
            bail!(
                "Record the full remaining balance of {} or more. Partial payments aren't allowed.",
                money::format_money(minimum_minor, &currency)
            );
        }

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
            payment_type: input.payment_type,
            used_full_balance: input.used_full_balance,
        };

        payments.push(draft);
        let computed = recompute_payment_changes(share.total_due_minor(), &payments);
        let Some(new_payment) = computed.iter().find(|item| item.payment.id == payment_id) else {
            bail!("The payment could not be calculated.");
        };
        let change_minor = new_payment.computed_change_minor;

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO payments (id, bill_share_id, amount_minor, paid_at, note, change_minor,
                                   change_returned_at, change_return_note, voided_at, void_reason,
                                   created_at, payment_type, used_full_balance)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL, NULL, ?7, ?8, ?9)",
            params![
                payment_id,
                input.bill_share_id,
                input.amount_minor,
                input.paid_at,
                input.note,
                change_minor,
                now,
                input.payment_type.as_str(),
                input.used_full_balance as i64
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

    /// Reverses a payment confirmation. The row is kept so the amount, paid
    /// date, note, and any legacy reason stay readable; `voided_at` is the
    /// only reversal marker. Only the remaining active payments count toward
    /// the balance and change totals.
    pub fn undo_payment(&self, payment_id: &str) -> Result<()> {
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
            bail!("This payment was already undone.");
        }
        if payment.change_returned_at.is_some() {
            bail!("This payment can't be undone because its change was already returned.");
        }

        let Some(share) = self.share_by_id(&payment.bill_share_id)? else {
            bail!("Bill share not found.");
        };
        let Some(bill) = self
            .all_bills()?
            .into_iter()
            .find(|bill| bill.id == share.bill_id)
        else {
            bail!("Bill not found.");
        };
        if bill.archived_at.is_some() {
            bail!("Restore this bill before undoing a payment.");
        }

        let now = now_ms();
        let updated: Vec<PaymentRecord> = payments
            .get(&payment.bill_share_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                if item.id == payment_id {
                    PaymentRecord {
                        voided_at: Some(now),
                        change_minor: 0,
                        ..item
                    }
                } else {
                    item
                }
            })
            .collect();

        let computed = recompute_payment_changes(share.total_due_minor(), &updated);

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE payments SET voided_at = ?1, change_minor = 0 WHERE id = ?2",
            params![now, payment_id],
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
            bail!("Undone payments cannot return change.");
        }
        if payment.change_returned_at.is_some() {
            bail!("This change was already marked returned.");
        }

        let Some(share) = self.share_by_id(&payment.bill_share_id)? else {
            bail!("Bill share not found.");
        };
        let computed = recompute_payment_changes(
            share.total_due_minor(),
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

    /// Corrects the date a returned change was handed back. Only affects the
    /// date; the amount always stays the computed change.
    pub fn update_change_returned_at(&self, payment_id: &str, returned_at: i64) -> Result<()> {
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
            bail!("Undone payments cannot return change.");
        }
        if payment.change_returned_at.is_none() {
            bail!("This change has not been marked returned yet.");
        }
        if payment.change_minor <= 0 {
            bail!("There is no change to return for this payment.");
        }

        self.conn.execute(
            "UPDATE payments SET change_returned_at = ?1 WHERE id = ?2",
            params![returned_at, payment_id],
        )?;
        Ok(())
    }
}
