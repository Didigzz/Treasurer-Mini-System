
use anyhow::{bail, Result};
use rusqlite::params;

use crate::backup::{self};
use crate::models::*;
use crate::money::is_currency_code;

use super::now_ms;
use super::Treasurer;

impl Treasurer {
    pub fn app_settings(&self) -> Result<AppSettings> {
        let currency = self
            .setting("currency_code")?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "PHP".to_string());
        let last_backup_at = self
            .setting("last_backup_at")?
            .filter(|value| !value.is_empty());
        let allow_centavos = self.setting("allow_centavos")?.as_deref() != Some("false");

        Ok(AppSettings {
            currency_code: currency,
            allow_centavos,
            last_backup_at,
        })
    }

    pub(super) fn setting(&self, key: &str) -> Result<Option<String>> {
        backup::get_setting(&self.conn, key)
    }

    pub(super) fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now_ms()],
        )?;
        Ok(())
    }

    pub fn set_currency_code(&self, currency_code: &str) -> Result<()> {
        if !is_currency_code(currency_code) {
            bail!("Choose a supported currency.");
        }
        self.set_setting("currency_code", currency_code)
    }

    pub fn set_allow_centavos(&self, allow: bool) -> Result<()> {
        self.set_setting("allow_centavos", if allow { "true" } else { "false" })
    }

    pub fn set_last_backup_at(&self, value: &str) -> Result<()> {
        self.set_setting("last_backup_at", value)
    }
}
