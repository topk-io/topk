use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};

pub const DEFAULT_HOST: &str = "topk.io";

#[derive(clap::Args, Clone, Debug)]
pub struct HostArgs {
    /// API domain
    #[arg(
        long,
        env = "TOPK_HOST",
        value_parser = parse_host,
        global = true,
        hide = true
    )]
    pub host: Option<String>,

    /// Connect over HTTPS (default: true; --https false for a plaintext endpoint)
    #[arg(
        long,
        env = "TOPK_HTTPS",
        num_args = 0..=1,
        default_missing_value = "true",
        global = true,
        hide = true
    )]
    pub https: Option<bool>,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Host {
    pub host: String,
    pub https: bool,
}

impl From<HostArgs> for Host {
    fn from(args: HostArgs) -> Self {
        Self {
            host: args.host.unwrap_or_else(|| DEFAULT_HOST.to_owned()),
            https: args.https.unwrap_or(true),
        }
    }
}

impl Host {
    pub fn key(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn api_url(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };
        format!("{protocol}://api.{}", self.host)
    }
}

/// `name[:port]`. With a port, every service is reached on it; without one, on its own default.
fn parse_host(host: &str) -> Result<String, String> {
    if let Some((_, port)) = host.rsplit_once(':') {
        port.parse::<u16>()
            .map_err(|e| format!("invalid port {port:?}: {e}"))?;
    }
    Ok(host.to_string())
}
