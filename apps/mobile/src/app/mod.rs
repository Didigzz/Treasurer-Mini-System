mod backup;
mod bills;
mod boarders;
mod calendar;
mod dashboard;
mod history;
mod settings;
mod update;

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use treasurer_core::db::repository::Treasurer;
use treasurer_core::models::*;
use treasurer_core::money;

use self::bills::{Pick, ShareEntry};

use crate::update::UpdateStatus;
use crate::version;
use crate::AppWindow;

pub fn run() -> Result<()> {
    let db_path = crate::platform::db_path();
    let db = Treasurer::open(&db_path)
        .with_context(|| format!("Could not open the database at {}", db_path.display()))?;

    let ui = AppWindow::new()?;
    let app = Rc::new(RefCell::new(App::new(ui.as_weak(), db)));
    App::wire(app.clone());
    start_update_loop(app.clone());
    app.borrow_mut().start();

    ui.run()?;
    Ok(())
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
    /// Full, unfiltered shares of the open bill (or ledger), in bill order.
    detail_shares: Vec<ShareEntry>,
    /// Visible share IDs, parallel to the `shares` model.
    display_share_ids: Vec<String>,
    payment_ids: Vec<String>,
    history_slots: Vec<(i32, i32)>,
    type_ids: Vec<String>,
    type_archived: Vec<bool>,
    form_type_ids: Vec<String>,
    pick_rows: Vec<Pick>,
    detail_bill: Option<String>,
    detail_boarder: Option<String>,
    detail_total_minor: i64,
    detail_locked: bool,
    detail_archived: bool,
    share_filter: String,
    share_filter_mode: i32,
    /// 0 All, 1 Cash, 2 Online; filters the bill's share list only.
    share_type_filter: i32,
    /// Net money collected by each method on the open bill (whole bill).
    cash_collected_minor: i64,
    online_collected_minor: i64,
    /// Share ID captured when the editor opens; resolved against `detail_shares`.
    edit_share_id: Option<String>,
    ledger_bill_ids: Vec<String>,
    /// Tab (0, 1 or 3) that pushed screens return to.
    last_tab: i32,
    /// The open bill was reached from a boarder ledger, so Back returns there.
    bill_from_ledger: bool,
    /// Calendar picker: which field is being edited and its pending value.
    calendar_target: i32,
    calendar_date: NaiveDate,
    calendar_year: i32,
    calendar_month: i32,
    calendar_month_only: bool,
    calendar_hour: i32,
    calendar_minute: i32,
    calendar_pm: bool,
    /// Payments behind the overpayment dialog and the return-change sheet.
    overpay_payment_id: Option<String>,
    overpay_share_id: Option<String>,
    return_change_payment_id: Option<String>,
    return_change_share_id: Option<String>,
    return_change_date: NaiveDate,
    /// Payment detail sheet and returned-date correction.
    detail_payment_id: Option<String>,
    detail_payment_share_id: Option<String>,
    detail_returned_at: Option<i64>,
    change_correction_payment_id: Option<String>,
    /// Manage shares sheet ids.
    manage_share_ids: Vec<String>,
    manage_candidate_ids: Vec<String>,
    /// Share id parallel to `payment_ids` (bill or ledger payment rows).
    payment_share_ids: Vec<String>,
    /// Payment awaiting confirmation in the centered undo dialog.
    undo_payment_id: Option<String>,
    undo_share_id: Option<String>,
    /// Payment ids parallel to the Edit share sheet's list, newest first.
    share_sheet_payment_ids: Vec<String>,
    /// Updater state shared with the worker thread, the last rendered status,
    /// and the timer that polls it.
    update: Option<crate::update::Shared>,
    update_state: UpdateStatus,
    update_last_checked: Option<std::time::Instant>,
    install_started: Option<std::time::Instant>,
    update_timer: slint::Timer,
}

impl App {
    fn new(ui: slint::Weak<AppWindow>, db: Treasurer) -> Self {
        let (year, month) = money::current_period();
        let today = Local::now().date_naive();
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
            detail_shares: Vec::new(),
            display_share_ids: Vec::new(),
            payment_ids: Vec::new(),
            history_slots: Vec::new(),
            type_ids: Vec::new(),
            type_archived: Vec::new(),
            form_type_ids: Vec::new(),
            pick_rows: Vec::new(),
            detail_bill: None,
            detail_boarder: None,
            detail_total_minor: 0,
            detail_locked: false,
            detail_archived: false,
            share_filter: String::new(),
            share_filter_mode: 0,
            share_type_filter: 0,
            cash_collected_minor: 0,
            online_collected_minor: 0,
            edit_share_id: None,
            ledger_bill_ids: Vec::new(),
            last_tab: 0,
            bill_from_ledger: false,
            calendar_target: 0,
            calendar_date: today,
            calendar_year: year,
            calendar_month: month,
            calendar_month_only: false,
            calendar_hour: 12,
            calendar_minute: 0,
            calendar_pm: false,
            overpay_payment_id: None,
            overpay_share_id: None,
            return_change_payment_id: None,
            return_change_share_id: None,
            return_change_date: today,
            detail_payment_id: None,
            detail_payment_share_id: None,
            detail_returned_at: None,
            change_correction_payment_id: None,
            manage_share_ids: Vec::new(),
            manage_candidate_ids: Vec::new(),
            payment_share_ids: Vec::new(),
            undo_payment_id: None,
            undo_share_id: None,
            share_sheet_payment_ids: Vec::new(),
            update: None,
            update_state: UpdateStatus::Idle,
            update_last_checked: None,
            install_started: None,
            update_timer: slint::Timer::default(),
        }
    }

    fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("window is alive")
    }

    /// Shows an error or validation problem. Errors stay until dismissed.
    fn message(&self, text: impl Into<SharedString>) {
        let ui = self.ui();
        ui.set_message_error(true);
        ui.set_message(text.into());
    }

    /// Screens 10 and 11 share the `shares` / `payments` models. Clear them and
    /// their id lists before loading another detail so a failed load can never
    /// leave rows that point at a different bill's ids.
    fn clear_detail_lists(&mut self) {
        self.detail_shares.clear();
        self.display_share_ids.clear();
        self.payment_ids.clear();
        self.payment_share_ids.clear();
        self.ledger_bill_ids.clear();
        self.detail_total_minor = 0;
        self.detail_locked = false;
        self.detail_archived = false;
        self.share_filter.clear();
        self.share_filter_mode = 0;
        self.share_type_filter = 0;
        self.cash_collected_minor = 0;
        self.online_collected_minor = 0;
        self.edit_share_id = None;
        self.share_sheet_payment_ids.clear();
        let ui = self.ui();
        ui.set_shares(slint::ModelRc::default());
        ui.set_payments(slint::ModelRc::default());
        ui.set_share_preview(slint::ModelRc::default());
        ui.set_share_payments(slint::ModelRc::default());
        ui.set_share_payments_caption("".into());
        ui.set_share_filter("".into());
        ui.set_share_filter_mode(0);
        ui.set_share_type_filter(0);
        ui.set_share_type_collected_caption("".into());
        ui.set_share_type_collected("".into());
        ui.set_share_total_count(0);
        ui.set_share_visible_count(0);
        ui.set_share_edit_error("".into());
        ui.set_share_edit_can_save(false);
    }

    /// Shows a confirmation that an action succeeded.
    fn notify(&self, text: impl Into<SharedString>) {
        let ui = self.ui();
        ui.set_message_error(false);
        ui.set_message(text.into());
    }

    fn start(&mut self) {
        self.load_settings();
        self.refresh_dashboard();
        self.refresh_boarders();
        self.refresh_history();
        self.refresh_settings();
        self.load_form_types();
        let month_names: Vec<SharedString> = money::MONTH_NAMES
            .iter()
            .map(|name| SharedString::from(*name))
            .collect();
        self.ui()
            .set_month_names(ModelRc::new(VecModel::from(month_names)));
        self.ui().set_app_version(version::version_label().into());
        self.ui().set_screen_title("Dashboard".into());
        self.start_updates();
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
        if matches!(screen, 0 | 1 | 3) {
            self.last_tab = screen;
        }
        let previous = self.ui().get_screen();
        let ui = self.ui();
        ui.set_screen(screen);
        ui.set_message("".into());
        ui.set_message_error(false);
        ui.set_screen_title(
            match screen {
                0 => "Dashboard",
                1 => "Boarders",
                3 => "History",
                4 => "Settings",
                10 => "Bill",
                11 => "Boarder",
                12 => "New bill",
                _ => "Treasurer",
            }
            .into(),
        );

        // Leaving Settings cancels an in-flight download (spec §6.2); entering
        // it re-checks the install permission after a trip to system settings.
        if previous == 4 && screen != 4 {
            self.cancel_update_download();
        }
        if screen == 4 {
            self.resume_update_state();
        }

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
        let screen = self.ui().get_screen();
        if screen == 10 && self.bill_from_ledger && self.detail_boarder.is_some() {
            self.detail_bill = None;
            self.bill_from_ledger = false;
            self.set_screen(11);
            self.refresh_ledger();
            return;
        }
        self.detail_bill = None;
        self.detail_boarder = None;
        self.bill_from_ledger = false;
        self.set_screen(self.last_tab);
    }

    fn wire(app: Rc<RefCell<App>>) {
        let ui = app.borrow().ui();

        {
            let app = app.clone();
            ui.on_nav(move |screen| app.borrow_mut().set_screen(screen));
        }
        {
            let app = app.clone();
            ui.on_back(move || app.borrow_mut().back());
        }

        dashboard::wire(&app, &ui);
        calendar::wire(&app, &ui);
        boarders::wire(&app, &ui);
        bills::wire(&app, &ui);
        history::wire(&app, &ui);
        settings::wire(&app, &ui);
        backup::wire(&app, &ui);
        update::wire(&app, &ui);
    }
}

/// 150 ms UI-thread poll of the updater's shared state.
fn start_update_loop(app: Rc<RefCell<App>>) {
    let weak = Rc::downgrade(&app);
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(150),
        move || {
            if let Some(app) = weak.upgrade() {
                app.borrow_mut().poll_updates();
            }
        },
    );
    app.borrow_mut().update_timer = timer;
}

/// Up to two initials for an avatar, e.g. "Maria Santos" -> "MS".
pub(super) fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

/// Share of `expected` that has been collected, clamped to 0..=1.
pub(super) fn progress(applied: i64, expected: i64) -> f32 {
    if expected <= 0 {
        return if applied > 0 { 1.0 } else { 0.0 };
    }
    (applied as f64 / expected as f64).clamp(0.0, 1.0) as f32
}
