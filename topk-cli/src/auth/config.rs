use anyhow::{ensure, Context, Result};
use clap::builder::NonEmptyStringValueParser;
use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;

use super::store::CredentialsStore;

const AUTH_DOMAIN: &str = "topk-prod.us.auth0.com";
const AUTH_CLIENT_ID: &str = "2LqddiN2N5fQplfMP2MIYPHM6ttFNeaG";
const AUTH_AUDIENCE: &str = "https://api.topk.io";

#[derive(clap::Args, Clone)]
pub struct Config {
    #[arg(long = "auth-domain", env = "TOPK_AUTH_DOMAIN", default_value = AUTH_DOMAIN, value_parser = parse_domain, hide = true, global = true)]
    pub issuer: Url,
    #[arg(long = "auth-client-id", env = "TOPK_AUTH_CLIENT_ID", default_value = AUTH_CLIENT_ID, value_parser = NonEmptyStringValueParser::new(), hide = true, global = true)]
    pub client_id: String,
    #[arg(
        long = "auth-audience",
        env = "TOPK_AUTH_AUDIENCE",
        default_value = AUTH_AUDIENCE,
        hide = true,
        global = true
    )]
    pub audience: String,
    /// Credentials store for a new session; existing sessions keep their store.
    #[arg(
        long = "credentials-store",
        env = "TOPK_CREDENTIALS_STORE",
        default_value = "auto",
        hide = true,
        global = true
    )]
    pub store: CredentialsStore,
}

/// Resolved issuer, OAuth client, and API audience used for authentication.
#[derive(Clone, Serialize)]
pub(super) struct OAuthConfig {
    pub issuer: Url,
    pub client_id: String,
    pub audience: String,
}

impl OAuthConfig {
    pub fn key(&self) -> Result<String> {
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
}

impl Config {
    pub(super) fn oauth(&self) -> OAuthConfig {
        OAuthConfig {
            issuer: self.issuer.clone(),
            client_id: self.client_id.clone(),
            audience: self.audience.clone(),
        }
    }
}

fn parse_domain(domain: &str) -> Result<Url> {
    let issuer =
        Url::parse(&format!("https://{domain}/")).context("invalid authentication domain")?;
    ensure!(
        issuer.host_str().is_some()
            && issuer.path() == "/"
            && issuer.username().is_empty()
            && issuer.password().is_none()
            && issuer.query().is_none()
            && issuer.fragment().is_none(),
        "authentication domain must be a hostname"
    );
    Ok(issuer)
}
