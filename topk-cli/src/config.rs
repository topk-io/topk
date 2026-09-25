use std::path::PathBuf;

/// The CLI configuration directory.
pub fn dir() -> Option<PathBuf> {
    std::env::var_os("TOPK_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|d| d.join("topk")))
}
