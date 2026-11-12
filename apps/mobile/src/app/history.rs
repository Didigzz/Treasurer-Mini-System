use std::{cell::RefCell, rc::Rc};

use slint::{ModelRc, VecModel};

use treasurer_core::models::*;
use treasurer_core::money;

use super::{progress, App};
use crate::{AppWindow, HistoryRow};

impl App {
    pub(super) fn refresh_history(&mut self) {
        let data = match self.db.bill_history() {
            Ok(data) => data,
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.history_slots = data
            .periods
            .iter()
            .map(|period| (period.year, period.month))
            .collect();

        let rows: Vec<HistoryRow> = data
            .periods
            .iter()
            .map(|period| {
                let bills: Vec<&BillWithDetails> = data
                    .bills
                    .iter()
                    .filter(|item| {
                        item.bill.period_year == period.year
                            && item.bill.period_month == period.month
                    })
                    .collect();
                let expected: i64 = bills.iter().map(|item| item.summary.expected_minor).sum();
                let applied: i64 = bills.iter().map(|item| item.summary.applied_minor).sum();
                let outstanding: i64 = bills
                    .iter()
                    .map(|item| item.summary.outstanding_minor)
                    .sum();

                HistoryRow {
                    period: money::format_period(period.year, period.month).into(),
                    bills: format!(
                        "{} bill{}",
                        period.bill_count,
                        if period.bill_count == 1 { "" } else { "s" }
                    )
                    .into(),
                    expected: money::format_money(expected, &self.currency).into(),
                    applied: money::format_money(applied, &self.currency).into(),
                    outstanding: money::format_money(outstanding, &self.currency).into(),
                    progress: progress(applied, expected),
                    settled: outstanding <= 0,
                }
            })
            .collect();
        self.ui().set_history(ModelRc::new(VecModel::from(rows)));
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_open_history(move |index| {
            let mut app = app.borrow_mut();
            if let Some((year, month)) = app.history_slots.get(index.max(0) as usize).copied() {
                app.year = year;
                app.month = month;
                app.set_screen(0);
                app.refresh_dashboard();
            }
        });
    }
}
