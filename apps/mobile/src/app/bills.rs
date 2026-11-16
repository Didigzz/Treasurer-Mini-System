use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use chrono::{Local, NaiveDate};
use slint::{ModelRc, SharedString, VecModel};

use treasurer_core::calculations::{
    collected_for_payment_type, latest_active_payment_type, matches_payment_type_filter,
    resplit_with_pinned, split_with_overrides, validate_additional_charge, SplitOutcome,
};
use treasurer_core::db::repository::now_ms;
use treasurer_core::models::*;
use treasurer_core::money;
use treasurer_core::selection::{selected_count, selection_state, toggle_select_all, SelectionState};

use super::calendar::{parse_hhmm, parse_iso_date, TARGET_CHANGE_RETURNED};
use super::{progress, App};
use crate::{AppWindow, ManageRow, PaymentRow, PickRow, ShareRow, ShareSplitPreviewRow, UndoPickRow};

pub(super) struct Pick {
    pub(super) boarder_id: String,
    pub(super) name: String,
    pub(super) selected: bool,
    pub(super) amount: String,
}

/// Raw values for one share of the open bill. Formatting happens when the
/// filtered rows are built, so the filter and the editor never reparse money.
pub(super) struct ShareEntry {
    pub(super) id: String,
    pub(super) boarder_id: String,
    pub(super) name: String,
    pub(super) base_due: i64,
    pub(super) charge: i64,
    pub(super) paid: i64,
    pub(super) balance: i64,
    pub(super) change: i64,
    pub(super) change_returned: i64,
    pub(super) status: ShareStatus,
    /// Type of the newest active payment, shared by the pill and the filter.
    pub(super) latest_payment_type: Option<PaymentType>,
}

impl App {
    pub(super) fn load_form_types(&mut self) {
        let types = match self.db.list_bill_types() {
            Ok(types) => types,
            Err(_) => return,
        };
        let active: Vec<_> = types
            .iter()
            .filter(|row| !row.bill_type.is_archived)
            .collect();
        self.form_type_ids = active.iter().map(|row| row.bill_type.id.clone()).collect();
        let names: Vec<SharedString> = active
            .iter()
            .map(|row| SharedString::from(row.bill_type.name.clone()))
            .collect();
        self.ui()
            .set_bill_type_names(ModelRc::new(VecModel::from(names)));
    }

    pub(super) fn open_new_bill_form(&mut self) {
        self.load_form_types();
        let boarders = match self.db.active_boarders() {
            Ok(boarders) => boarders,
            Err(error) => return self.message(format!("{error:#}")),
        };
        self.pick_rows = boarders
            .into_iter()
            .map(|boarder| Pick {
                boarder_id: boarder.id,
                name: boarder.name,
                selected: false,
                amount: String::new(),
            })
            .collect();

        let ui = self.ui();
        ui.set_bill_year(self.year.to_string().into());
        ui.set_bill_month(self.month.to_string().into());
        ui.set_bill_period_label(money::format_period(self.year, self.month).into());
        ui.set_bill_total("".into());
        ui.set_bill_due("".into());
        ui.set_bill_due_label("".into());
        ui.set_bill_note("".into());
        ui.set_bill_type_index(0);
        self.render_picks();
    }

    pub(super) fn render_picks(&self) {
        let rows: Vec<PickRow> = self
            .pick_rows
            .iter()
            .map(|pick| PickRow {
                name: pick.name.clone().into(),
                selected: pick.selected,
                amount: pick.amount.clone().into(),
            })
            .collect();
        let flags: Vec<bool> = self.pick_rows.iter().map(|pick| pick.selected).collect();
        let state = selection_state(&flags);
        let ui = self.ui();
        ui.set_picks(ModelRc::new(VecModel::from(rows)));
        ui.set_picks_selected_count(selected_count(&flags) as i32);
        ui.set_picks_total_count(flags.len() as i32);
        ui.set_picks_all_selected(state == SelectionState::All);
        ui.set_picks_some_selected(state == SelectionState::Some);
    }

    pub(super) fn refresh_bill_detail(&mut self) {
        let Some(id) = self.detail_bill.clone() else {
            return;
        };
        let detail = match self.db.bill_detail(&id) {
            Ok(Some(detail)) => detail,
            Ok(None) => {
                self.clear_detail_lists();
                return self.message("Bill not found.");
            }
            Err(error) => {
                self.clear_detail_lists();
                return self.message(format!("{error:#}"));
            }
        };

        let ui = self.ui();
        ui.set_detail_title(
            format!(
                "{} · {}",
                detail.current_type_name,
                money::format_period(detail.bill.period_year, detail.bill.period_month)
            )
            .into(),
        );
        let mut subtitle = format!(
            "Total {}",
            money::format_money(detail.bill.total_minor, &self.currency)
        );
        if let Some(due) = &detail.bill.due_date {
            subtitle.push_str(&format!(" · due {}", money::format_date_only(due)));
        }
        if let Some(note) = &detail.bill.note {
            subtitle.push_str(&format!(" · {note}"));
        }
        ui.set_detail_subtitle(subtitle.into());
        let summary = &detail.summary;
        ui.set_detail_status(summary.status.label().into());
        ui.set_detail_archived(detail.bill.archived_at.is_some());
        ui.set_detail_expected(money::format_money(summary.expected_minor, &self.currency).into());
        ui.set_detail_collected(money::format_money(summary.applied_minor, &self.currency).into());
        ui.set_detail_outstanding(
            money::format_money(summary.outstanding_minor, &self.currency).into(),
        );
        ui.set_detail_change(
            money::format_money(summary.change_outstanding_minor, &self.currency).into(),
        );
        ui.set_detail_progress(progress(summary.applied_minor, summary.expected_minor));

        self.detail_total_minor = detail.bill.total_minor;
        self.detail_locked = detail.is_locked;
        self.detail_archived = detail.bill.archived_at.is_some();
        ui.set_detail_total(
            money::format_money(detail.bill.total_minor, &self.currency).into(),
        );

        self.detail_shares = detail
            .shares
            .iter()
            .map(|share| ShareEntry {
                id: share.share.id.clone(),
                boarder_id: share.share.boarder_id.clone(),
                name: share.display_name().to_string(),
                base_due: share.share.amount_due_minor,
                charge: share.share.additional_charge_minor,
                paid: share.state.paid_minor,
                balance: share.state.balance_minor,
                change: share.state.change_outstanding_minor,
                change_returned: share.state.change_returned_minor,
                status: share.state.status,
                latest_payment_type: latest_active_payment_type(&share.state.payments),
            })
            .collect();
        self.cash_collected_minor = detail
            .shares
            .iter()
            .map(|share| collected_for_payment_type(&share.state.payments, PaymentType::Cash))
            .sum();
        self.online_collected_minor = detail
            .shares
            .iter()
            .map(|share| collected_for_payment_type(&share.state.payments, PaymentType::Online))
            .sum();
        self.render_shares();

        let mut payment_rows: Vec<(i64, PaymentRow, String, String)> = Vec::new();
        for share in &detail.shares {
            for payment in &share.state.payments {
                let record = &payment.payment;
                let computed = payment.computed_change_minor;
                let mut subtitle = money::format_datetime(record.paid_at);
                if let Some(note) = &record.note {
                    subtitle.push_str(&format!(" · {note}"));
                }
                if !record.is_active() {
                    match &record.void_reason {
                        Some(reason) => subtitle.push_str(&format!(" · undone: {reason}")),
                        None => subtitle.push_str(" · undone"),
                    }
                }
                let change_status = if !record.is_active() || computed <= 0 {
                    String::new()
                } else if record.change_returned_at.is_some() {
                    format!(
                        "Change Returned {}",
                        money::format_money(computed, &self.currency)
                    )
                } else {
                    format!(
                        "Change to Return {}",
                        money::format_money(computed, &self.currency)
                    )
                };
                let can_return = record.is_active()
                    && record.change_returned_at.is_none()
                    && computed > 0;
                payment_rows.push((
                    record.paid_at,
                    PaymentRow {
                        title: format!(
                            "{} · {}",
                            share.display_name(),
                            money::format_date(record.paid_at)
                        )
                        .into(),
                        subtitle: subtitle.into(),
                        amount: money::format_money(record.amount_minor, &self.currency).into(),
                        payment_type: record.payment_type.label().into(),
                        status: record.status_label().into(),
                        change: money::format_money(computed, &self.currency).into(),
                        change_status: change_status.into(),
                        can_return,
                        has_change: computed > 0,
                        voided: !record.is_active(),
                    },
                    record.id.clone(),
                    record.bill_share_id.clone(),
                ));
            }
        }
        payment_rows.sort_by_key(|b| std::cmp::Reverse(b.0));
        self.payment_ids = payment_rows.iter().map(|(_, _, id, _)| id.clone()).collect();
        self.payment_share_ids = payment_rows
            .iter()
            .map(|(_, _, _, share_id)| share_id.clone())
            .collect();
        ui.set_payments(ModelRc::new(VecModel::from(
            payment_rows
                .into_iter()
                .map(|(_, row, _, _)| row)
                .collect::<Vec<_>>(),
        )));
    }

    /// Opens a bill from the dashboard list, or from the boarder ledger when
    /// `from_ledger` is set (Back then returns to that ledger).
    pub(super) fn open_bill(&mut self, index: usize, from_ledger: bool) {
        let ids = if from_ledger {
            &self.ledger_bill_ids
        } else {
            &self.bill_ids
        };
        let Some(id) = ids.get(index).cloned() else {
            return;
        };
        self.clear_detail_lists();
        self.detail_bill = Some(id);
        self.bill_from_ledger = from_ledger;
        let ui = self.ui();
        ui.set_payment_amount("".into());
        ui.set_payment_date("".into());
        ui.set_payment_time("".into());
        ui.set_payment_date_label("".into());
        self.set_screen(10);
        self.refresh_bill_detail();
    }

    pub(super) fn pick_toggle(&mut self, index: usize) {
        if let Some(pick) = self.pick_rows.get_mut(index) {
            pick.selected = !pick.selected;
        }
        self.render_picks();
    }

    pub(super) fn pick_amount(&mut self, index: usize, amount: &str) {
        if let Some(pick) = self.pick_rows.get_mut(index) {
            pick.amount = amount.to_string();
        }
    }

    /// Tri-state select all: everything selected clears the selection;
    /// none or some selects every active boarder. Typed amounts are kept.
    pub(super) fn select_all_picks(&mut self) {
        let flags: Vec<bool> = self.pick_rows.iter().map(|pick| pick.selected).collect();
        let next = toggle_select_all(&flags);
        for (pick, selected) in self.pick_rows.iter_mut().zip(next) {
            pick.selected = selected;
        }
        self.render_picks();
    }

    pub(super) fn create_bill(&mut self) {
        let ui = self.ui();
        if self.form_type_ids.is_empty() {
            return self.message("Add a bill type in Settings first.");
        }
        let type_index = ui.get_bill_type_index().max(0) as usize;
        let Some(bill_type_id) = self.form_type_ids.get(type_index).cloned() else {
            return self.message("Choose a bill type.");
        };
        let year: i32 = ui.get_bill_year().trim().parse().unwrap_or(0);
        let month: i32 = ui.get_bill_month().trim().parse().unwrap_or(0);
        if !(2000..=2100).contains(&year) || !(1..=12).contains(&month) {
            return self.message("Enter a valid year and month.");
        }

        let Some(total_minor) =
            money::parse_amount_input(&ui.get_bill_total(), self.allow_centavos)
        else {
            return self.message("Enter a valid bill total.");
        };

        let selected: Vec<&Pick> = self.pick_rows.iter().filter(|pick| pick.selected).collect();
        if selected.is_empty() {
            return self.message("Select at least one boarder.");
        }

        // A typed amount is pinned; every blank amount shares the remainder.
        let mut overrides: HashMap<String, i64> = HashMap::new();
        for pick in &selected {
            if pick.amount.trim().is_empty() {
                continue;
            }
            let Some(amount) = money::parse_money_to_minor(&pick.amount) else {
                return self.message("Enter valid custom amounts.");
            };
            overrides.insert(pick.boarder_id.clone(), amount);
        }

        let boarders: Vec<SplitBoarder> = selected
            .iter()
            .map(|pick| SplitBoarder {
                boarder_id: pick.boarder_id.clone(),
                name: pick.name.clone(),
                created_at: 0,
            })
            .collect();
        let shares = match split_with_overrides(
            total_minor,
            &boarders,
            &overrides,
            self.allow_centavos,
        ) {
            SplitOutcome::Ok(success) => success.shares,
            SplitOutcome::Err(failure) => return self.message(failure.error),
        };

        let due_date = {
            let value = ui.get_bill_due().trim().to_string();
            if value.is_empty() {
                None
            } else if NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_ok() {
                Some(value)
            } else {
                return self.message("Due date must look like 2026-11-30.");
            }
        };
        let note = {
            let value = ui.get_bill_note().trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        };

        let input = CreateBillInput {
            bill_type_id,
            period_year: year,
            period_month: month,
            total_minor,
            due_date,
            note,
            shares,
        };

        match self.db.create_bill(&input) {
            Ok(_) => {
                self.year = year;
                self.month = month;
                self.detail_bill = None;
                self.set_screen(0);
                self.refresh_history();
                // After set_screen, which clears the banner.
                self.notify("Bill created.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    /// Filters `detail_shares` by name, status, and payment type, then rebuilds
    /// the visible row model and the parallel ID list used by Pay and Edit.
    pub(super) fn render_shares(&mut self) {
        let currency = self.currency.clone();
        let needle = self.share_filter.trim().to_lowercase();
        let mode = self.share_filter_mode;
        let type_mode = self.share_type_filter;

        let visible: Vec<&ShareEntry> = self
            .detail_shares
            .iter()
            .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            .filter(|entry| match mode {
                1 => entry.balance > 0,
                2 => entry.balance == 0,
                _ => true,
            })
            .filter(|entry| matches_payment_type_filter(entry.latest_payment_type, type_mode))
            .collect();

        let rows: Vec<ShareRow> = visible
            .iter()
            .map(|entry| {
                let change_status = if entry.change > 0 {
                    if entry.change_returned > 0 {
                        format!(
                            "Change Returned {} · {} to return",
                            money::format_money(entry.change_returned, &currency),
                            money::format_money(entry.change, &currency)
                        )
                    } else {
                        format!(
                            "Change to Return {}",
                            money::format_money(entry.change, &currency)
                        )
                    }
                } else if entry.change_returned > 0 {
                    format!(
                        "Change Returned {}",
                        money::format_money(entry.change_returned, &currency)
                    )
                } else {
                    String::new()
                };
                ShareRow {
                    name: entry.name.clone().into(),
                    due: money::format_money(entry.base_due, &currency).into(),
                    paid: money::format_money(entry.paid, &currency).into(),
                    balance: money::format_money(entry.balance, &currency).into(),
                    balance_input: money::minor_to_input_value(entry.balance.max(0)).into(),
                    change: money::format_money(entry.change, &currency).into(),
                    change_status: change_status.clone().into(),
                    charge: money::format_money(entry.charge, &currency).into(),
                    status: match (entry.status, entry.latest_payment_type) {
                        (ShareStatus::Paid, Some(payment_type)) => payment_type.status_label(),
                        _ => entry.status.label(),
                    }
                    .into(),
                    settled: entry.balance == 0,
                    has_balance: entry.balance > 0,
                    has_change: entry.change > 0,
                    has_change_status: !change_status.is_empty(),
                    has_charge: entry.charge > 0,
                }
            })
            .collect();

        self.display_share_ids = visible.iter().map(|entry| entry.id.clone()).collect();
        debug_assert_eq!(self.display_share_ids.len(), rows.len());

        let ui = self.ui();
        // The method totals cover the whole bill, so they never depend on the
        // visible rows; only the type selection decides which one shows.
        let (caption, collected) = match type_mode {
            1 => (
                "Cash collected",
                money::format_money(self.cash_collected_minor, &currency),
            ),
            2 => (
                "Online collected",
                money::format_money(self.online_collected_minor, &currency),
            ),
            _ => ("", String::new()),
        };

        ui.set_shares(ModelRc::new(VecModel::from(rows)));
        ui.set_share_total_count(self.detail_shares.len() as i32);
        ui.set_share_visible_count(self.display_share_ids.len() as i32);
        ui.set_share_type_collected_caption(caption.into());
        ui.set_share_type_collected(collected.into());
    }

    pub(super) fn set_share_filter(&mut self, text: &str) {
        self.share_filter = text.to_string();
        self.ui().set_share_filter(self.share_filter.clone().into());
        self.render_shares();
    }

    pub(super) fn set_share_filter_mode(&mut self, mode: i32) {
        self.share_filter_mode = mode.clamp(0, 2);
        self.ui()
            .set_share_filter_mode(self.share_filter_mode);
        self.render_shares();
    }

    pub(super) fn set_share_type_filter(&mut self, mode: i32) {
        self.share_type_filter = mode.clamp(0, 2);
        self.ui().set_share_type_filter(self.share_type_filter);
        self.render_shares();
    }

    /// The boarder behind the share captured when the editor opened. Using the
    /// share ID (not the visible row index) keeps filtering harmless.
    fn edited_boarder_id(&self) -> Option<String> {
        let share_id = self.edit_share_id.as_deref()?;
        self.detail_shares
            .iter()
            .find(|entry| entry.id == share_id)
            .map(|entry| entry.boarder_id.clone())
    }

    fn current_split(&self) -> Vec<SplitResult> {
        self.detail_shares
            .iter()
            .map(|entry| SplitResult {
                boarder_id: entry.boarder_id.clone(),
                amount_minor: entry.base_due,
            })
            .collect()
    }

    fn split_boarders(&self) -> Vec<SplitBoarder> {
        self.detail_shares
            .iter()
            .map(|entry| SplitBoarder {
                boarder_id: entry.boarder_id.clone(),
                name: entry.name.clone(),
                created_at: 0,
            })
            .collect()
    }

    fn parse_share_edit(&self, amount_text: &str, charge_text: &str) -> Result<(i64, i64), String> {
        if amount_text.trim().is_empty() {
            return Err("Enter a share amount.".to_string());
        }
        let amount = money::parse_money_to_minor(amount_text)
            .ok_or_else(|| "Enter a valid share amount.".to_string())?;
        let charge = if charge_text.trim().is_empty() {
            0
        } else {
            money::parse_money_to_minor(charge_text)
                .ok_or_else(|| "Enter a valid additional charge.".to_string())?
        };
        if let Some(message) = validate_additional_charge(charge, self.allow_centavos) {
            return Err(message);
        }
        Ok((amount, charge))
    }

    /// The split Save will persist for an edit of `boarder_id` at
    /// `amount_minor`. Locked bills keep their stored split, and so does an
    /// edit that leaves the amount alone: adding or clearing a charge must
    /// never move another boarder's share.
    fn resolved_split(
        &self,
        boarder_id: &str,
        amount_minor: i64,
    ) -> Result<Vec<SplitResult>, String> {
        let amount_unchanged = self
            .detail_shares
            .iter()
            .find(|entry| entry.boarder_id == boarder_id)
            .map(|entry| entry.base_due == amount_minor)
            .unwrap_or(false);
        if self.detail_locked || amount_unchanged {
            return Ok(self.current_split());
        }

        match resplit_with_pinned(
            self.detail_total_minor,
            &self.split_boarders(),
            boarder_id,
            amount_minor,
            self.allow_centavos,
        ) {
            SplitOutcome::Ok(success) => Ok(success.shares),
            SplitOutcome::Err(failure) => Err(failure.error),
        }
    }

    pub(super) fn open_edit_share(&mut self, index: usize) {
        let Some(share_id) = self.display_share_ids.get(index).cloned() else {
            return;
        };
        let Some((name, base_due, charge)) = self
            .detail_shares
            .iter()
            .find(|entry| entry.id == share_id)
            .map(|entry| (entry.name.clone(), entry.base_due, entry.charge))
        else {
            return;
        };

        self.edit_share_id = Some(share_id);

        let amount = money::minor_to_input_value(base_due);
        let charge_text = if charge > 0 {
            money::minor_to_input_value(charge)
        } else {
            String::new()
        };

        let ui = self.ui();
        ui.set_share_edit_target(index as i32);
        ui.set_share_edit_name(name.into());
        ui.set_detail_total(
            money::format_money(self.detail_total_minor, &self.currency).into(),
        );
        ui.set_share_edit_amount(amount.clone().into());
        ui.set_share_edit_charge(charge_text.clone().into());
        ui.set_share_edit_locked(self.detail_locked);
        ui.set_share_edit_archived(self.detail_archived);
        self.preview_share_split(&amount, &charge_text);
    }

    pub(super) fn preview_share_split(&mut self, amount_text: &str, charge_text: &str) {
        let Some(boarder_id) = self.edited_boarder_id() else {
            return;
        };

        let (amount_minor, charge_minor) = match self.parse_share_edit(amount_text, charge_text) {
            Ok(values) => values,
            Err(message) => {
                let ui = self.ui();
                ui.set_share_edit_error(message.into());
                ui.set_share_edit_can_save(false);
                return;
            }
        };

        let split = match self.resolved_split(&boarder_id, amount_minor) {
            Ok(split) => split,
            Err(message) => {
                let ui = self.ui();
                ui.set_share_edit_error(message.into());
                ui.set_share_edit_can_save(false);
                return;
            }
        };

        let mut rows = Vec::with_capacity(self.detail_shares.len());
        let mut shares_total = 0i64;
        let mut charges_total = 0i64;
        for entry in &self.detail_shares {
            let base = split
                .iter()
                .find(|result| result.boarder_id == entry.boarder_id)
                .map(|result| result.amount_minor)
                .unwrap_or(entry.base_due);
            let charge = if entry.boarder_id == boarder_id {
                charge_minor
            } else {
                entry.charge
            };
            shares_total += base;
            charges_total += charge;
            rows.push(ShareSplitPreviewRow {
                name: entry.name.clone().into(),
                amount: money::format_money(base + charge, &self.currency).into(),
                pinned: entry.boarder_id == boarder_id,
            });
        }

        let ui = self.ui();
        ui.set_share_preview(ModelRc::new(VecModel::from(rows)));
        ui.set_share_edit_footer(
            format!(
                "Shares {} · Charges {} · Expected {}",
                money::format_money(shares_total, &self.currency),
                money::format_money(charges_total, &self.currency),
                money::format_money(shares_total + charges_total, &self.currency)
            )
            .into(),
        );
        ui.set_share_edit_error("".into());
        ui.set_share_edit_can_save(true);
    }

    pub(super) fn save_share(&mut self) {
        let Some(bill_id) = self.detail_bill.clone() else {
            return;
        };
        let Some(boarder_id) = self.edited_boarder_id() else {
            return;
        };

        let ui = self.ui();
        let amount_text = ui.get_share_edit_amount().to_string();
        let charge_text = ui.get_share_edit_charge().to_string();

        let (amount_minor, charge_minor) = match self.parse_share_edit(&amount_text, &charge_text) {
            Ok(values) => values,
            Err(message) => {
                ui.set_share_edit_error(message.into());
                ui.set_share_edit_can_save(false);
                return;
            }
        };

        let split = match self.resolved_split(&boarder_id, amount_minor) {
            Ok(split) => split,
            Err(message) => {
                ui.set_share_edit_error(message.into());
                ui.set_share_edit_can_save(false);
                return;
            }
        };

        let changed_amounts = split.iter().any(|result| {
            self.detail_shares
                .iter()
                .find(|entry| entry.boarder_id == result.boarder_id)
                .map(|entry| entry.base_due != result.amount_minor)
                .unwrap_or(false)
        });

        match self
            .db
            .save_share_edit(&bill_id, &split, &boarder_id, charge_minor)
        {
            Ok(()) => {
                self.edit_share_id = None;
                self.refresh_bill_detail();
                self.refresh_dashboard();
                self.refresh_history();
                self.refresh_boarders();
                self.notify(if changed_amounts {
                    "Share updated."
                } else {
                    "Additional charge updated."
                });
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn pay_share(&mut self, index: usize) {
        let Some(share_id) = self.display_share_ids.get(index).cloned() else {
            return self.message("That share is no longer available. Go back and reopen the bill.");
        };
        let ui = self.ui();
        let amount_text = ui.get_payment_amount();
        let Some(amount_minor) = money::parse_amount_input(&amount_text, self.allow_centavos)
        else {
            return self.message("Enter a valid payment amount.");
        };
        if amount_minor <= 0 {
            return self.message("Enter a payment amount greater than zero.");
        }

        // Same rule as the domain, applied here so the Pay sheet reports it
        // inline instead of a banner round trip.
        if let Some(entry) = self.detail_shares.iter().find(|entry| entry.id == share_id) {
            if amount_minor < entry.balance {
                return self.message(format!(
                    "Record the full remaining balance of {} or more. Partial payments aren't allowed.",
                    money::format_money(entry.balance, &self.currency)
                ));
            }
        }

        // The picker always supplies a date; an empty value would only mean
        // "now" for safety (e.g. after a restore that lost the field).
        let paid_at = match parse_iso_date(&ui.get_payment_date()) {
            Some(date) => {
                let (hour, minute) = parse_hhmm(&ui.get_payment_time()).unwrap_or((12, 0));
                money::local_datetime_ms(date, hour as u32, minute as u32)
            }
            None => now_ms(),
        };

        let payment_type = if ui.get_payment_type_index() == 1 {
            PaymentType::Online
        } else {
            PaymentType::Cash
        };
        // The "Use full balance" button fills the amount with the outstanding
        // balance. Only treat the flag as used when that amount is still intact.
        let used_full_balance = ui.get_payment_used_full_balance()
            && self
                .detail_shares
                .iter()
                .find(|entry| entry.id == share_id)
                .map(|entry| entry.balance > 0 && entry.balance == amount_minor)
                .unwrap_or(false);

        match self.db.record_payment(&CreatePaymentInput {
            bill_share_id: share_id.clone(),
            amount_minor,
            paid_at,
            note: None,
            payment_type,
            used_full_balance,
        }) {
            // The payment is committed before any dialog appears; a refresh
            // failure can never make a saved payment look failed.
            Ok(payment_id) => {
                ui.set_payment_amount("".into());
                self.refresh_after_change();

                let context = self.db.share_payment_context(&share_id).ok().flatten();
                let change = context
                    .as_ref()
                    .and_then(|context| {
                        context
                            .share
                            .state
                            .payments
                            .iter()
                            .find(|item| item.payment.id == payment_id)
                            .map(|item| item.computed_change_minor)
                    })
                    .unwrap_or(0);

                if change > 0 {
                    if let Some(context) = context {
                        self.overpay_payment_id = Some(payment_id);
                        self.overpay_share_id = Some(share_id);
                        let ui = self.ui();
                        ui.set_overpay_subtitle(
                            format!(
                                "{} · {} {}",
                                context.share.display_name(),
                                context.bill.bill_type_name_snapshot,
                                money::format_period(
                                    context.bill.period_year,
                                    context.bill.period_month
                                )
                            )
                            .into(),
                        );
                        ui.set_overpay_amount(
                            money::format_money(amount_minor, &self.currency).into(),
                        );
                        ui.set_overpay_change(
                            money::format_money(change, &self.currency).into(),
                        );
                        ui.set_overpay_due(
                            money::format_money(
                                context.share.share.total_due_minor(),
                                &self.currency,
                            )
                            .into(),
                        );
                        ui.set_overpay_shown(true);
                    }
                } else {
                    self.notify(format!(
                        "Payment of {} recorded.",
                        money::format_money(amount_minor, &self.currency)
                    ));
                }
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    /// Gates the Pay sheet's Record button and explains the minimum. Slint
    /// cannot compare formatted money strings, so the check lives here.
    pub(super) fn payment_amount_edited(&mut self, index: i32, value: &str) {
        let balance = self
            .display_share_ids
            .get(index.max(0) as usize)
            .and_then(|id| self.detail_shares.iter().find(|entry| &entry.id == id))
            .map(|entry| entry.balance);

        let (can_record, hint) = match (balance, money::parse_amount_input(value, self.allow_centavos))
        {
            (Some(balance), Some(amount)) if amount > 0 => {
                if amount >= balance {
                    (true, String::new())
                } else {
                    (
                        false,
                        format!(
                            "Record at least {} — partial payments aren't allowed.",
                            money::format_money(balance, &self.currency)
                        ),
                    )
                }
            }
            _ => (false, String::new()),
        };

        let ui = self.ui();
        ui.set_payment_can_record(can_record);
        ui.set_payment_min_hint(hint.into());
    }

    /// Refreshes every list a payment or share change can affect.
    fn refresh_after_change(&mut self) {
        self.refresh_bill_detail();
        self.refresh_dashboard();
        self.refresh_history();
        self.refresh_boarders();
        if self.ui().get_screen() == 11 {
            self.refresh_ledger();
        }
        self.refresh_open_payment_detail();
    }

    fn set_undo_error(&self, text: impl Into<SharedString>) {
        self.ui().set_undo_error(text.into());
    }

    /// Share-row Undo, offered when the share is Paid. One active payment opens
    /// the confirmation dialog; several open the payment picker.
    pub(super) fn open_undo_share(&mut self, index: usize) {
        let Some(share_id) = self.display_share_ids.get(index).cloned() else {
            return self.message("That share is no longer available. Go back and reopen the bill.");
        };
        let context = match self.db.share_payment_context(&share_id) {
            Ok(Some(context)) => context,
            Ok(None) => {
                return self.message("That share is no longer available. Go back and reopen the bill.")
            }
            Err(error) => return self.message(format!("{error:#}")),
        };

        let mut active: Vec<&PaymentWithComputedChange> = context
            .share
            .state
            .payments
            .iter()
            .filter(|item| item.payment.is_active())
            .collect();
        if active.is_empty() {
            return self.message("There is no payment to undo for this share.");
        }
        active.sort_by(|a, b| {
            b.payment
                .paid_at
                .cmp(&a.payment.paid_at)
                .then(b.payment.created_at.cmp(&a.payment.created_at))
                .then(b.payment.id.cmp(&a.payment.id))
        });

        if active.len() == 1 {
            let payment_id = active[0].payment.id.clone();
            return self.prepare_undo(&payment_id, &share_id);
        }

        self.undo_picker_ids = active
            .iter()
            .map(|item| item.payment.id.clone())
            .collect();
        self.undo_share_id = Some(share_id);
        let rows: Vec<UndoPickRow> = active
            .iter()
            .enumerate()
            .map(|(position, item)| UndoPickRow {
                title: money::format_datetime(item.payment.paid_at).into(),
                subtitle: item.payment.payment_type.label().into(),
                amount: money::format_money(item.payment.amount_minor, &self.currency).into(),
                newest: position == 0,
            })
            .collect();

        let ui = self.ui();
        ui.set_undo_payments(ModelRc::new(VecModel::from(rows)));
        ui.set_undo_picker_subtitle(
            format!(
                "Choose the payment to reverse for {}.",
                context.share.display_name()
            )
            .into(),
        );
    }

    pub(super) fn undo_pick(&mut self, index: usize) {
        let Some(payment_id) = self.undo_picker_ids.get(index).cloned() else {
            return self.message("That payment is no longer available.");
        };
        let Some(share_id) = self.undo_share_id.clone() else {
            return self.message("That payment is no longer available.");
        };
        self.prepare_undo(&payment_id, &share_id);
    }

    /// Payment-row Undo from the bill or the boarder ledger.
    pub(super) fn open_undo_payment(&mut self, index: usize) {
        let Some(payment_id) = self.payment_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        let Some(share_id) = self.payment_share_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        self.prepare_undo(&payment_id, &share_id);
    }

    pub(super) fn open_undo_detail(&mut self) {
        let (Some(payment_id), Some(share_id)) = (
            self.detail_payment_id.clone(),
            self.detail_payment_share_id.clone(),
        ) else {
            return self.message("That payment is no longer available.");
        };
        self.prepare_undo(&payment_id, &share_id);
    }

    /// Validates the undo guards and fills the centered confirmation dialog.
    /// Nothing is written until `undo_confirm`.
    fn prepare_undo(&mut self, payment_id: &str, share_id: &str) {
        let context = match self.db.share_payment_context(share_id) {
            Ok(Some(context)) => context,
            Ok(None) => return self.message("That payment is no longer available."),
            Err(error) => return self.message(format!("{error:#}")),
        };
        let Some(item) = context
            .share
            .state
            .payments
            .iter()
            .find(|item| item.payment.id == payment_id)
        else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        let record = &item.payment;
        if !record.is_active() {
            return self.message("This payment was already undone.");
        }
        if record.change_returned_at.is_some() {
            return self
                .message("This payment can't be undone because its change was already returned.");
        }
        if context.bill.archived_at.is_some() {
            return self.message("Restore this bill before undoing a payment.");
        }

        let other_active = context
            .share
            .state
            .payments
            .iter()
            .filter(|item| item.payment.is_active() && item.payment.id != payment_id)
            .count();
        let computed_change = item.computed_change_minor;

        self.undo_payment_id = Some(payment_id.to_string());
        self.undo_share_id = Some(share_id.to_string());

        let ui = self.ui();
        ui.set_undo_context(
            format!(
                "{} · {} {}",
                context.share.display_name(),
                context.bill.bill_type_name_snapshot,
                money::format_period(context.bill.period_year, context.bill.period_month)
            )
            .into(),
        );
        ui.set_undo_payment_label(
            format!(
                "{} · {}",
                money::format_money(record.amount_minor, &self.currency),
                money::format_datetime(record.paid_at)
            )
            .into(),
        );
        ui.set_undo_remaining(
            if other_active > 0 {
                "Other payments remain on this share, so the status will be recalculated."
            } else {
                ""
            }
            .into(),
        );
        ui.set_undo_warning(
            if computed_change > 0 {
                format!(
                    "This payment has {} of change to return. Undoing it clears that change obligation.",
                    money::format_money(computed_change, &self.currency)
                )
            } else {
                String::new()
            }
            .into(),
        );
        ui.set_undo_error("".into());
        ui.set_undo_shown(true);
    }

    pub(super) fn undo_confirm(&mut self) {
        let Some(payment_id) = self.undo_payment_id.clone() else {
            return self.set_undo_error("That payment is no longer available.");
        };
        self.ui().set_undo_error("".into());

        match self.db.undo_payment(&payment_id) {
            Ok(()) => {
                self.undo_payment_id = None;
                self.undo_share_id = None;
                self.undo_picker_ids.clear();
                let ui = self.ui();
                ui.set_undo_shown(false);
                ui.set_undo_payments(slint::ModelRc::default());
                self.refresh_after_change();
                self.notify("Payment undone.");
            }
            Err(error) => self.set_undo_error(format!("{error:#}")),
        }
    }

    pub(super) fn undo_cancel(&mut self) {
        self.undo_payment_id = None;
        self.undo_share_id = None;
        self.undo_picker_ids.clear();
        self.ui().set_undo_error("".into());
    }

    /// Opens the return-change confirmation for one payment row.
    pub(super) fn return_change(&mut self, index: usize) {
        let Some(payment_id) = self.payment_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        let Some(share_id) = self.payment_share_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        self.prepare_return_change(&payment_id, &share_id);
    }

    /// Opens the return-change confirmation from a share row, using that
    /// share's newest payment that still has change outstanding.
    pub(super) fn return_share_change(&mut self, index: usize) {
        let Some(share_id) = self.display_share_ids.get(index).cloned() else {
            return self.message("That share is no longer available. Go back and reopen the bill.");
        };
        let context = match self.db.share_payment_context(&share_id) {
            Ok(Some(context)) => context,
            Ok(None) => return self.message("That share is no longer available."),
            Err(error) => return self.message(format!("{error:#}")),
        };
        let newest = context
            .share
            .state
            .payments
            .iter()
            .filter(|item| {
                item.payment.is_active()
                    && item.payment.change_returned_at.is_none()
                    && item.computed_change_minor > 0
            })
            .max_by(|a, b| {
                a.payment
                    .paid_at
                    .cmp(&b.payment.paid_at)
                    .then(a.payment.created_at.cmp(&b.payment.created_at))
                    .then(a.payment.id.cmp(&b.payment.id))
            })
            .map(|item| item.payment.id.clone());
        let Some(payment_id) = newest else {
            return self.message("There is no change to return for this share.");
        };
        self.prepare_return_change(&payment_id, &share_id);
    }

    fn prepare_return_change(&mut self, payment_id: &str, share_id: &str) {
        let context = match self.db.share_payment_context(share_id) {
            Ok(Some(context)) => context,
            Ok(None) => return self.message("That payment is no longer available."),
            Err(error) => return self.message(format!("{error:#}")),
        };
        let Some(payment) = context
            .share
            .state
            .payments
            .iter()
            .find(|item| item.payment.id == payment_id)
        else {
            return self.message("That payment is no longer available.");
        };
        if !payment.payment.is_active() {
            return self.message("Undone payments cannot return change.");
        }
        if payment.payment.change_returned_at.is_some() {
            return self.message("This change was already marked returned.");
        }
        if payment.computed_change_minor <= 0 {
            return self.message("There is no change to return for this payment.");
        }

        let change = payment.computed_change_minor;
        self.return_change_payment_id = Some(payment_id.to_string());
        self.return_change_share_id = Some(share_id.to_string());
        self.return_change_date = Local::now().date_naive();
        self.change_correction_payment_id = None;

        let ui = self.ui();
        ui.set_return_change_subtitle(
            format!("{} · {} to return", context.share.display_name(), money::format_money(change, &self.currency)).into(),
        );
        ui.set_return_change_amount(money::format_money(change, &self.currency).into());
        ui.set_return_change_date_label(
            money::format_date_only(&money::to_iso_date(self.return_change_date)).into(),
        );
        ui.set_return_change_note("".into());
    }

    pub(super) fn return_change_confirm(&mut self) {
        let Some(payment_id) = self.return_change_payment_id.clone() else {
            return self.message("That payment is no longer available.");
        };
        let amount = self.ui().get_return_change_amount().to_string();
        let note = self.ui().get_return_change_note().to_string();
        let note = note.trim();
        let note = if note.is_empty() { None } else { Some(note) };
        let at = money::local_datetime_ms(self.return_change_date, 12, 0);

        match self.db.mark_change_returned(&payment_id, at, note) {
            Ok(()) => {
                self.return_change_payment_id = None;
                self.return_change_share_id = None;
                self.refresh_after_change();
                self.notify(format!("Change of {amount} marked as returned."));
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn return_change_pick_date(&mut self) {
        if self.return_change_payment_id.is_none() {
            return self.message("That payment is no longer available.");
        }
        self.change_correction_payment_id = None;
        self.open_calendar(TARGET_CHANGE_RETURNED);
    }

    pub(super) fn overpay_dismiss(&mut self) {
        self.overpay_payment_id = None;
        self.overpay_share_id = None;
    }

    pub(super) fn overpay_return_change(&mut self) {
        let (Some(payment_id), Some(share_id)) = (
            self.overpay_payment_id.clone(),
            self.overpay_share_id.clone(),
        ) else {
            return self.message("That payment is no longer available.");
        };
        self.prepare_return_change(&payment_id, &share_id);
    }

    /// Opens the read-only payment detail for a bill or ledger payment row.
    pub(super) fn open_payment(&mut self, index: usize) {
        let Some(payment_id) = self.payment_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        let Some(share_id) = self.payment_share_ids.get(index).cloned() else {
            return self.message("That payment is no longer available. Go back and reopen the bill.");
        };
        self.show_payment_detail(&payment_id, &share_id);
    }

    pub(super) fn show_payment_detail(&mut self, payment_id: &str, share_id: &str) {
        let context = match self.db.share_payment_context(share_id) {
            Ok(Some(context)) => context,
            Ok(None) => return self.message("That payment is no longer available."),
            Err(error) => return self.message(format!("{error:#}")),
        };
        let Some(payment) = context
            .share
            .state
            .payments
            .iter()
            .find(|item| item.payment.id == payment_id)
        else {
            return self.message("That payment is no longer available.");
        };

        let record = &payment.payment;
        let computed = payment.computed_change_minor;
        let returned = record.change_returned_at;
        let currency = self.currency.clone();
        let due = context.share.share.total_due_minor();
        let net = (record.amount_minor - computed).max(0);
        let tenant_name = context.share.display_name().to_string();
        let bill_reference = format!(
            "{} · {}",
            context.bill.bill_type_name_snapshot,
            money::format_period(context.bill.period_year, context.bill.period_month)
        );
        let has_balance_used = record.used_full_balance && record.is_active();

        self.detail_payment_id = Some(payment_id.to_string());
        self.detail_payment_share_id = Some(share_id.to_string());
        self.detail_returned_at = returned;

        let change_returned = if computed <= 0 {
            "—".to_string()
        } else if returned.is_some() {
            money::format_money(computed, &currency)
        } else {
            format!(
                "Change to Return {}",
                money::format_money(computed, &currency)
            )
        };
        let change_status = if computed <= 0 || !record.is_active() {
            String::new()
        } else if returned.is_some() {
            "Change Returned".to_string()
        } else {
            "Change to Return".to_string()
        };

        let ui = self.ui();
        ui.set_pd_subtitle(
            format!(
                "{} · {} · {}",
                context.share.display_name(),
                context.bill.bill_type_name_snapshot,
                money::format_period(context.bill.period_year, context.bill.period_month)
            )
            .into(),
        );
        ui.set_pd_total(money::format_money(due, &currency).into());
        ui.set_pd_received(money::format_money(record.amount_minor, &currency).into());
        ui.set_pd_tenant(tenant_name.into());
        ui.set_pd_bill_ref(bill_reference.into());
        ui.set_pd_payment_type(record.payment_type.label().into());
        ui.set_pd_balance_used(money::format_money(net, &currency).into());
        ui.set_pd_has_balance_used(has_balance_used);
        ui.set_pd_change_returned(change_returned.into());
        ui.set_pd_net(money::format_money(net, &currency).into());
        ui.set_pd_payment_status(record.status_label().into());
        ui.set_pd_change_status(change_status.into());
        ui.set_pd_recorded(money::format_datetime(record.paid_at).into());
        ui.set_pd_returned_on(
            returned
                .map(money::format_date)
                .unwrap_or_default()
                .into(),
        );
        ui.set_pd_undone(!record.is_active());
        ui.set_pd_undone_reason(record.void_reason.clone().unwrap_or_default().into());
        ui.set_pd_undone_on(
            record
                .voided_at
                .map(money::format_datetime)
                .unwrap_or_default()
                .into(),
        );
        ui.set_pd_has_returned(returned.is_some());
        ui.set_pd_can_return(record.is_active() && returned.is_none() && computed > 0);
        ui.set_pd_can_undo(record.is_active() && context.bill.archived_at.is_none());
    }

    fn refresh_open_payment_detail(&mut self) {
        if let (Some(payment_id), Some(share_id)) = (
            self.detail_payment_id.clone(),
            self.detail_payment_share_id.clone(),
        ) {
            self.show_payment_detail(&payment_id, &share_id);
        }
    }

    pub(super) fn payment_detail_correct_date(&mut self) {
        if self.detail_returned_at.is_none() {
            return self.message("This change has not been returned yet.");
        }
        self.change_correction_payment_id = self.detail_payment_id.clone();
        self.open_calendar(TARGET_CHANGE_RETURNED);
    }

    pub(super) fn payment_detail_return(&mut self) {
        let (Some(payment_id), Some(share_id)) = (
            self.detail_payment_id.clone(),
            self.detail_payment_share_id.clone(),
        ) else {
            return self.message("That payment is no longer available.");
        };
        self.prepare_return_change(&payment_id, &share_id);
    }

    /// Builds the Manage shares sheet for the open bill.
    pub(super) fn manage_shares(&mut self) {
        if self.detail_bill.is_none() {
            return self.message("Open a bill first.");
        }
        self.manage_share_ids = self
            .detail_shares
            .iter()
            .map(|entry| entry.id.clone())
            .collect();
        let rows: Vec<ManageRow> = self
            .detail_shares
            .iter()
            .map(|entry| ManageRow {
                name: entry.name.clone().into(),
                amount: money::format_money(entry.base_due, &self.currency).into(),
            })
            .collect();

        let on_bill: HashSet<&str> = self
            .detail_shares
            .iter()
            .map(|entry| entry.boarder_id.as_str())
            .collect();
        let mut candidate_ids = Vec::new();
        let mut candidate_names: Vec<SharedString> = Vec::new();
        match self.db.active_boarders() {
            Ok(boarders) => {
                for boarder in boarders {
                    if !on_bill.contains(boarder.id.as_str()) {
                        candidate_ids.push(boarder.id);
                        candidate_names.push(boarder.name.into());
                    }
                }
            }
            Err(error) => self.message(format!("{error:#}")),
        }
        self.manage_candidate_ids = candidate_ids;

        let ui = self.ui();
        ui.set_manage_rows(ModelRc::new(VecModel::from(rows)));
        ui.set_manage_candidates(ModelRc::new(VecModel::from(candidate_names)));
        ui.set_manage_subtitle(
            format!(
                "{} · Total {}",
                ui.get_detail_title(),
                money::format_money(self.detail_total_minor, &self.currency)
            )
            .into(),
        );
    }

    pub(super) fn manage_exclude(&mut self, index: usize) {
        let Some(bill_id) = self.detail_bill.clone() else {
            return self.message("Open a bill first.");
        };
        let Some(share_id) = self.manage_share_ids.get(index).cloned() else {
            return self.message("That share is no longer available.");
        };
        let Some(boarder_id) = self
            .detail_shares
            .iter()
            .find(|entry| entry.id == share_id)
            .map(|entry| entry.boarder_id.clone())
        else {
            return self.message("That share is no longer available.");
        };

        match self.db.exclude_share(&bill_id, &boarder_id) {
            Ok(()) => {
                self.refresh_after_change();
                self.manage_share_ids.clear();
                self.manage_candidate_ids.clear();
                self.notify("Boarder excluded from this bill.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn manage_add(&mut self, index: usize) {
        let Some(bill_id) = self.detail_bill.clone() else {
            return self.message("Open a bill first.");
        };
        let Some(boarder_id) = self.manage_candidate_ids.get(index).cloned() else {
            return self.message("That boarder is no longer available.");
        };

        match self.db.add_share_to_bill(&bill_id, &boarder_id) {
            Ok(()) => {
                self.refresh_after_change();
                self.manage_share_ids.clear();
                self.manage_candidate_ids.clear();
                self.notify("Boarder added to this bill.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn manage_edit(&mut self, index: usize) {
        let Some(share_id) = self.manage_share_ids.get(index).cloned() else {
            return self.message("That share is no longer available.");
        };
        // Editing is reached from the unfiltered manage list, so clear the
        // share filters before resolving the visible row index.
        self.share_filter.clear();
        self.share_filter_mode = 0;
        self.share_type_filter = 0;
        {
            let ui = self.ui();
            ui.set_share_filter("".into());
            ui.set_share_filter_mode(0);
            ui.set_share_type_filter(0);
        }
        self.render_shares();

        let Some(display_index) = self
            .display_share_ids
            .iter()
            .position(|id| *id == share_id)
        else {
            return self.message("That share is no longer available.");
        };
        self.open_edit_share(display_index);
    }

    /// Permanently deletes the open bill along with its shares and payments.
    pub(super) fn delete_bill(&mut self) {
        let Some(id) = self.detail_bill.clone() else {
            return self.message("Open a bill first.");
        };
        match self.db.delete_bill(&id) {
            Ok(()) => {
                self.detail_bill = None;
                self.clear_detail_lists();
                // Return to where the bill was opened from (ledger or tab).
                self.back();
                self.refresh_dashboard();
                self.refresh_history();
                self.refresh_boarders();
                // After back(), whose screen change clears the banner.
                self.notify("Bill deleted.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn archive_bill(&mut self) {
        let Some(id) = self.detail_bill.clone() else {
            return;
        };
        let archived = match self.db.bill_detail(&id) {
            Ok(Some(detail)) => detail.bill.archived_at.is_some(),
            _ => false,
        };
        match self.db.set_bill_archived(&id, !archived) {
            Ok(()) => {
                // Return to where the bill was opened from (ledger or tab).
                self.back();
                self.refresh_dashboard();
                // After back(), whose screen change clears the banner.
                self.notify(if archived {
                    "Bill restored."
                } else {
                    "Bill archived."
                });
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_open_bill(move |index| {
            let mut app = app.borrow_mut();
            let index = index.max(0) as usize;
            let from_ledger = app.ui().get_screen() == 11;
            app.open_bill(index, from_ledger);
        });
    }
    {
        let app = app.clone();
        ui.on_open_new_bill(move || app.borrow_mut().set_screen(12));
    }
    {
        let app = app.clone();
        ui.on_pick_toggle(move |index| app.borrow_mut().pick_toggle(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_pick_amount(move |index, amount| {
            app.borrow_mut().pick_amount(index.max(0) as usize, &amount)
        });
    }
    {
        let app = app.clone();
        ui.on_create_bill(move || app.borrow_mut().create_bill());
    }
    {
        let app = app.clone();
        ui.on_pay_share(move |index| app.borrow_mut().pay_share(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_select_all_picks(move || app.borrow_mut().select_all_picks());
    }
    {
        let app = app.clone();
        ui.on_undo_payment(move |index| {
            app.borrow_mut().open_undo_payment(index.max(0) as usize)
        });
    }
    {
        let app = app.clone();
        ui.on_undo_share_payment(move |index| {
            app.borrow_mut().open_undo_share(index.max(0) as usize)
        });
    }
    {
        let app = app.clone();
        ui.on_undo_detail_payment(move || app.borrow_mut().open_undo_detail());
    }
    {
        let app = app.clone();
        ui.on_undo_pick(move |index| app.borrow_mut().undo_pick(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_undo_confirm(move || app.borrow_mut().undo_confirm());
    }
    {
        let app = app.clone();
        ui.on_undo_cancel(move || app.borrow_mut().undo_cancel());
    }
    {
        let app = app.clone();
        ui.on_return_change(move |index| app.borrow_mut().return_change(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_archive_bill(move || app.borrow_mut().archive_bill());
    }
    {
        let app = app.clone();
        ui.on_delete_bill(move || app.borrow_mut().delete_bill());
    }
    {
        let app = app.clone();
        ui.on_set_share_filter(move |text| app.borrow_mut().set_share_filter(&text));
    }
    {
        let app = app.clone();
        ui.on_set_share_filter_mode(move |mode| {
            app.borrow_mut().set_share_filter_mode(mode)
        });
    }
    {
        let app = app.clone();
        ui.on_set_share_type_filter(move |mode| {
            app.borrow_mut().set_share_type_filter(mode)
        });
    }
    {
        let app = app.clone();
        ui.on_payment_amount_edited(move |index, value| {
            app.borrow_mut()
                .payment_amount_edited(index, &value)
        });
    }
    {
        let app = app.clone();
        ui.on_edit_share(move |index| {
            app.borrow_mut().open_edit_share(index.max(0) as usize)
        });
    }
    {
        let app = app.clone();
        ui.on_preview_share_split(move |amount, charge| {
            app.borrow_mut().preview_share_split(&amount, &charge)
        });
    }
    {
        let app = app.clone();
        ui.on_save_share(move || app.borrow_mut().save_share());
    }
    {
        let app = app.clone();
        ui.on_return_change_confirm(move || app.borrow_mut().return_change_confirm());
    }
    {
        let app = app.clone();
        ui.on_return_change_pick_date(move || app.borrow_mut().return_change_pick_date());
    }
    {
        let app = app.clone();
        ui.on_overpay_dismiss(move || app.borrow_mut().overpay_dismiss());
    }
    {
        let app = app.clone();
        ui.on_overpay_return_change(move || app.borrow_mut().overpay_return_change());
    }
    {
        let app = app.clone();
        ui.on_manage_shares(move || app.borrow_mut().manage_shares());
    }
    {
        let app = app.clone();
        ui.on_manage_edit(move |index| app.borrow_mut().manage_edit(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_manage_exclude(move |index| app.borrow_mut().manage_exclude(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_manage_add(move |index| app.borrow_mut().manage_add(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_open_payment(move |index| app.borrow_mut().open_payment(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_return_share_change(move |index| {
            app.borrow_mut().return_share_change(index.max(0) as usize)
        });
    }
    {
        let app = app.clone();
        ui.on_payment_detail_correct_date(move || {
            app.borrow_mut().payment_detail_correct_date()
        });
    }
    {
        let app = app.clone();
        ui.on_payment_detail_return(move || app.borrow_mut().payment_detail_return());
    }
}
