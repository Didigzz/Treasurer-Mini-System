use slint::{ModelRc, VecModel};

use treasurer_core::calculations::matches_boarder_filter;
use treasurer_core::models::*;
use treasurer_core::money;

use super::{initials, progress, App};
use crate::{AppWindow, BalanceRow, BillRow};
use std::{cell::RefCell, rc::Rc};

impl App {
    pub(super) fn refresh_dashboard(&mut self) {
        let data = match self
            .db
            .dashboard(self.year, self.month, self.include_archived)
        {
            Ok(data) => data,
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.bill_ids = data.bills.iter().map(|item| item.bill.id.clone()).collect();

        let ui = self.ui();
        ui.set_include_archived(self.include_archived);
        ui.set_period_label(money::format_period(self.year, self.month).into());
        ui.set_total_expected(
            money::format_money(data.totals.expected_minor, &self.currency).into(),
        );
        ui.set_total_paid(money::format_money(data.totals.applied_minor, &self.currency).into());
        ui.set_total_outstanding(
            money::format_money(data.totals.outstanding_minor, &self.currency).into(),
        );
        ui.set_total_change(
            money::format_money(data.totals.change_outstanding_minor, &self.currency).into(),
        );
        ui.set_total_progress(progress(
            data.totals.applied_minor,
            data.totals.expected_minor,
        ));
        ui.set_archived_note(
            if data.archived_bill_count > 0 {
                format!(
                    "{} archived bill{} hidden",
                    data.archived_bill_count,
                    if data.archived_bill_count == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            } else {
                String::new()
            }
            .into(),
        );

        let rows: Vec<BillRow> = data
            .bills
            .iter()
            .map(|item| {
                let subtitle = match &item.bill.due_date {
                    Some(due) => format!(
                        "{} · {} payments · due {}",
                        money::format_period(item.bill.period_year, item.bill.period_month),
                        item.shares.len(),
                        money::format_date_only(due)
                    ),
                    None => format!(
                        "{} · {} payments",
                        money::format_period(item.bill.period_year, item.bill.period_month),
                        item.shares.len()
                    ),
                };
                BillRow {
                    title: item.current_type_name.clone().into(),
                    subtitle: subtitle.into(),
                    amount: money::format_money(item.summary.expected_minor, &self.currency).into(),
                    paid: money::format_money(item.summary.applied_minor, &self.currency).into(),
                    status: item.summary.status.label().into(),
                    progress: item.summary.progress as f32,
                    review: item.summary.needs_review,
                }
            })
            .collect();
        ui.set_bills(ModelRc::new(VecModel::from(rows)));

        let filtered: Vec<&BoarderPeriodSummary> = data
            .boarder_summaries
            .iter()
            .filter(|summary| matches_boarder_filter(summary, self.filter))
            .collect();
        self.balance_ids = filtered
            .iter()
            .map(|summary| summary.boarder_id.clone())
            .collect();
        let balances: Vec<BalanceRow> = filtered
            .iter()
            .map(|summary| BalanceRow {
                name: summary.boarder_name.clone().into(),
                initials: initials(&summary.boarder_name).into(),
                amount: money::format_money(summary.outstanding_minor, &self.currency).into(),
                change: money::format_money(summary.change_outstanding_minor, &self.currency)
                    .into(),
                has_due: summary.outstanding_minor > 0,
                has_change: summary.change_outstanding_minor > 0,
            })
            .collect();
        ui.set_balances(ModelRc::new(VecModel::from(balances)));
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_prev_month(move || {
            let mut app = app.borrow_mut();
            let (year, month) = money::shift_period(app.year, app.month, -1);
            app.year = year;
            app.month = month;
            app.refresh_dashboard();
        });
    }
    {
        let app = app.clone();
        ui.on_next_month(move || {
            let mut app = app.borrow_mut();
            let (year, month) = money::shift_period(app.year, app.month, 1);
            app.year = year;
            app.month = month;
            app.refresh_dashboard();
        });
    }
    {
        let app = app.clone();
        ui.on_set_filter(move |filter| {
            let mut app = app.borrow_mut();
            app.filter = match filter {
                1 => ShareFilter::Unpaid,
                2 => ShareFilter::Paid,
                3 => ShareFilter::Change,
                _ => ShareFilter::All,
            };
            app.ui().set_filter(filter);
            app.refresh_dashboard();
        });
    }
    {
        let app = app.clone();
        ui.on_toggle_archived(move || {
            let mut app = app.borrow_mut();
            app.include_archived = !app.include_archived;
            app.refresh_dashboard();
        });
    }
}
