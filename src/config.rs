// Passbolt ~ Open source password manager for teams
// Copyright (c) Passbolt SA (https://www.passbolt.com)
//
// Licensed under GNU Affero General Public License version 3 of the or any later version.
// For full copyright and license information, please see the LICENSE
// Redistributions of files must retain the above copyright notice.
//
// @copyright     Copyright (c) Passbolt SA (https://www.passbolt.com)
// @license       https://opensource.org/licenses/AGPL-3.0 AGPL License
// @link          https://www.passbolt.com Passbolt(tm)
// @since         0.1.0

//! `passbolt-mcp.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub passbolt: PassboltConfig,
}

/// `[passbolt]`: the MCP user and its server.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PassboltConfig {
    pub server_url: String,
    pub private_key_file: PathBuf,
    pub passphrase_file: Option<PathBuf>,
    pub passphrase_command: Option<Vec<String>>,
    pub totp_secret_file: Option<PathBuf>,
}

/// `passbolt-mcp.toml` in the configuration directory of the system.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "passbolt-mcp")
        .map(|dirs| dirs.config_dir().join("passbolt-mcp.toml"))
}

impl Config {
    /// `path` when given (`--config`, `PASSBOLT_MCP_CONFIG`), else the default file if any.
    pub fn find(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            return Self::load(path);
        }
        match default_path() {
            Some(path) if path.exists() => Self::load(&path),
            _ => Ok(Self::default()),
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("invalid {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passbolt_section() {
        let config = Config::parse(
            r#"
            [passbolt]
            server_url = "https://passbolt.example.com"
            private_key_file = "agent.asc"
            passphrase_command = ["pass", "show", "agent"]
            "#,
        )
        .unwrap();
        assert_eq!(config.passbolt.server_url, "https://passbolt.example.com");
        assert_eq!(config.passbolt.private_key_file, Path::new("agent.asc"));
        assert_eq!(
            config.passbolt.passphrase_command.unwrap(),
            ["pass", "show", "agent"]
        );
        assert!(config.passbolt.passphrase_file.is_none());
    }

    #[test]
    fn empty() {
        assert_eq!(Config::parse("").unwrap().passbolt.server_url, "");
    }

    #[test]
    fn unknown_key() {
        assert!(Config::parse("[passbolt]\npassphrase = \"secret\"").is_err());
    }
}
