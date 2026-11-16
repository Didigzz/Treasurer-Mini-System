//! GitHub Releases feed request. All HTTP lives here; the parsed payload is
//! handed to `treasurer_core::update` for validation.

use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use treasurer_core::update::{
    build_release_info, ReleaseAsset, ReleaseInfo, ReleaseResponse, UpdateFeedError,
};

use super::FailureKind;

const FEED_URL: &str =
    "https://api.github.com/repos/Didigzz/Treasurer-Mini-System/releases/latest";
const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
const USER_AGENT: &str = "boarding-house-treasurer";

pub struct DownloadStream {
    pub reader: Box<dyn Read>,
    pub total: Option<u64>,
}

pub fn fetch_latest() -> Result<ReleaseInfo, FailureKind> {
    let text = fetch_text(&feed_url())?;
    let response = parse_release(&text)?;
    build_release_info(response).map_err(|error| match error {
        UpdateFeedError::MissingVersionCode => FailureKind::MissingVersionCode,
        UpdateFeedError::MissingApk => FailureKind::MissingApk,
        UpdateFeedError::MissingChecksum => FailureKind::MissingChecksum,
    })
}

/// The feed URL, with the desktop-only `TREASURER_UPDATE_FEED` override used
/// to sweep every state against a scratch or `file://` payload.
fn feed_url() -> String {
    #[cfg(not(target_os = "android"))]
    if let Ok(url) = std::env::var("TREASURER_UPDATE_FEED") {
        if !url.trim().is_empty() {
            return url.trim().to_string();
        }
    }
    FEED_URL.to_string()
}

fn parse_release(text: &str) -> Result<ReleaseResponse, FailureKind> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| FailureKind::BadFeed)?;

    let tag_name = value
        .get("tag_name")
        .and_then(|value| value.as_str())
        .ok_or(FailureKind::BadFeed)?
        .to_string();
    let body = value
        .get("body")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let html_url = value
        .get("html_url")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let assets = value
        .get("assets")
        .and_then(|value| value.as_array())
        .ok_or(FailureKind::BadFeed)?
        .iter()
        .filter_map(|asset| {
            Some(ReleaseAsset {
                name: asset.get("name")?.as_str()?.to_string(),
                download_url: asset.get("browser_download_url")?.as_str()?.to_string(),
                size: asset.get("size").and_then(|value| value.as_u64()).unwrap_or(0),
            })
        })
        .collect();

    Ok(ReleaseResponse {
        tag_name,
        body,
        html_url,
        assets,
    })
}

/// Fetches a small text resource (the feed or the `.sha256` sidecar).
pub fn fetch_text(url: &str) -> Result<String, FailureKind> {
    if let Some(path) = local_path(url) {
        return std::fs::read_to_string(path).map_err(|_| FailureKind::Network);
    }
    let response = get(url)?;
    response.into_string().map_err(|_| FailureKind::Network)
}

/// Opens the APK byte stream and reports the server's own length, if any.
pub fn open_download(url: &str) -> Result<DownloadStream, FailureKind> {
    if let Some(path) = local_path(url) {
        let file = std::fs::File::open(path).map_err(|_| FailureKind::Download)?;
        let total = file.metadata().ok().map(|metadata| metadata.len());
        return Ok(DownloadStream {
            reader: Box::new(file),
            total,
        });
    }

    let response = get(url)?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    Ok(DownloadStream {
        reader: Box::new(response.into_reader()),
        total,
    })
}

fn get(url: &str) -> Result<ureq::Response, FailureKind> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(CHECK_TIMEOUT)
        .timeout(CHECK_TIMEOUT)
        .redirects(5)
        .build();

    match agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set(
            "User-Agent",
            &format!("{USER_AGENT}/{}", crate::version::VERSION_NAME),
        )
        .call()
    {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(code, response)) => {
            let rate_limited = (code == 403 || code == 429)
                && response
                    .header("x-ratelimit-remaining")
                    .map(|value| value.trim() == "0")
                    .unwrap_or(false);
            if rate_limited {
                Err(FailureKind::RateLimited)
            } else {
                Err(FailureKind::Network)
            }
        }
        Err(ureq::Error::Transport(_)) => Err(FailureKind::Network),
    }
}

fn local_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let trimmed = if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
        &rest[1..]
    } else {
        rest
    };
    Some(PathBuf::from(trimmed))
}
