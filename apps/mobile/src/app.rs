use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, SecondsFormat, TimeZone};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use treasurer_core::calculations::{equal_split, matches_boarder_filter, validate_custom_shares};
use treasurer_core::db::repository::{now_ms, Treasurer};
use treasurer_core::models::*;
use treasurer_core::money::{self, CURRENCY_OPTIONS};

use crate::{
    AppWindow, BalanceRow, BillRow, BoarderRow, HistoryRow, PaymentRow, PickRow, ShareRow, TypeRow,
};

pub fn run() -> Result<()> {
    let db_path = crate::platform::db_path();
    let db = Treasurer::open(&db_path)
        .with_context(|| format!("Could not open the database at {}", db_path.display()))?;

    let ui = AppWindow::new()?;
    let app = Rc::new(RefCell::new(App::new(ui.as_weak(), db)));
    App::wire(app.clone());
    app.borrow_mut().start();

    ui.run()?;
    Ok(())
}

struct Pick {
    boarder_id: String,
    name: String,
    selected: bool,
    amount: String,
}

struct App {
    ui: slint::Weak<AppWindow>,
    db: Treasurer,
    year: i32,
    month: i32,
    filter: ShareFilter,
    include_archived: bool,
    currency: String,
    allow_centavos: bool,
    bill_ids: Vec<String>,
    balance_ids: Vec<String>,
    boarder_ids: Vec<String>,
    boarder_archived: Vec<bool>,
    share_ids: Vec<String>,
    payment_ids: Vec<String>,
    history_slots: Vec<(i32, i32)>,
    type_ids: Vec<String>,
    type_archived: Vec<bool>,
    form_type_ids: Vec<String>,
    pick_rows: Vec<Pick>,
    detail_bill: Option<String>,
    detail_boarder: Option<String>,
    ledger_bill_ids: Vec<String>,
}

impl App {
    fn new(ui: slint::Weak<AppWindow>, db: Treasurer) -> Self {
        let (year, month) = money::current_period();
        Self {
            ui,
            db,
            year,
            month,
            filter: ShareFilter::All,
            include_archived: false,
            currency: "PHP".to_string(),
            allow_centavos: true,
            bill_ids: Vec::new(),
            balance_ids: Vec::new(),
            boarder_ids: Vec::new(),
            boarder_archived: Vec::new(),
            share_ids: Vec::new(),
            payment_ids: Vec::new(),
            history_slots: Vec::new(),
            type_ids: Vec::new(),
            type_archived: Vec::new(),
            form_type_ids: Vec::new(),
            pick_rows: Vec::new(),
            detail_bill: None,
            detail_boarder: None,
            ledger_bill_ids: Vec::new(),
        }
    }

    fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("window is alive")
    }

    fn message(&self, text: impl Into<SharedString>) {
        self.ui().set_message(text.into());
    }

    fn start(&mut self) {
        self.load_settings();
        self.refresh_dashboard();
        self.refresh_boarders();
        self.refresh_history();
        self.refresh_settings();
        self.load_form_types();
        self.ui().set_app_version("Boarding House Treasurer 1.1.0 · Cargo/Slint".into());
        self.ui().set_screen_title("Dashboard".into());
    }

    fn load_settings(&mut self) {
        match self.db.app_settings() {
            Ok(settings) => {
                self.currency = settings.currency_code;
                self.allow_centavos = settings.allow_centavos;
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn set_screen(&mut self, screen: i32) {
        let ui = self.ui();
        ui.set_screen(screen);
        ui.set_message("".into());
        ui.set_screen_title(
            match screen {
                0 => "Dashboard",
                1 => "Boarders",
                3 => "History",
                4 => "Settings",
                10 => "Bill",
                11 => "Boarder ledger",
                12 => "New bill",
                _ => "Treasurer",
            }
            .into(),
        );

        match screen {
            0 => self.refresh_dashboard(),
            1 => self.refresh_boarders(),
            3 => self.refresh_history(),
            4 => self.refresh_settings(),
            12 => self.open_new_bill_form(),
            _ => {}
        }
    }

    fn back(&mut self) {
        let target = match self.ui().get_screen() {
            10 => 0,
            11 => 1,
            12 => 0,
            _ => 0,
        };
        self.detail_bill = None;
        self.detail_boarder = None;
        self.set_screen(target);
    }

    fn refresh_dashboard(&mut self) {
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
        ui.set_total_expected(money::format_money(data.totals.expected_minor, &self.currency).into());
        ui.set_total_paid(money::format_money(data.totals.applied_minor, &self.currency).into());
        ui.set_total_outstanding(
            money::format_money(data.totals.outstanding_minor, &self.currency).into(),
        );
        ui.set_total_change(
            money::format_money(data.totals.change_outstanding_minor, &self.currency).into(),
        );
        ui.set_archived_note(
            if data.archived_bill_count > 0 {
                format!(
                    "{} archived bill{} hidden",
                    data.archived_bill_count,
                    if data.archived_bill_count == 1 { "" } else { "s" }
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
                amount: money::format_money(summary.outstanding_minor, &self.currency).into(),
                change: money::format_money(summary.change_outstanding_minor, &self.currency)
                    .into(),
            })
            .collect();
        ui.set_balances(ModelRc::new(VecModel::from(balances)));
    }

    fn refresh_boarders(&mut self) {
        let rows = match self.db.list_boarders_with_balances() {
            Ok(rows) => rows,
            Err(error) => return self.message(format!("{error:#}")),
        };

        self.boarder_ids = rows
            .iter()
            .map(|row| row.boarder.id.clone())
            .collect();
        self.boarder_archived = rows
            .iter()
            .map(|row| row.boarder.status == BoarderStatus::Archived)
            .collect();

        let ui = self.ui();
        let model: Vec<BoarderRow> = rows
            .iter()
            .map(|row| BoarderRow {
                name: row.boarder.name.clone().into(),
                amount: money::format_money(row.outstanding_minor, &self.currency).into(),
                change: money::format_money(row.change_outstanding_minor, &self.currency).into(),
                archived: row.boarder.status == BoarderStatus::Archived,
            })
            .collect();
        ui.set_boarders(ModelRc::new(VecModel::from(model)));
    }

    fn refresh_history(&mut self) {
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
                let outstanding: i64 = bills.iter().map(|item| item.summary.outstanding_minor).sum();

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
                }
            })
            .collect();
        self.ui().set_history(ModelRc::new(VecModel::from(rows)));
    }

    fn refresh_settings(&mut self) {
        let settings = match self.db.app_settings() {
            Ok(settings) => settings,
            Err(error) => return self.message(format!("{error:#}")),
        };
        self.currency = settings.currency_code.clone();
        self.allow_centavos = settings.allow_centavos;

        let types = match self.db.list_bill_types() {
            Ok(types) => types,
            Err(error) => return self.message(format!("{error:#}")),
        };
        self.type_ids = types
            .iter()
            .map(|row| row.bill_type.id.clone())
            .collect();
        self.type_archived = types.iter().map(|row| row.bill_type.is_archived).collect();

        let ui = self.ui();
        let currency_names: Vec<SharedString> = CURRENCY_OPTIONS
            .iter()
            .map(|(_, label)| SharedString::from(*label))
            .collect();
        ui.set_currency_names(ModelRc::new(VecModel::from(currency_names)));

        if let Some(index) = CURRENCY_OPTIONS
            .iter()
            .position(|(code, _)| *code == self.currency)
        {
            ui.set_currency_index(index as i32);
        }
        ui.set_allow_centavos(self.allow_centavos);

        ui.set_last_backup(
            settings
                .last_backup_at
                .as_deref()
                .map(format_iso_date)
                .unwrap_or_else(|| "never".to_string())
                .into(),
        );

        let rows: Vec<TypeRow> = types
            .iter()
            .map(|row| TypeRow {
                name: row.bill_type.name.clone().into(),
                count: format!(
                    "{} bill{}",
                    row.bill_count,
                    if row.bill_count == 1 { "" } else { "s" }
                )
                .into(),
                archived: row.bill_type.is_archived,
            })
            .collect();
        ui.set_types(ModelRc::new(VecModel::from(rows)));
    }

    fn load_form_types(&mut self) {
        let types = match self.db.list_bill_types() {
            Ok(types) => types,
            Err(_) => return,
        };
        let active: Vec<_> = types
            .iter()
            .filter(|row| !row.bill_type.is_archived)
            .collect();
        self.form_type_ids = active
            .iter()
            .map(|row| row.bill_type.id.clone())
            .collect();
        let names: Vec<SharedString> = active
            .iter()
            .map(|row| SharedString::from(row.bill_type.name.clone()))
            .collect();
        self.ui()
            .set_bill_type_names(ModelRc::new(VecModel::from(names)));
    }

    fn open_new_bill_form(&mut self) {
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

    fn render_picks(&self) {
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

    fn refresh_bill_detail(&mut self) {
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
        let mut subtitle = format!("Total {}", money::format_money(detail.bill.total_minor, &self.currency));
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
                    subtitle.push_str(&format!(" · change returned {}", money::format_date(returned)));
                }
                let can_return = record.is_active()
                    && record.change_returned_at.is_none()
                    && payment.computed_change_minor > 0;
                payment_rows.push((
                    record.paid_at,
                    PaymentRow {
                        title: format!("{} · {}", share.display_name(), money::format_date(record.paid_at))
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
        self.payment_ids = payment_rows
            .iter()
            .map(|(_, _, id)| id.clone())
            .collect();
        ui.set_payments(ModelRc::new(VecModel::from(
            payment_rows.into_iter().map(|(_, row, _)| row).collect::<Vec<_>>(),
        )));
    }

    fn refresh_ledger(&mut self) {
        let Some(id) = self.detail_boarder.clone() else {
            return;
        };
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
        ui.set_detail_title(ledger.boarder.name.clone().into());
        ui.set_detail_summary(
            format!(
                "{} expected · {} collected · {} outstanding · {} change",
                money::format_money(ledger.summary.expected_minor, &self.currency),
                money::format_money(ledger.summary.applied_minor, &self.currency),
                money::format_money(ledger.summary.outstanding_minor, &self.currency),
                money::format_money(ledger.summary.change_outstanding_minor, &self.currency)
            )
            .into(),
        );

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
                change: money::format_money(item.state.change_outstanding_minor, &self.currency)
                    .into(),
                status: item.state.status.label().into(),
            })
            .collect();
        ui.set_shares(ModelRc::new(VecModel::from(shares)));

        let payments: Vec<PaymentRow> = ledger
            .payments
            .iter()
            .map(|item| PaymentRow {
                title: format!(
                    "{} {} · {}",
                    item.bill_type_name,
                    money::format_short_period(item.period_year, item.period_month),
                    money::format_money(item.payment.payment.amount_minor, &self.currency)
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
                change: money::format_money(
                    item.payment.computed_change_minor,
                    &self.currency,
                )
                .into(),
                can_return: false,
            })
            .collect();
        ui.set_payments(ModelRc::new(VecModel::from(payments)));
    }

    fn open_bill(&mut self, index: usize) {
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

    fn open_boarder(&mut self, index: usize) {
        let Some(id) = self.balance_ids.get(index).cloned() else {
            return;
        };
        self.detail_boarder = Some(id);
        self.set_screen(11);
        self.refresh_ledger();
    }

    fn open_ledger(&mut self, index: usize) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return;
        };
        self.detail_boarder = Some(id);
        self.set_screen(11);
        self.refresh_ledger();
    }

    fn pick_toggle(&mut self, index: usize) {
        if let Some(pick) = self.pick_rows.get_mut(index) {
            pick.selected = !pick.selected;
        }
        self.render_picks();
    }

    fn pick_amount(&mut self, index: usize, amount: &str) {
        if let Some(pick) = self.pick_rows.get_mut(index) {
            pick.amount = amount.to_string();
        }
    }

    fn create_bill(&mut self) {
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

        let any_custom = selected
            .iter()
            .any(|pick| !pick.amount.trim().is_empty());
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
                return self.message(validation.error.unwrap_or_else(|| "Check the split.".into()));
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

    fn pay_share(&mut self, index: usize) {
        let Some(share_id) = self.share_ids.get(index).cloned() else {
            return;
        };
        let ui = self.ui();
        let amount_text = ui.get_payment_amount();
        let Some(amount_minor) = money::parse_amount_input(&amount_text, self.allow_centavos) else {
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

    fn void_payment(&mut self, index: usize) {
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

    fn return_change(&mut self, index: usize) {
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

    fn archive_bill(&mut self) {
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

    fn add_boarder(&mut self, name: SharedString) {
        match self.db.create_boarder(&name) {
            Ok(_) => {
                self.ui().set_new_boarder_name("".into());
                self.message("Boarder added.");
                self.refresh_boarders();
                self.refresh_dashboard();
                self.load_form_types();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn rename_boarder(&mut self, index: usize, name: SharedString) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return;
        };
        match self.db.rename_boarder(&id, &name) {
            Ok(()) => {
                self.ui().set_rename_name("".into());
                self.message("Boarder renamed.");
                self.refresh_boarders();
                self.refresh_dashboard();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn archive_boarder(&mut self, index: usize) {
        let Some(id) = self.boarder_ids.get(index).cloned() else {
            return;
        };
        let archived = self
            .boarder_archived
            .get(index)
            .copied()
            .unwrap_or(false);
        match self.db.set_boarder_archived(&id, !archived) {
            Ok(()) => {
                self.message(if archived {
                    "Boarder restored."
                } else {
                    "Boarder archived."
                });
                self.refresh_boarders();
                self.refresh_dashboard();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn add_bill_type(&mut self, name: SharedString) {
        match self.db.create_bill_type(&name) {
            Ok(_) => {
                self.ui().set_type_name("".into());
                self.message("Bill type added.");
                self.refresh_settings();
                self.load_form_types();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn archive_type(&mut self, index: usize) {
        let Some(id) = self.type_ids.get(index).cloned() else {
            return;
        };
        let archived = self.type_archived.get(index).copied().unwrap_or(false);
        match self.db.set_bill_type_archived(&id, !archived) {
            Ok(()) => {
                self.message(if archived {
                    "Bill type restored."
                } else {
                    "Bill type archived."
                });
                self.refresh_settings();
                self.load_form_types();
                self.refresh_dashboard();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn save_settings(&mut self) {
        let ui = self.ui();
        let index = ui.get_currency_index().max(0) as usize;
        let Some((code, _)) = CURRENCY_OPTIONS.get(index) else {
            return self.message("Choose a currency.");
        };
        let allow = ui.get_allow_centavos();

        if let Err(error) = self.db.set_currency_code(code) {
            return self.message(format!("{error:#}"));
        }
        if let Err(error) = self.db.set_allow_centavos(allow) {
            return self.message(format!("{error:#}"));
        }

        self.currency = (*code).to_string();
        self.allow_centavos = allow;
        self.message("Settings saved.");
        self.refresh_dashboard();
        self.refresh_settings();
    }

    fn export_backup(&mut self) {
        let json = match self.db.export_backup_json() {
            Ok(json) => json,
            Err(error) => return self.message(format!("{error:#}")),
        };

        let dir = crate::platform::backup_dir();
        if let Err(error) = std::fs::create_dir_all(&dir) {
            return self.message(format!("Could not create {}: {error}", dir.display()));
        }
        let stamp = Local::now().format("%Y-%m-%d-%H%M").to_string();
        let path = dir.join(format!("boarding-house-treasurer-backup-{stamp}.json"));
        if let Err(error) = std::fs::write(&path, json) {
            return self.message(format!("Could not write the backup: {error}"));
        }

        let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let _ = self.db.set_last_backup_at(&now);
        self.message(format!("Backup saved to {}", path.display()));
        self.refresh_settings();
    }

    fn import_backup(&mut self) {
        let dir = crate::platform::backup_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                return self.message(format!(
                    "No backup folder at {} ({error}). Export one first.",
                    dir.display()
                ))
            }
        };

        let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            if newest.as_ref().map(|(time, _)| modified > *time).unwrap_or(true) {
                newest = Some((modified, path));
            }
        }

        let Some((_, path)) = newest else {
            return self.message(format!("No backup files in {}.", dir.display()));
        };
        let json = match std::fs::read_to_string(&path) {
            Ok(json) => json,
            Err(error) => return self.message(format!("Could not read the backup: {error}")),
        };

        match self.db.import_backup_json(&json) {
            Ok(()) => {
                self.message(format!("Restored from {}.", path.file_name().and_then(|name| name.to_str()).unwrap_or("backup")));
                self.load_settings();
                self.refresh_dashboard();
                self.refresh_boarders();
                self.refresh_history();
                self.refresh_settings();
                self.load_form_types();
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    fn wire(app: Rc<RefCell<App>>) {
        let ui = app.borrow().ui();

        {
            let app = app.clone();
            ui.on_nav(move |screen| app.borrow_mut().set_screen(screen));
        }
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
            ui.on_open_boarder(move |index| {
                app.borrow_mut().open_boarder(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_open_ledger(move |index| {
                app.borrow_mut().open_ledger(index.max(0) as usize)
            });
        }
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
        {
            let app = app.clone();
            ui.on_back(move || app.borrow_mut().back());
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
            ui.on_archive_boarder(move |index| {
                app.borrow_mut().archive_boarder(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_add_bill_type(move |name| app.borrow_mut().add_bill_type(name));
        }
        {
            let app = app.clone();
            ui.on_archive_type(move |index| {
                app.borrow_mut().archive_type(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_open_new_bill(move || app.borrow_mut().set_screen(12));
        }
        {
            let app = app.clone();
            ui.on_pick_toggle(move |index| {
                app.borrow_mut().pick_toggle(index.max(0) as usize)
            });
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
            ui.on_pay_share(move |index| {
                app.borrow_mut().pay_share(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_void_payment(move |index| {
                app.borrow_mut().void_payment(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_return_change(move |index| {
                app.borrow_mut().return_change(index.max(0) as usize)
            });
        }
        {
            let app = app.clone();
            ui.on_archive_bill(move || app.borrow_mut().archive_bill());
        }
        {
            let app = app.clone();
            ui.on_export_backup(move || app.borrow_mut().export_backup());
        }
        {
            let app = app.clone();
            ui.on_import_backup(move || app.borrow_mut().import_backup());
        }
        {
            let app = app.clone();
            ui.on_save_settings(move || app.borrow_mut().save_settings());
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
}

fn parse_payment_date(text: &SharedString) -> i64 {
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

fn format_iso_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| money::format_date(date.timestamp_millis()))
        .unwrap_or_else(|_| value.to_string())
}
