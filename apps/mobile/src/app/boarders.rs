use std::{cell::RefCell, rc::Rc};

use slint::{ModelRc, SharedString, VecModel};

use treasurer_core::models::*;
use treasurer_core::money;

use super::{initials, progress, App};
use crate::{AppWindow, BoarderRow, PaymentRow, ShareRow};

impl App {
    pub(super) fn refresh_boarders(&mut self) {
        let rows = match self.db.list_boarders_with_balances() {
            Ok(rows) => rows,
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.boarder_ids = rows.iter().map(|row| row.boarder.id.clone()).collect();
        self.boarder_archived = rows
            .iter()
            .map(|row| row.boarder.status == BoarderStatus::Archived)
            .collect();

        let ui = self.ui();
        let model: Vec<BoarderRow> = rows
            .iter()
            .map(|row| BoarderRow {
                name: row.boarder.name.clone().into(),
                initials: initials(&row.boarder.name).into(),
                amount: money::format_money(row.outstanding_minor, &self.currency).into(),
                change: money::format_money(row.change_outstanding_minor, &self.currency).into(),
                archived: row.boarder.status == BoarderStatus::Archived,
                has_due: row.outstanding_minor > 0,
                has_change: row.change_outstanding_minor > 0,
            })
            .collect();
        ui.set_boarders(ModelRc::new(VecModel::from(model)));
    }

    pub(super) fn refresh_ledger(&mut self) {
        let Some(id) = self.detail_boarder.clone() else {
            return;
        };
        self.clear_detail_lists();
        let ledger = match self.db.boarder_ledger(&id) {
            Ok(Some(ledger)) => ledger,
            Ok(None) => return self.message("Boarder not found."),
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.ledger_bill_ids = ledger
            .items
            .iter()
            .map(|item| item.bill.id.clone())
            .collect();

        let ui = self.ui();
        let summary = &ledger.summary;
        ui.set_detail_title(ledger.boarder.name.clone().into());
        ui.set_detail_initials(initials(&ledger.boarder.name).into());
        ui.set_detail_expected(money::format_money(summary.expected_minor, &self.currency).into());
        ui.set_detail_collected(money::format_money(summary.applied_minor, &self.currency).into());
        ui.set_detail_outstanding(
            money::format_money(summary.outstanding_minor, &self.currency).into(),
        );
        ui.set_detail_change(
            money::format_money(summary.change_outstanding_minor, &self.currency).into(),
        );
        ui.set_detail_progress(progress(summary.applied_minor, summary.expected_minor));

        let shares: Vec<ShareRow> = ledger
            .items
            .iter()
            .map(|item| {
                let outstanding = item.state.change_outstanding_minor;
                let returned = item.state.change_returned_minor;
                let change_status = if outstanding > 0 {
                    if returned > 0 {
                        format!(
                            "Change Returned {} · {} to return",
                            money::format_money(returned, &self.currency),
                            money::format_money(outstanding, &self.currency)
                        )
                    } else {
                        format!(
                            "Change to Return {}",
                            money::format_money(outstanding, &self.currency)
                        )
                    }
                } else if returned > 0 {
                    format!(
                        "Change Returned {}",
                        money::format_money(returned, &self.currency)
                    )
                } else {
                    String::new()
                };

                ShareRow {
                    name: format!(
                        "{} {}",
                        item.bill.bill_type_name_snapshot,
                        money::format_short_period(item.bill.period_year, item.bill.period_month)
                    )
                    .into(),
                    due: money::format_money(item.share.share.total_due_minor(), &self.currency)
                        .into(),
                    paid: money::format_money(item.state.paid_minor, &self.currency).into(),
                    balance: money::format_money(item.state.balance_minor, &self.currency).into(),
                    balance_input: money::minor_to_input_value(item.state.balance_minor.max(0))
                        .into(),
                    change: money::format_money(outstanding, &self.currency).into(),
                    change_status: change_status.clone().into(),
                    charge: money::format_money(
                        item.share.share.additional_charge_minor,
                        &self.currency,
                    )
                    .into(),
                    status: item.state.status.label().into(),
                    settled: item.state.balance_minor == 0,
                    has_balance: item.state.balance_minor > 0,
                    has_change: outstanding > 0,
                    has_change_status: !change_status.is_empty(),
                    has_charge: item.share.share.additional_charge_minor > 0,
                }
            })
            .collect();
        ui.set_shares(ModelRc::new(VecModel::from(shares)));

        let currency = self.currency.clone();
        let mut payment_ids = Vec::with_capacity(ledger.payments.len());
        let mut payment_share_ids = Vec::with_capacity(ledger.payments.len());
        let payments: Vec<PaymentRow> = ledger
            .payments
            .iter()
            .map(|item| {
                let record = &item.payment.payment;
                let computed = item.payment.computed_change_minor;
                let change_status = if !record.is_active() || computed <= 0 {
                    String::new()
                } else if record.change_returned_at.is_some() {
                    format!(
                        "Change Returned {}",
                        money::format_money(computed, &currency)
                    )
                } else {
                    format!(
                        "Change to Return {}",
                        money::format_money(computed, &currency)
                    )
                };

                payment_ids.push(record.id.clone());
                payment_share_ids.push(record.bill_share_id.clone());

                PaymentRow {
                    title: format!(
                        "{} {}",
                        item.bill_type_name,
                        money::format_short_period(item.period_year, item.period_month)
                    )
                    .into(),
                    subtitle: format!(
                        "{}{}",
                        money::format_datetime(record.paid_at),
                        if record.voided_at.is_some() {
                            " · undone"
                        } else {
                            ""
                        }
                    )
                    .into(),
                    amount: money::format_money(record.amount_minor, &currency).into(),
                    payment_type: record.payment_type.label().into(),
                    status: record.status_label().into(),
                    change: money::format_money(computed, &currency).into(),
                    change_status: change_status.into(),
                    can_return: record.is_active()
                        && record.change_returned_at.is_none()
                        && computed > 0,
                    has_change: computed > 0,
                    voided: !record.is_active(),
                }
            })
            .collect();
        self.payment_ids = payment_ids;
        self.payment_share_ids = payment_share_ids;
        ui.set_payments(ModelRc::new(VecModel::from(payments)));
    }

    pub(super) fn open_boarder(&mut self, index: usize) {
        let Some(id) = self.balance_ids.get(index).cloned() else {
            return;
        };
        self.detail_boarder = Some(id);
        self.set_screen(11);
        self.refresh_ledger();
    }

    pub(super) fn open_ledger(&mut self, index: usize) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return;
        };
        self.detail_boarder = Some(id);
        self.set_screen(11);
        self.refresh_ledger();
    }

    pub(super) fn add_boarder(&mut self, name: SharedString) {
        match self.db.create_boarder(&name) {
            Ok(_) => {
                self.ui().set_new_boarder_name("".into());
                self.refresh_boarders();
                self.refresh_dashboard();
                self.load_form_types();
                self.notify("Boarder added.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn rename_boarder(&mut self, index: usize, name: SharedString) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return self.message("That boarder is no longer in the list.");
        };
        match self.db.rename_boarder(&id, &name) {
            Ok(()) => {
                self.ui().set_rename_name("".into());
                self.refresh_boarders();
                self.refresh_dashboard();
                self.notify("Boarder renamed.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn delete_boarder(&mut self, index: usize) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return self.message("That boarder is no longer in the list.");
        };
        match self.db.delete_boarder(&id) {
            Ok(()) => {
                if self.detail_boarder.as_deref() == Some(id.as_str()) {
                    self.detail_boarder = None;
                }
                self.refresh_boarders();
                self.refresh_dashboard();
                self.notify("Boarder deleted.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn archive_boarder(&mut self, index: usize) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return;
        };
        let archived = self.boarder_archived.get(index).copied().unwrap_or(false);
        match self.db.set_boarder_archived(&id, !archived) {
            Ok(()) => {
                self.refresh_boarders();
                self.refresh_dashboard();
                self.notify(if archived {
                    "Boarder restored."
                } else {
                    "Boarder archived."
                });
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_open_boarder(move |index| app.borrow_mut().open_boarder(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_open_ledger(move |index| app.borrow_mut().open_ledger(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_add_boarder(move |name| app.borrow_mut().add_boarder(name));
    }
    {
        let app = app.clone();
        ui.on_rename_boarder(move |index, name| {
            app.borrow_mut().rename_boarder(index.max(0) as usize, name)
        });
    }
    {
        let app = app.clone();
        ui.on_archive_boarder(move |index| app.borrow_mut().archive_boarder(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_delete_boarder(move |index| app.borrow_mut().delete_boarder(index.max(0) as usize));
    }
}
