use std::path::PathBuf;

/// The CLI configuration directory.
pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("topk"))
}
