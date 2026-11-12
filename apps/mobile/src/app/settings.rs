use std::{cell::RefCell, rc::Rc};

use slint::{ModelRc, SharedString, VecModel};

use treasurer_core::money::{self, CURRENCY_OPTIONS};

use super::App;
use crate::{AppWindow, TypeRow};

impl App {
    pub(super) fn refresh_settings(&mut self) {
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
        self.type_ids = types.iter().map(|row| row.bill_type.id.clone()).collect();
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

    pub(super) fn add_bill_type(&mut self, name: SharedString) {
        match self.db.create_bill_type(&name) {
            Ok(_) => {
                self.ui().set_type_name("".into());
                self.refresh_settings();
                self.load_form_types();
                self.notify("Bill type added.");
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn archive_type(&mut self, index: usize) {
        let Some(id) = self.type_ids.get(index).cloned() else {
            return;
        };
        let archived = self.type_archived.get(index).copied().unwrap_or(false);
        match self.db.set_bill_type_archived(&id, !archived) {
            Ok(()) => {
                self.refresh_settings();
                self.load_form_types();
                self.refresh_dashboard();
                self.notify(if archived {
                    "Bill type restored."
                } else {
                    "Bill type archived."
                });
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }

    pub(super) fn save_settings(&mut self) {
        let ui = self.ui();
        let index = ui.get_currency_index().max(0) as usize;
        let Some((code, _)) = CURRENCY_OPTIONS.get(index) else {
            self.refresh_settings();
            return self.message("Choose a currency.");
        };
        let allow = ui.get_allow_centavos();

        let saved = self
            .db
            .set_currency_code(code)
            .and_then(|()| self.db.set_allow_centavos(allow));
        if let Err(error) = saved {
            // Re-read what is actually stored so the controls (and the
            // in-memory currency) never show an unsaved value.
            self.refresh_settings();
            self.refresh_dashboard();
            return self.message(format!("{error:#}"));
        }

        self.currency = (*code).to_string();
        self.allow_centavos = allow;
        self.refresh_dashboard();
        self.refresh_settings();
        self.notify("Settings saved.");
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_add_bill_type(move |name| app.borrow_mut().add_bill_type(name));
    }
    {
        let app = app.clone();
        ui.on_archive_type(move |index| app.borrow_mut().archive_type(index.max(0) as usize));
    }
    {
        let app = app.clone();
        ui.on_save_settings(move || app.borrow_mut().save_settings());
    }
}

pub(super) fn format_iso_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| money::format_date(date.timestamp_millis()))
        .unwrap_or_else(|_| value.to_string())
}
