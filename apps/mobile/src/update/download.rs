//! APK download with progress, SHA-256 verification, and cancellation.
//!
//! Runs on a worker thread and publishes state through `UpdateShared`; it never
//! touches the UI or the database.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use sha2::{Digest, Sha256};

use super::{feed, finish, lock, set_status, FailureKind, ReleaseInfo, Shared, UpdateStatus};
use treasurer_core::update::{checksum_matches, parse_sha256};

/// Guards against a wrong asset being uploaded as a release.
const MAX_APK_BYTES: u64 = 150 * 1024 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;

enum DownloadError {
    Cancelled,
    Failed(FailureKind, Option<String>),
}

fn failed(kind: FailureKind) -> DownloadError {
    DownloadError::Failed(kind, None)
}

pub fn run(shared: &Shared, release: ReleaseInfo) {
    match download(shared, &release) {
        Ok(path) => finish(shared, UpdateStatus::Ready { release, path }),
        Err(DownloadError::Cancelled) => finish(
            shared,
            UpdateStatus::Available {
                release,
                cancelled: true,
            },
        ),
        Err(DownloadError::Failed(kind, raw)) => finish(
            shared,
            UpdateStatus::Failed {
                kind,
                raw,
                release: Some(release),
            },
        ),
    }
}

fn download(shared: &Shared, release: &ReleaseInfo) -> Result<PathBuf, DownloadError> {
    let cancel = lock(shared).cancel.clone();

    if release.apk_size > MAX_APK_BYTES {
        return Err(DownloadError::Failed(
            FailureKind::Download,
            Some("That update is larger than expected.".to_string()),
        ));
    }

    let expected = match feed::fetch_text(&release.sha256_url) {
        Ok(text) => {
            parse_sha256(&text).ok_or(DownloadError::Failed(FailureKind::MissingChecksum, None))?
        }
        Err(_) => return Err(failed(FailureKind::Download)),
    };

    let mut stream = feed::open_download(&release.apk_url).map_err(|_| failed(FailureKind::Download))?;
    let total = stream
        .total
        .filter(|value| *value > 0)
        .unwrap_or(release.apk_size);
    if total > MAX_APK_BYTES {
        return Err(DownloadError::Failed(
            FailureKind::Download,
            Some("That update is larger than expected.".to_string()),
        ));
    }

    let dir = crate::platform::updates_dir();
    std::fs::create_dir_all(&dir).map_err(|_| failed(FailureKind::Write))?;
    remove_other_apks(&dir, release.version_code);

    let target = dir.join(format!(
        "{}-{}",
        release.version_code,
        file_name_of(&release.apk_name)
    ));
    let part = part_path(&target);

    let mut file = File::create(&part).map_err(|_| failed(FailureKind::Write))?;
    let mut buffer = vec![0u8; CHUNK_BYTES];
    let mut downloaded: u64 = 0;

    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err(DownloadError::Cancelled);
        }
        let read = match stream.reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(_) => {
                drop(file);
                let _ = std::fs::remove_file(&part);
                return Err(failed(FailureKind::Download));
            }
        };
        if file.write_all(&buffer[..read]).is_err() {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err(failed(FailureKind::Write));
        }
        downloaded += read as u64;
        set_status(
            shared,
            UpdateStatus::Downloading {
                release: release.clone(),
                downloaded,
                total,
            },
        );
    }

    let _ = file.sync_all();
    if downloaded == 0 {
        let _ = std::fs::remove_file(&part);
        return Err(failed(FailureKind::Download));
    }

    let computed = match sha256_file(&part) {
        Ok(computed) => computed,
        Err(_) => {
            let _ = std::fs::remove_file(&part);
            return Err(failed(FailureKind::Write));
        }
    };
    if !checksum_matches(&computed, &expected) {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&target);
        return Err(failed(FailureKind::ChecksumMismatch));
    }

    if cancel.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&part);
        return Err(DownloadError::Cancelled);
    }

    if let Err(error) = std::fs::rename(&part, &target) {
        let _ = std::fs::remove_file(&part);
        return Err(DownloadError::Failed(FailureKind::Write, Some(error.to_string())));
    }

    Ok(target)
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// One APK at most: everything else in the folder is removed first.
fn remove_other_apks(dir: &Path, keep_code: i64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let is_this_release = name.starts_with(&format!("{keep_code}-"));
        if !is_this_release {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn part_path(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("update.apk")
        .to_string();
    name.push_str(".part");
    target.with_file_name(name)
}

fn file_name_of(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("update.apk")
        .to_string()
}
