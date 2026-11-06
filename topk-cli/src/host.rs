#[derive(clap::Args, Clone, Debug)]
pub struct Host {
    /// API domain
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        value_parser = parse_host,
        global = true,
        hide = true
    )]
    pub host: String,

    /// Connect over HTTPS (default: true; --https false for a plaintext endpoint)
    #[arg(
        long,
        env = "TOPK_HTTPS",
        default_value = "true",
        num_args = 0..=1,
        default_missing_value = "true",
        global = true,
        hide = true
    )]
    pub https: bool,
}

/// `name[:port]`. With a port, every service is reached on it; without one, on its own default.
fn parse_host(host: &str) -> Result<String, String> {
    if let Some((_, port)) = host.rsplit_once(':') {
        port.parse::<u16>()
            .map_err(|e| format!("invalid port {port:?}: {e}"))?;
    }
    Ok(host.to_string())
}
