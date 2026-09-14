use chrono::Utc;
use serde::{Deserialize, Serialize};

const REFRESH_EARLY_SECS: u64 = 60;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

impl Session {
    pub fn needs_refresh(&self) -> bool {
        self.expires_at <= (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
