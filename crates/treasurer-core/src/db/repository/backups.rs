
use anyhow::Result;

use crate::backup::{self, BackupPayload};

use super::Treasurer;

impl Treasurer {
    pub fn create_backup(&self) -> Result<BackupPayload> {
        backup::create_backup_payload(&self.conn)
    }

    pub fn export_backup_json(&self) -> Result<String> {
        backup::export_backup_json(&self.conn)
    }

    pub fn import_backup_json(&self, json: &str) -> Result<()> {
        let payload = backup::parse_backup_json(json)?;
        backup::restore_backup(&self.conn, payload)
    }
}
