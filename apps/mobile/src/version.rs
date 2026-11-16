//! Compiled-in app version. One source of truth for the updater and the
//! Settings footer; the Android manifest must agree (see the test below).

pub const VERSION_NAME: &str = "1.4.1";
pub const VERSION_CODE: i32 = 6;
pub const APP_LABEL: &str = "Boarding House Treasurer";

/// "Boarding House Treasurer 1.4.1"
pub fn version_label() -> String {
    format!("{APP_LABEL} {VERSION_NAME}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_value(key: &str) -> Option<String> {
        include_str!("../manifest.yaml").lines().find_map(|line| {
            let line = line.trim();
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(':'))
                .map(|value| value.trim().to_string())
        })
    }

    #[test]
    fn manifest_matches_the_compiled_version() {
        assert_eq!(manifest_value("version_name").as_deref(), Some(VERSION_NAME));
        assert_eq!(
            manifest_value("version_code").as_deref(),
            Some(VERSION_CODE.to_string().as_str())
        );
    }
}
