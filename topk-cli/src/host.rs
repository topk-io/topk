#[derive(clap::Args, Clone, Debug)]
pub struct Host {
    /// API domain
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
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
