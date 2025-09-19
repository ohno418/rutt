//! Configuration loading and management for Gmail IMAP settings.
//!
//! Handles loading TOML configuration files containing Gmail credentials and
//! connection parameters.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Top-level configuration structure containing all settings.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Gmail-specific configuration settings.
    pub gmail: GmailConfig,
}

/// Gmail IMAP connection configuration.
#[derive(Debug, Deserialize)]
pub struct GmailConfig {
    /// Gmail username (email address).
    pub username: String,
    /// Gmail app password for IMAP access.
    pub app_password: String,
}

impl Config {
    /// Loads configuration from a TOML file at the specified path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path.as_ref()))?;

        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Loads configuration from the default "config.toml" file.
    pub fn load_default() -> Result<Self> {
        Self::load("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[gmail]
username = "test@gmail.com"
app_password = "test-password-123"
"#
        )
        .unwrap();

        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.gmail.username, "test@gmail.com");
        assert_eq!(config.gmail.app_password, "test-password-123");
    }

    #[test]
    fn test_load_missing_file() {
        let result = Config::load("/nonexistent/path/config.toml");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read config")
        );
    }

    #[test]
    fn test_load_invalid_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid toml content {{}}").unwrap();

        let result = Config::load(temp_file.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse config")
        );
    }

    #[test]
    fn test_load_missing_fields() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[gmail]
username = "test@gmail.com"
"#
        )
        .unwrap();

        let result = Config::load(temp_file.path());
        assert!(result.is_err());
    }
}
