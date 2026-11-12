mod backup;
mod bills;
mod boarders;
mod dashboard;
mod history;
mod settings;

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};

use slint::{ComponentHandle, SharedString};

use treasurer_core::db::repository::Treasurer;
use treasurer_core::models::*;
use treasurer_core::money;

use self::bills::Pick;

use crate::AppWindow;

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
    /// Tab (0, 1 or 3) that pushed screens return to.
    last_tab: i32,
    /// The open bill was reached from a boarder ledger, so Back returns there.
    bill_from_ledger: bool,
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
            last_tab: 0,
            bill_from_ledger: false,
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
        self.share_ids.clear();
        self.payment_ids.clear();
        self.ledger_bill_ids.clear();
        let ui = self.ui();
        ui.set_shares(slint::ModelRc::default());
        ui.set_payments(slint::ModelRc::default());
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
        self.ui()
            .set_app_version("Boarding House Treasurer 1.1.0".into());
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
        if matches!(screen, 0 | 1 | 3) {
            self.last_tab = screen;
        }
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
        boarders::wire(&app, &ui);
        bills::wire(&app, &ui);
        history::wire(&app, &ui);
        settings::wire(&app, &ui);
        backup::wire(&app, &ui);
    }
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
