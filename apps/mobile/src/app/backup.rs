use std::{cell::RefCell, rc::Rc};

use chrono::{Local, SecondsFormat};

use super::App;
use crate::AppWindow;

impl App {
    pub(super) fn export_backup(&mut self) {
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
        self.refresh_settings();
        self.notify(format!("Backup saved to {}", path.display()));
    }

    pub(super) fn import_backup(&mut self) {
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
            if newest
                .as_ref()
                .map(|(time, _)| modified > *time)
                .unwrap_or(true)
            {
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
                self.load_settings();
                self.refresh_dashboard();
                self.refresh_boarders();
                self.refresh_history();
                self.refresh_settings();
                self.load_form_types();
                self.notify(format!(
                    "Restored from {}.",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("backup")
                ));
            }
            Err(error) => self.message(format!("{error:#}")),
        }
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_export_backup(move || app.borrow_mut().export_backup());
    }
    {
        let app = app.clone();
        ui.on_import_backup(move || app.borrow_mut().import_backup());
    }
}
