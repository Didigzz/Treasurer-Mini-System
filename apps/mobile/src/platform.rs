use std::path::PathBuf;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static ANDROID_PATHS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_paths(db: PathBuf, backups: PathBuf) {
    let _ = ANDROID_PATHS.set((db, backups));
}

#[cfg(target_os = "android")]
pub fn db_path() -> PathBuf {
    ANDROID_PATHS
        .get()
        .map(|(db, _)| db.clone())
        .unwrap_or_else(|| PathBuf::from("/data/local/tmp/treasurer.db"))
}

#[cfg(target_os = "android")]
pub fn backup_dir() -> PathBuf {
    ANDROID_PATHS
        .get()
        .map(|(_, backups)| backups.clone())
        .unwrap_or_else(std::env::temp_dir)
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
fn desktop_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("com.boardinghouse.treasurer")
}
