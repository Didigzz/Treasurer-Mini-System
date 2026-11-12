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
        self.ui()
            .set_app_version("Boarding House Treasurer 1.1.0 · Cargo/Slint".into());
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
