use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::client::TokenResponse;

const REFRESH_MARGIN_SECS: u64 = 60;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

impl Session {
    pub fn from_response(res: TokenResponse, previous_refresh_token: Option<String>) -> Self {
        Self {
            access_token: res.access_token,
            refresh_token: res.refresh_token.or(previous_refresh_token),
            expires_at: (Utc::now().timestamp() as u64).saturating_add(res.expires_in),
        }
    }

    pub fn needs_refresh(&self) -> bool {
        self.expires_at <= (Utc::now().timestamp() as u64).saturating_add(REFRESH_MARGIN_SECS)
    }
}
