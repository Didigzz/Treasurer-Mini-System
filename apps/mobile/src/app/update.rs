//! Settings "App updates" state, copy, and actions.
//!
//! The worker owns the network and the filesystem; this module only renders the
//! published state and issues explicit actions (check, retry, install).

use std::{cell::RefCell, rc::Rc};

use super::App;
use crate::update::{self, FailureKind, UpdateStatus};
use crate::{version, AppWindow};
use treasurer_core::update::format_size;

const DATA_REASSURANCE: &str =
    "Your records stay on this device and aren't affected by an update.";

impl App {
    pub(super) fn shared_update(&mut self) -> update::Shared {
        self.update
            .get_or_insert_with(update::new_shared)
            .clone()
    }

    /// Called once at startup: installs the version label, clears stale
    /// downloads, renders `Idle`, and starts the silent launch check.
    pub(super) fn start_updates(&mut self) {
        self.ui().set_update_installed(
            format!(
                "{} ({})",
                version::VERSION_NAME,
                version::VERSION_CODE
            )
            .into(),
        );
        crate::platform::cleanup_updates_dir();
        self.render_update_state();
        self.check_for_updates();
    }

    pub(super) fn check_for_updates(&mut self) {
        let shared = self.shared_update();
        update::spawn_check(shared);
        self.poll_updates();
    }

    pub(super) fn retry_update(&mut self) {
        if let UpdateStatus::Failed {
            kind,
            release: Some(release),
            ..
        } = self.update_state.clone()
        {
            if matches!(kind, FailureKind::Download | FailureKind::ChecksumMismatch) {
                let shared = self.shared_update();
                update::spawn_download(shared, release);
                self.poll_updates();
                return;
            }
        }
        self.check_for_updates();
    }

    pub(super) fn cancel_update_download(&mut self) {
        let downloading = matches!(self.update_state, UpdateStatus::Downloading { .. })
            || matches!(
                self.update_state,
                UpdateStatus::Available {
                    cancelled: false,
                    ..
                }
            );
        if let (true, Some(shared)) = (downloading, &self.update) {
            update::request_cancel(shared);
        }
    }

    pub(super) fn install_update(&mut self) {
        let UpdateStatus::Ready { release, path } = self.update_state.clone() else {
            return;
        };

        #[cfg(target_os = "android")]
        {
            if !update::install::can_request_package_installs() {
                self.set_update_state(UpdateStatus::NeedsPermission { release, path });
                return;
            }
            match update::install::install_apk(&path) {
                Ok(()) => {
                    self.install_started = Some(std::time::Instant::now());
                    self.set_update_state(UpdateStatus::Installing { release, path });
                }
                Err(update::install::InstallError::Permission) => {
                    self.set_update_state(UpdateStatus::NeedsPermission { release, path });
                }
                Err(update::install::InstallError::Failed(raw)) => {
                    let _ = std::fs::remove_file(&path);
                    self.set_update_state(UpdateStatus::Failed {
                        kind: FailureKind::InstallFailed,
                        raw: Some(raw),
                        release: Some(release),
                    });
                }
            }
        }

        #[cfg(not(target_os = "android"))]
        let _ = (release, path);
    }

    pub(super) fn open_install_permission(&mut self) {
        #[cfg(target_os = "android")]
        {
            let _ = update::install::open_install_settings();
        }
    }

    /// Re-checks the install permission when Settings is entered again, so a
    /// granted permission turns back into an enabled Install button without a
    /// second download.
    pub(super) fn resume_update_state(&mut self) {
        let UpdateStatus::NeedsPermission { release, path } = self.update_state.clone() else {
            self.render_update_state();
            return;
        };

        #[cfg(target_os = "android")]
        if update::install::can_request_package_installs() {
            self.set_update_state(UpdateStatus::Ready { release, path });
            return;
        }

        #[cfg(not(target_os = "android"))]
        let _ = (release, path);
        self.render_update_state();
    }

    /// UI-thread poll driven by a 150 ms timer. Never blocks on the network.
    pub(super) fn poll_updates(&mut self) {
        let Some(shared) = self.update.clone() else {
            return;
        };
        let (status, last_checked) = {
            let state = shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (state.status.clone(), state.last_checked)
        };
        self.update_last_checked = last_checked;
        if status != self.update_state {
            self.update_state = status;
            self.render_update_state();
        }
        self.settle_install();
    }

    /// After the installer handoff the process usually dies. If it survives,
    /// return to `Ready` and keep the APK so Install can be tapped again.
    fn settle_install(&mut self) {
        let Some(started) = self.install_started else {
            return;
        };
        if started.elapsed().as_secs() < 5 {
            return;
        }
        self.install_started = None;
        let UpdateStatus::Installing { release, path } = self.update_state.clone() else {
            return;
        };
        self.set_update_state(UpdateStatus::Ready { release, path });
    }

    fn set_update_state(&mut self, status: UpdateStatus) {
        if let Some(shared) = &self.update {
            shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .status = status.clone();
        }
        self.update_state = status;
        self.render_update_state();
    }

    pub(super) fn render_update_state(&mut self) {
        let state = self.update_state.clone();
        let status: String;
        let mut summary = String::new();
        let mut detail = String::new();
        let mut progress = 0f32;
        let mut progress_label = String::new();
        let mut downloading = false;
        let mut can_check = false;
        let mut needs_permission = false;
        let mut can_retry = false;
        let mut attention = false;

        match &state {
            UpdateStatus::Idle => {
                status = "Not checked yet.".to_string();
                can_check = true;
            }
            UpdateStatus::Checking => {
                status = "Checking for updates…".to_string();
            }
            UpdateStatus::UpToDate { .. } => {
                status = "You're on the latest version.".to_string();
                detail = self.checked_ago();
                can_check = true;
            }
            UpdateStatus::Ahead {
                latest_name,
                installed,
                latest,
            } => {
                status = "You're on a newer build than the latest release.".to_string();
                detail = format!(
                    "Installed {} ({installed}) · latest release {latest_name} ({latest})",
                    version::VERSION_NAME
                );
                can_check = true;
            }
            UpdateStatus::Available { release, cancelled } => {
                status = format!(
                    "Version {} is available ({}).",
                    release.version_name,
                    format_size(release.apk_size)
                );
                summary = release.summary.clone();
                if *cancelled {
                    detail = "Download cancelled.".to_string();
                    can_check = true;
                } else {
                    detail = "Download starting…".to_string();
                }
            }
            UpdateStatus::Downloading {
                release,
                downloaded,
                total,
            } => {
                status = "Downloading update…".to_string();
                summary = release.summary.clone();
                downloading = true;
                let total = if *total == 0 {
                    release.apk_size
                } else {
                    *total
                };
                progress = if total == 0 {
                    0.0
                } else {
                    (*downloaded as f64 / total as f64).clamp(0.0, 1.0) as f32
                };
                let percent = (progress * 100.0).round() as u32;
                progress_label = format!(
                    "{percent}% · {} of {}",
                    format_size(*downloaded),
                    format_size(total)
                );
                detail = "Leaving Settings cancels the download.".to_string();
            }
            UpdateStatus::Ready { release, .. } => {
                summary = release.summary.clone();
                detail = format!(
                    "Version {} · {}",
                    release.version_name,
                    format_size(release.apk_size)
                );
                attention = true;

                #[cfg(target_os = "android")]
                {
                    status = "Update ready to install.".to_string();
                }
                #[cfg(not(target_os = "android"))]
                {
                    status =
                        "Update downloaded. Installing is only supported on Android.".to_string();
                }
            }
            UpdateStatus::Installing { release, .. } => {
                status = "Opening Android's installer…".to_string();
                summary = release.summary.clone();
                detail = DATA_REASSURANCE.to_string();
            }
            UpdateStatus::NeedsPermission { release, .. } => {
                status = "Allow installs from this app to continue.".to_string();
                summary = release.summary.clone();
                detail = "Android needs permission to install the update. Nothing is downloaded again."
                    .to_string();
                needs_permission = true;
            }
            UpdateStatus::Failed { kind, raw, release } => {
                let (status_copy, detail_copy, retry_allowed, check_allowed) = failure_copy(*kind);
                status = status_copy.to_string();
                detail = raw.clone().unwrap_or_else(|| detail_copy.to_string());
                can_retry = retry_allowed;
                can_check = check_allowed;
                if *kind == FailureKind::Download {
                    if let Some(release) = release {
                        summary = release.summary.clone();
                    }
                }
            }
        }

        #[cfg(target_os = "android")]
        let can_install = matches!(state, UpdateStatus::Ready { .. });
        #[cfg(not(target_os = "android"))]
        let can_install = false;

        let ui = self.ui();
        ui.set_update_status(status.into());
        ui.set_update_summary(summary.into());
        ui.set_update_detail(detail.into());
        ui.set_update_progress(progress);
        ui.set_update_progress_label(progress_label.into());
        ui.set_update_downloading(downloading);
        ui.set_update_can_check(can_check);
        ui.set_update_can_install(can_install);
        ui.set_update_needs_permission(needs_permission);
        ui.set_update_can_retry(can_retry);
        ui.set_update_attention(attention);
    }

    fn checked_ago(&self) -> String {
        let Some(checked) = self.update_last_checked else {
            return String::new();
        };
        let seconds = checked.elapsed().as_secs();
        if seconds < 60 {
            "Checked just now".to_string()
        } else if seconds < 3600 {
            format!("Checked {} minutes ago", seconds / 60)
        } else {
            format!("Checked {} hours ago", seconds / 3600)
        }
    }
}

/// Status line, detail, whether Retry applies, and whether Check applies.
fn failure_copy(kind: FailureKind) -> (&'static str, &'static str, bool, bool) {
    match kind {
        FailureKind::Network => (
            "Couldn't check for updates.",
            "Check your connection and try again.",
            true,
            true,
        ),
        FailureKind::RateLimited => (
            "Couldn't check for updates.",
            "GitHub is limiting requests right now. Try again later.",
            true,
            true,
        ),
        FailureKind::BadFeed => (
            "Couldn't read the update information.",
            "The latest release is not in the expected format.",
            true,
            true,
        ),
        FailureKind::MissingVersionCode => (
            "Couldn't read the update information.",
            "The latest release is missing its version code.",
            true,
            true,
        ),
        FailureKind::MissingApk => (
            "This release can't be installed.",
            "The latest release has no APK file.",
            true,
            true,
        ),
        FailureKind::MissingChecksum => (
            "This release can't be verified.",
            "The latest release has no checksum file, so it won't be installed.",
            true,
            true,
        ),
        FailureKind::ChecksumMismatch => (
            "The download didn't match its checksum.",
            "The file was deleted. Try the download again.",
            true,
            true,
        ),
        FailureKind::Download => (
            "Couldn't download the update.",
            "Check your connection and try again.",
            true,
            true,
        ),
        FailureKind::Write => (
            "Couldn't save the update.",
            "The phone may be low on storage.",
            true,
            true,
        ),
        FailureKind::InstallBlocked => (
            "Allow installs from this app to continue.",
            "Android needs permission to install the update. Nothing is downloaded again.",
            false,
            false,
        ),
        FailureKind::InstallSignatureMismatch => (
            "Android couldn't install this update.",
            "It's signed with a different key than the installed app. Export a backup in Backup & restore, uninstall the app, then install the update again.",
            false,
            true,
        ),
        FailureKind::InstallFailed => (
            "Android couldn't install this update.",
            "Android couldn't install the update.",
            true,
            false,
        ),
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_check_updates(move || app.borrow_mut().check_for_updates());
    }
    {
        let app = app.clone();
        ui.on_retry_update(move || app.borrow_mut().retry_update());
    }
    {
        let app = app.clone();
        ui.on_install_update(move || app.borrow_mut().install_update());
    }
    {
        let app = app.clone();
        ui.on_open_install_settings(move || app.borrow_mut().open_install_permission());
    }
}
