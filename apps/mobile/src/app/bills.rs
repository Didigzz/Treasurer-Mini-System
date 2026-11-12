use std::{cell::RefCell, rc::Rc};

use chrono::{Local, NaiveDate, TimeZone};
use slint::{ModelRc, SharedString, VecModel};

use treasurer_core::calculations::{equal_split, validate_custom_shares};
use treasurer_core::db::repository::now_ms;
use treasurer_core::models::*;
use treasurer_core::money;

use super::App;
use crate::{AppWindow, PaymentRow, PickRow, ShareRow};

pub(super) struct Pick {
    pub(super) boarder_id: String,
    pub(super) name: String,
    pub(super) selected: bool,
    pub(super) amount: String,
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
        ui.set_bill_total("".into());
        ui.set_bill_due("".into());
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
        self.ui().set_picks(ModelRc::new(VecModel::from(rows)));
    }

    pub(super) fn refresh_bill_detail(&mut self) {
        let Some(id) = self.detail_bill.clone() else {
            return;
        };
        let detail = match self.db.bill_detail(&id) {
            Ok(Some(detail)) => detail,
            Ok(None) => return self.message("Bill not found."),
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.share_ids = detail
            .shares
            .iter()
            .map(|share| share.share.id.clone())
            .collect();

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
        ui.set_detail_summary(
            format!(
                "{} expected · {} collected · {} outstanding · {} change · {}",
                money::format_money(detail.summary.expected_minor, &self.currency),
                money::format_money(detail.summary.applied_minor, &self.currency),
                money::format_money(detail.summary.outstanding_minor, &self.currency),
                money::format_money(detail.summary.change_outstanding_minor, &self.currency),
                detail.summary.status.label()
            )
            .into(),
        );

        let shares: Vec<ShareRow> = detail
            .shares
            .iter()
            .map(|share| ShareRow {
                name: share.display_name().to_string().into(),
                due: money::format_money(share.share.amount_due_minor, &self.currency).into(),
                paid: money::format_money(share.state.paid_minor, &self.currency).into(),
                balance: money::format_money(share.state.balance_minor, &self.currency).into(),
                change: money::format_money(share.state.change_outstanding_minor, &self.currency)
                    .into(),
                status: share.state.status.label().into(),
            })
            .collect();
        ui.set_shares(ModelRc::new(VecModel::from(shares)));

        let mut payment_rows: Vec<(i64, PaymentRow, String)> = Vec::new();
        for share in &detail.shares {
            for payment in &share.state.payments {
                let record = &payment.payment;
                let mut subtitle = money::format_datetime(record.paid_at);
                if let Some(note) = &record.note {
                    subtitle.push_str(&format!(" · {note}"));
                }
                if let Some(reason) = &record.void_reason {
                    subtitle.push_str(&format!(" · voided: {reason}"));
                } else if let Some(returned) = record.change_returned_at {
                    subtitle.push_str(&format!(
                        " · change returned {}",
                        money::format_date(returned)
                    ));
                }
                let can_return = record.is_active()
                    && record.change_returned_at.is_none()
                    && payment.computed_change_minor > 0;
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
                        change: money::format_money(payment.computed_change_minor, &self.currency)
                            .into(),
                        can_return,
                    },
                    record.id.clone(),
                ));
            }
        }
        payment_rows.sort_by(|a, b| b.0.cmp(&a.0));
        self.payment_ids = payment_rows.iter().map(|(_, _, id)| id.clone()).collect();
        ui.set_payments(ModelRc::new(VecModel::from(
            payment_rows
                .into_iter()
                .map(|(_, row, _)| row)
                .collect::<Vec<_>>(),
        )));
    }

    pub(super) fn open_bill(&mut self, index: usize) {
        let Some(id) = self
            .bill_ids
            .get(index)
            .or_else(|| self.ledger_bill_ids.get(index))
            .cloned()
        else {
            return;
        };
        self.detail_bill = Some(id);
        self.ui().set_payment_amount("".into());
        self.ui().set_payment_date("".into());
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

        let any_custom = selected.iter().any(|pick| !pick.amount.trim().is_empty());
        let shares: Vec<SplitResult> = if any_custom {
            let mut shares = Vec::with_capacity(selected.len());
            for pick in &selected {
                let amount = if pick.amount.trim().is_empty() {
                    0
                } else {
                    match money::parse_amount_input(&pick.amount, self.allow_centavos) {
                        Some(amount) => amount,
                        None => return self.message("Enter valid custom amounts."),
                    }
                };
                shares.push(SplitResult {
                    boarder_id: pick.boarder_id.clone(),
                    amount_minor: amount,
                });
            }
            let validation = validate_custom_shares(total_minor, &shares);
            if !validation.valid {
                return self.message(
                    validation
                        .error
                        .unwrap_or_else(|| "Check the split.".into()),
                );
            }
            shares
        } else {
            let boarders: Vec<SplitBoarder> = selected
                .iter()
                .map(|pick| SplitBoarder {
                    boarder_id: pick.boarder_id.clone(),
                    name: pick.name.clone(),
                    created_at: 0,
                })
                .collect();
            match equal_split(total_minor, &boarders, self.allow_centavos) {
                treasurer_core::calculations::SplitOutcome::Ok(success) => success.shares,
                treasurer_core::calculations::SplitOutcome::Err(failure) => {
                    return self.message(failure.error)
                }
            }
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
                self.message("Bill created.");
                self.detail_bill = None;
                self.set_screen(0);
                self.refresh_dashboard();
                self.refresh_history();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn pay_share(&mut self, index: usize) {
        let Some(share_id) = self.share_ids.get(index).cloned() else {
            return;
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
        let paid_at = parse_payment_date(&ui.get_payment_date());

        match self.db.record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor,
            paid_at,
            note: None,
        }) {
            Ok(_) => {
                ui.set_payment_amount("".into());
                self.message(format!(
                    "Payment of {} recorded.",
                    money::format_money(amount_minor, &self.currency)
                ));
                self.refresh_bill_detail();
                self.refresh_dashboard();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn void_payment(&mut self, index: usize) {
        let Some(payment_id) = self.payment_ids.get(index).cloned() else {
            return;
        };
        let reason = self.ui().get_void_reason().to_string();
        match self.db.void_payment(&payment_id, &reason) {
            Ok(()) => {
                self.message("Payment voided.");
                self.refresh_bill_detail();
                self.refresh_dashboard();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn return_change(&mut self, index: usize) {
        let Some(payment_id) = self.payment_ids.get(index).cloned() else {
            return;
        };
        match self.db.mark_change_returned(&payment_id, now_ms(), None) {
            Ok(()) => {
                self.message("Change marked as returned.");
                self.refresh_bill_detail();
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
                self.message(if archived {
                    "Bill restored."
                } else {
                    "Bill archived."
                });
                self.detail_bill = None;
                self.set_screen(0);
                self.refresh_dashboard();
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
            if app.ui().get_screen() == 11 {
                let id = app.ledger_bill_ids.get(index).cloned();
                if let Some(id) = id {
                    app.detail_bill = Some(id);
                    app.ui().set_payment_amount("".into());
                    app.ui().set_payment_date("".into());
                    app.set_screen(10);
                    app.refresh_bill_detail();
                }
            } else {
                app.open_bill(index);
            }
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
        ui.on_void_payment(move |index| app.borrow_mut().void_payment(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_return_change(move |index| app.borrow_mut().return_change(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_archive_bill(move || app.borrow_mut().archive_bill());
    }
}

pub(super) fn parse_payment_date(text: &SharedString) -> i64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return now_ms();
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        if let Some(datetime) = date.and_hms_opt(12, 0, 0) {
            if let Some(local) = Local.from_local_datetime(&datetime).single() {
                return local.timestamp_millis();
            }
        }
    }
    now_ms()
}
