//! Loading of the user configuration file.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// User configuration loaded from `~/.config/rutt/config.toml`.
///
/// ```toml
/// [user]
/// email = "you@example.com"
/// password = "secret"
///
/// [imap]
/// host = "imap.example.com"
/// port = 993          # optional, default 993
/// mailbox = "INBOX"   # optional, default INBOX
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    pub user: User,
    pub imap: Imap,
}

/// `[user]` section: login credentials.
#[derive(Debug, Deserialize)]
pub struct User {
    pub email: String,
    pub password: String,
}

/// `[imap]` section: server connection settings.
#[derive(Debug, Deserialize)]
pub struct Imap {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_mailbox")]
    pub mailbox: String,
}

fn default_port() -> u16 {
    993
}

fn default_mailbox() -> String {
    "INBOX".to_string()
}

/// Path of the config file: `$XDG_CONFIG_HOME/rutt/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().context("could not determine config directory")?;
    Ok(dir.join("rutt").join("config.toml"))
}

/// Read and parse the config file.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    let config: Config = toml::from_str(&text)
        .with_context(|| format!("failed to parse config file {}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_defaults() {
        let c: Config = toml::from_str(
            r#"
[user]
email = "a@b"
password = "p"

[imap]
host = "imap.example.com"
"#,
        )
        .unwrap();
        assert_eq!(c.user.email, "a@b");
        assert_eq!(c.imap.host, "imap.example.com");
        assert_eq!(c.imap.port, 993);
        assert_eq!(c.imap.mailbox, "INBOX");
    }
}
