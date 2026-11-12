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
            .map(|item| ShareRow {
                name: format!(
                    "{} {}",
                    item.bill.bill_type_name_snapshot,
                    money::format_short_period(item.bill.period_year, item.bill.period_month)
                )
                .into(),
                due: money::format_money(item.share.share.amount_due_minor, &self.currency).into(),
                paid: money::format_money(item.state.paid_minor, &self.currency).into(),
                balance: money::format_money(item.state.balance_minor, &self.currency).into(),
                balance_input: money::minor_to_input_value(item.state.balance_minor.max(0)).into(),
                change: money::format_money(item.state.change_outstanding_minor, &self.currency)
                    .into(),
                status: item.state.status.label().into(),
                has_balance: item.state.balance_minor > 0,
                has_change: item.state.change_outstanding_minor > 0,
            })
            .collect();
        ui.set_shares(ModelRc::new(VecModel::from(shares)));

        let payments: Vec<PaymentRow> = ledger
            .payments
            .iter()
            .map(|item| PaymentRow {
                title: format!(
                    "{} {}",
                    item.bill_type_name,
                    money::format_short_period(item.period_year, item.period_month)
                )
                .into(),
                subtitle: format!(
                    "{}{}",
                    money::format_datetime(item.payment.payment.paid_at),
                    if item.payment.payment.voided_at.is_some() {
                        " · voided"
                    } else {
                        ""
                    }
                )
                .into(),
                amount: money::format_money(item.payment.payment.amount_minor, &self.currency)
                    .into(),
                change: money::format_money(item.payment.computed_change_minor, &self.currency)
                    .into(),
                can_return: false,
                has_change: item.payment.computed_change_minor > 0,
                voided: item.payment.payment.voided_at.is_some(),
            })
            .collect();
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
}
