use std::path::PathBuf;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static ANDROID_PATHS: OnceLock<(PathBuf, PathBuf, PathBuf)> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_paths(db: PathBuf, backups: PathBuf, updates: PathBuf) {
    let _ = ANDROID_PATHS.set((db, backups, updates));
}

#[cfg(target_os = "android")]
pub fn db_path() -> PathBuf {
    ANDROID_PATHS
        .get()
        .map(|(db, _, _)| db.clone())
        .unwrap_or_else(|| PathBuf::from("/data/local/tmp/treasurer.db"))
}

#[cfg(target_os = "android")]
pub fn backup_dir() -> PathBuf {
    ANDROID_PATHS
        .get()
        .map(|(_, backups, _)| backups.clone())
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(target_os = "android")]
pub fn updates_dir() -> PathBuf {
    ANDROID_PATHS
        .get()
        .map(|(_, _, updates)| updates.clone())
        .unwrap_or_else(|| std::env::temp_dir().join("updates"))
}

#[cfg(not(target_os = "android"))]
pub fn db_path() -> PathBuf {
    desktop_dir().join("treasurer.db")
}

#[cfg(not(target_os = "android"))]
pub fn backup_dir() -> PathBuf {
    desktop_dir().join("backups")
}

#[cfg(not(target_os = "android"))]
pub fn updates_dir() -> PathBuf {
    desktop_dir().join("updates")
}

/// Removes every file in the update folder: downloads never survive a restart,
/// and a stale `.part` can never be resumed or installed.
pub fn cleanup_updates_dir() {
    let dir = updates_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(not(target_os = "android"))]
fn desktop_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("com.boardinghouse.treasurer")
}
