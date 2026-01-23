/// Bot version information from Cargo.toml (read at compile time)
pub const BOT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(dead_code)]
/// Get bot version string for display
pub fn get_bot_version_string() -> String {
    format!("v{}", BOT_VERSION)
}
