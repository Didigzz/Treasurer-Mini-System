//! Updater state shared between the UI thread and one worker thread.
//!
//! `App` is an `Rc<RefCell<..>>`, so the worker never touches it. It publishes
//! plain data through `UpdateShared`; a `slint::Timer` on the UI thread reads it
//! and renders.

pub mod feed;
mod download;

#[cfg(target_os = "android")]
pub mod install;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use treasurer_core::update::ReleaseInfo;

use crate::version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Network,
    RateLimited,
    BadFeed,
    MissingVersionCode,
    MissingApk,
    MissingChecksum,
    ChecksumMismatch,
    Download,
    Write,
    InstallBlocked,
    InstallSignatureMismatch,
    InstallFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        latest: String,
        code: i64,
    },
    Ahead {
        latest_name: String,
        installed: i64,
        latest: i64,
    },
    Available {
        release: ReleaseInfo,
        cancelled: bool,
    },
    Downloading {
        release: ReleaseInfo,
        downloaded: u64,
        total: u64,
    },
    Ready {
        release: ReleaseInfo,
        path: PathBuf,
    },
    Installing {
        release: ReleaseInfo,
        path: PathBuf,
    },
    NeedsPermission {
        release: ReleaseInfo,
        path: PathBuf,
    },
    Failed {
        kind: FailureKind,
        /// Android's verbatim status name, or a specific download note.
        raw: Option<String>,
        /// The release Retry should download again, when it is still known.
        release: Option<ReleaseInfo>,
    },
}

pub struct UpdateShared {
    pub status: UpdateStatus,
    pub last_checked: Option<Instant>,
    pub cancel: Arc<AtomicBool>,
    pub running: bool,
}

pub type Shared = Arc<Mutex<UpdateShared>>;

pub fn new_shared() -> Shared {
    Arc::new(Mutex::new(UpdateShared {
        status: UpdateStatus::Idle,
        last_checked: None,
        cancel: Arc::new(AtomicBool::new(false)),
        running: false,
    }))
}

fn lock(shared: &Shared) -> MutexGuard<'_, UpdateShared> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn set_status(shared: &Shared, status: UpdateStatus) {
    lock(shared).status = status;
}

pub(super) fn finish(shared: &Shared, status: UpdateStatus) {
    let mut state = lock(shared);
    state.status = status;
    state.running = false;
}

/// Starts a feed check. Returns false when a check or download is already
/// running, so the Check button can never start concurrent work.
pub fn spawn_check(shared: Shared) -> bool {
    {
        let mut state = lock(&shared);
        if state.running {
            return false;
        }
        state.running = true;
        state.cancel.store(false, Ordering::SeqCst);
        state.status = UpdateStatus::Checking;
        state.last_checked = None;
    }

    std::thread::spawn(move || {
        let outcome = feed::fetch_latest();
        match outcome {
            Err(kind) => finish(
                &shared,
                UpdateStatus::Failed {
                    kind,
                    raw: None,
                    release: None,
                },
            ),
            Ok(release) => {
                let installed = version::VERSION_CODE as i64;
                if release.version_code == installed {
                    finish(
                        &shared,
                        UpdateStatus::UpToDate {
                            latest: release.version_name,
                            code: installed,
                        },
                    );
                } else if release.version_code < installed {
                    finish(
                        &shared,
                        UpdateStatus::Ahead {
                            latest_name: release.version_name,
                            installed,
                            latest: release.version_code,
                        },
                    );
                } else {
                    set_status(
                        &shared,
                        UpdateStatus::Available {
                            release: release.clone(),
                            cancelled: false,
                        },
                    );
                    download::run(&shared, release);
                }
            }
        }
        lock(&shared).last_checked = Some(Instant::now());
    });

    true
}

/// Starts (or retries) the APK download for a release the check already found.
pub fn spawn_download(shared: Shared, release: ReleaseInfo) -> bool {
    {
        let mut state = lock(&shared);
        if state.running {
            return false;
        }
        state.running = true;
        state.cancel.store(false, Ordering::SeqCst);
        state.status = UpdateStatus::Downloading {
            release: release.clone(),
            downloaded: 0,
            total: release.apk_size,
        };
    }

    std::thread::spawn(move || download::run(&shared, release));
    true
}

/// Asks the running download to stop. The worker deletes the partial file and
/// returns the state to `Available { cancelled: true }`.
pub fn request_cancel(shared: &Shared) {
    lock(shared).cancel.store(true, Ordering::SeqCst);
}
