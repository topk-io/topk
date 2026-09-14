use serde::{Deserialize, Serialize};

use crate::auth::oauth::jwt_payload;

#[derive(Debug, Clone, Serialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    /// Unix seconds.
    pub exp: u64,
    pub name: Option<String>,
    pub email: Option<String>,
    /// Auth0 organization
    pub org_id: Option<String>,
    pub org_display_name: Option<String>,
    pub role: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
struct Raw {
    sub: String,
    exp: u64,
    org_id: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl AccessTokenClaims {
    pub fn parse(token: &str, audience: &str) -> Option<Self> {
        let mut raw: Raw = serde_json::from_value(jwt_payload(token)?).ok()?;
        let mut custom = |name: &str| {
            raw.extra
                .remove(&format!("{audience}/{name}"))
                .and_then(|v| v.as_str().map(str::to_string))
        };
        Some(Self {
            name: custom("name"),
            email: custom("email"),
            role: custom("role"),
            org_display_name: custom("org_display_name"),
            permissions: raw
                .extra
                .remove(&format!("{audience}/permissions"))
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
            sub: raw.sub,
            exp: raw.exp,
            org_id: raw.org_id,
        })
    }

    pub fn username(&self) -> String {
        match (&self.email, &self.name) {
            (Some(email), _) => email.clone(),
            (None, Some(name)) => name.clone(),
            (None, None) => self.sub.clone(),
        }
    }

    pub fn team(&self) -> Option<&str> {
        match &self.org_id {
            Some(_) => self.org_display_name.as_deref(),
            None => None,
        }
    }

    pub fn account(&self) -> String {
        match self.team() {
            Some(org) => format!("{} in {org}", self.username()),
            None => self.username(),
        }
    }
}
