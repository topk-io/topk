use clap::builder::NonEmptyStringValueParser;
use url::Url;

use crate::auth::oauth::OAuthConfig;

const AUTH_ISSUER: &str = "https://topk-prod.us.auth0.com/";
const AUTH_CLIENT_ID: &str = "2LqddiN2N5fQplfMP2MIYPHM6ttFNeaG";
const AUTH_AUDIENCE: &str = "https://api.topk.io";

#[derive(clap::Args, Clone)]
pub struct Config {
    #[arg(
        long = "auth-issuer",
        env = "TOPK_AUTH_ISSUER",
        default_value = AUTH_ISSUER,
        hide = true,
        global = true
    )]
    pub issuer: Url,
    #[arg(
        long = "auth-client-id",
        env = "TOPK_AUTH_CLIENT_ID",
        default_value = AUTH_CLIENT_ID,
        value_parser = NonEmptyStringValueParser::new(),
        hide = true,
        global = true
    )]
    pub client_id: String,
    #[arg(
        long = "auth-audience",
        env = "TOPK_AUTH_AUDIENCE",
        default_value = AUTH_AUDIENCE,
        hide = true,
        global = true
    )]
    pub audience: String,
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
