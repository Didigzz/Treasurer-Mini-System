//! Pure helpers for the in-app updater.
//!
//! Tag parsing, asset selection, checksum handling, and display formatting live
//! here so they are covered by core tests. Nothing in this module touches the
//! network, the filesystem, or the database.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version_name: String,
    pub version_code: i64,
    pub tag_name: String,
    pub apk_name: String,
    pub apk_url: String,
    pub apk_size: u64,
    pub sha256_url: String,
    pub summary: String,
    pub html_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseResponse {
    pub tag_name: String,
    pub body: Option<String>,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFeedError {
    MissingVersionCode,
    MissingApk,
    MissingChecksum,
}

/// Parses `v1.4.0+5` / `1.4.0+5`. Returns None for anything else.
pub fn parse_release_tag(tag: &str) -> Option<(String, i64)> {
    let tag = tag.trim();
    let tag = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);
    let (name, code) = tag.split_once('+')?;
    let name = name.trim();
    let code = code.trim();

    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    if !parts
        .iter()
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }

    let code: i64 = code.parse().ok()?;
    if code <= 0 {
        return None;
    }
    Some((name.to_string(), code))
}

/// True only when the release code is strictly greater than the installed code.
pub fn is_newer(release_code: i64, installed_code: i32) -> bool {
    release_code > installed_code as i64
}

/// First asset whose name ends with ".apk" (case-insensitive), in API order.
pub fn pick_apk_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    assets
        .iter()
        .find(|asset| asset.name.to_ascii_lowercase().ends_with(".apk"))
}

/// The sidecar name for an APK asset: "<apk name>.sha256".
pub fn checksum_asset_name(apk_name: &str) -> String {
    format!("{apk_name}.sha256")
}

/// First 64-hex token, lowercased.
pub fn parse_sha256(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(|token| token.to_ascii_lowercase())
}

/// First non-empty line, whitespace-collapsed, truncated to `max` chars with '…'.
pub fn release_summary(body: &str, max: usize) -> String {
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return String::new();
    };
    let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        return collapsed;
    }
    let mut summary: String = collapsed.chars().take(max).collect();
    summary.push('…');
    summary
}

/// "29.1 MB" / "812 KB" / "512 B" style, 1024-based units.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.1} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Case-insensitive hex comparison of the computed digest with the published one.
pub fn checksum_matches(computed_hex: &str, expected_hex: &str) -> bool {
    let computed = computed_hex.trim();
    let expected = expected_hex.trim();
    computed.len() == 64
        && expected.len() == 64
        && computed
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && expected
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && computed.eq_ignore_ascii_case(expected)
}

/// Turns a full GitHub release payload into ReleaseInfo, or a typed error.
pub fn build_release_info(response: ReleaseResponse) -> Result<ReleaseInfo, UpdateFeedError> {
    let (version_name, version_code) =
        parse_release_tag(&response.tag_name).ok_or(UpdateFeedError::MissingVersionCode)?;

    let apk = pick_apk_asset(&response.assets)
        .ok_or(UpdateFeedError::MissingApk)?
        .clone();

    let sidecar = checksum_asset_name(&apk.name);
    let checksum = response
        .assets
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(&sidecar))
        .ok_or(UpdateFeedError::MissingChecksum)?;

    Ok(ReleaseInfo {
        version_name,
        version_code,
        tag_name: response.tag_name,
        apk_name: apk.name,
        apk_url: apk.download_url,
        apk_size: apk.size,
        sha256_url: checksum.download_url.clone(),
        summary: release_summary(response.body.as_deref().unwrap_or(""), 160),
        html_url: response.html_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, size: u64) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            download_url: format!("https://example.test/{name}"),
            size,
        }
    }

    fn response(tag: &str, assets: Vec<ReleaseAsset>) -> ReleaseResponse {
        ReleaseResponse {
            tag_name: tag.to_string(),
            body: Some("First line\nSecond line".to_string()),
            html_url: "https://example.test/release".to_string(),
            assets,
        }
    }

    #[test]
    fn parses_release_tags() {
        assert_eq!(
            parse_release_tag("v1.4.0+5"),
            Some(("1.4.0".to_string(), 5))
        );
        assert_eq!(
            parse_release_tag("1.4.0+5"),
            Some(("1.4.0".to_string(), 5))
        );
        assert_eq!(
            parse_release_tag("  v1.4.0+5  "),
            Some(("1.4.0".to_string(), 5))
        );
        assert_eq!(parse_release_tag("v1.4+5"), None);
        assert_eq!(parse_release_tag("v1.4.0"), None);
        assert_eq!(parse_release_tag("v1.4.0+abc"), None);
        assert_eq!(parse_release_tag("v1.4.0+0"), None);
        assert_eq!(parse_release_tag("v1.4.0.1+5"), None);
        assert_eq!(parse_release_tag(""), None);
    }

    #[test]
    fn compares_version_codes() {
        assert!(is_newer(6, 5));
        assert!(!is_newer(5, 5));
        assert!(!is_newer(4, 5));
    }

    #[test]
    fn picks_the_first_apk_asset() {
        let single = vec![asset("app-1.4.0.apk", 10)];
        assert_eq!(pick_apk_asset(&single).unwrap().name, "app-1.4.0.apk");

        let later = vec![asset("notes.txt", 1), asset("app.apk", 10)];
        assert_eq!(pick_apk_asset(&later).unwrap().name, "app.apk");

        let uppercase = vec![asset("APP.APK", 10)];
        assert_eq!(pick_apk_asset(&uppercase).unwrap().name, "APP.APK");

        let none = vec![asset("notes.txt", 1)];
        assert!(pick_apk_asset(&none).is_none());

        let two = vec![asset("first.apk", 10), asset("second.apk", 20)];
        assert_eq!(pick_apk_asset(&two).unwrap().name, "first.apk");
    }

    #[test]
    fn names_the_checksum_sidecar() {
        assert_eq!(checksum_asset_name("app.apk"), "app.apk.sha256");
    }

    #[test]
    fn parses_checksum_files() {
        let hex = "a".repeat(64);
        assert_eq!(parse_sha256(&hex), Some(hex.clone()));
        assert_eq!(
            parse_sha256(&format!("{hex}  boarding-house-treasurer-1.4.0.apk")),
            Some(hex.clone())
        );
        let upper = "A".repeat(64);
        assert_eq!(parse_sha256(&upper), Some(hex.clone()));
        assert_eq!(parse_sha256(&"a".repeat(63)), None);
        assert_eq!(parse_sha256(&"a".repeat(65)), None);
        assert_eq!(parse_sha256("no checksum here"), None);
        assert_eq!(parse_sha256(""), None);
    }

    #[test]
    fn compares_checksums_case_insensitively() {
        let lower = "a".repeat(64);
        let upper = "A".repeat(64);
        assert!(checksum_matches(&lower, &upper));
        assert!(!checksum_matches(&lower, &"b".repeat(64)));
        assert!(!checksum_matches(&"a".repeat(63), &lower));
        assert!(!checksum_matches(&"a".repeat(65), &lower));
        assert!(!checksum_matches("", ""));
    }

    #[test]
    fn summarises_the_release_body() {
        assert_eq!(release_summary("Hello\nWorld", 160), "Hello");
        assert_eq!(
            release_summary("\n\n   Fixed   the   split  \nbody", 160),
            "Fixed the split"
        );
        assert_eq!(release_summary("", 160), "");
        assert_eq!(release_summary("   \n \n", 160), "");

        let long = "x".repeat(200);
        let summary = release_summary(&long, 160);
        assert_eq!(summary.chars().count(), 161);
        assert!(summary.ends_with('…'));

        let multibyte = "ñ".repeat(200);
        let summary = release_summary(&multibyte, 160);
        assert_eq!(summary.chars().count(), 161);
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(812 * 1024), "812 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(29 * 1024 * 1024 + 100 * 1024), "29.1 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn builds_release_info_from_a_full_payload() {
        let info = build_release_info(response(
            "v1.4.1+6",
            vec![asset("treasurer-1.4.1.apk", 30500000), asset("treasurer-1.4.1.apk.sha256", 64)],
        ))
        .unwrap();

        assert_eq!(info.version_name, "1.4.1");
        assert_eq!(info.version_code, 6);
        assert_eq!(info.tag_name, "v1.4.1+6");
        assert_eq!(info.apk_name, "treasurer-1.4.1.apk");
        assert_eq!(info.apk_size, 30500000);
        assert_eq!(
            info.sha256_url,
            "https://example.test/treasurer-1.4.1.apk.sha256"
        );
        assert_eq!(info.summary, "First line");
        assert_eq!(info.html_url, "https://example.test/release");
    }

    #[test]
    fn rejects_incomplete_payloads() {
        assert_eq!(
            build_release_info(response(
                "v1.4.0",
                vec![asset("app.apk", 1), asset("app.apk.sha256", 64)]
            )),
            Err(UpdateFeedError::MissingVersionCode)
        );
        assert_eq!(
            build_release_info(response(
                "v1.4.0+5",
                vec![asset("notes.txt", 1), asset("notes.txt.sha256", 64)]
            )),
            Err(UpdateFeedError::MissingApk)
        );
        assert_eq!(
            build_release_info(response("v1.4.0+5", vec![asset("app.apk", 1)])),
            Err(UpdateFeedError::MissingChecksum)
        );
    }
}
