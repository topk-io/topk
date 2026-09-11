use anyhow::{Context, Result};
use chrono::Utc;
use oauth2::basic::BasicTokenResponse;
use oauth2::TokenResponse;
use serde::{Deserialize, Serialize};

const REFRESH_EARLY_SECS: u64 = 60;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

impl Session {
    pub fn from_response(
        res: BasicTokenResponse,
        previous_refresh_token: Option<String>,
    ) -> Result<Self> {
        let expires_at = res
            .expires_in()
            .context("token response is missing expires_in")?
            .as_secs()
            .saturating_add(Utc::now().timestamp() as u64);
        Ok(Self {
            access_token: res.access_token().secret().clone(),
            refresh_token: res
                .refresh_token()
                .map(|token| token.secret().clone())
                .or(previous_refresh_token),
            expires_at,
        })
    }

    pub fn needs_refresh(&self) -> bool {
        self.expires_at <= (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
