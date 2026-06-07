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
    pub access: AccessConfig,
    /// The directory of the file, or the default one: where the server key is pinned.
    #[serde(skip)]
    pub dir: PathBuf,
}

/// `[passbolt]`: the MCP user and its server.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PassboltConfig {
    pub server_url: String,
    pub private_key_file: PathBuf,
    pub passphrase_file: Option<PathBuf>,
    pub passphrase_command: Option<Vec<String>>,
    pub totp_secret_file: Option<PathBuf>,
    /// The server key pinned by `passbolt-mcp verify` [default: server-key.asc next to the file]
    pub server_key_file: Option<PathBuf>,
    /// Accept any certificate: for a test server only.
    pub tls_skip_verify: bool,
    /// `host:port:address`, like `curl --resolve`.
    pub resolve: Vec<String>,
}

/// `passbolt-mcp.toml` in the configuration directory of the system.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "passbolt-mcp")
        .map(|dirs| dirs.config_dir().join("passbolt-mcp.toml"))
}

/// `[access]`: what the agent may do. Reading metadata is always allowed; the rest is off.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AccessConfig {
    pub write: bool,
    pub secrets: bool,
    pub share: bool,
    /// When not empty, only these tools.
    pub allow_tools: Vec<String>,
    /// Never these tools.
    pub deny_tools: Vec<String>,
}

impl Config {
    /// `path` when given (`--config`, `PASSBOLT_MCP_CONFIG`), else the default file if any.
    pub fn find(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            return Self::load(path);
        }
        match default_path() {
            Some(path) if path.exists() => Self::load(&path),
            Some(path) => Ok(Self {
                dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
                ..Self::default()
            }),
            None => Ok(Self::default()),
        }
    }

    #[must_use]
    pub fn server_key_path(&self) -> PathBuf {
        self.passbolt
            .server_key_file
            .clone()
            .unwrap_or_else(|| self.dir.join("server-key.asc"))
    }

    pub fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let config = Self::parse(&text).with_context(|| format!("invalid {}", path.display()))?;
        Ok(config.relative_to(path.parent().unwrap_or_else(|| Path::new(""))))
    }

    /// Relative paths are relative to the directory of the file.
    fn relative_to(mut self, dir: &Path) -> Self {
        self.dir = dir.to_path_buf();
        let passbolt = &mut self.passbolt;
        for path in [
            Some(&mut passbolt.private_key_file),
            passbolt.passphrase_file.as_mut(),
            passbolt.totp_secret_file.as_mut(),
            passbolt.server_key_file.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            if path.is_relative() && !path.as_os_str().is_empty() {
                *path = dir.join(&*path);
            }
        }
        self
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
    fn relative_paths() {
        let config = Config::parse(
            "[passbolt]\nprivate_key_file = \"agent.asc\"\npassphrase_file = \"/run/secret\"",
        )
        .unwrap()
        .relative_to(Path::new("/etc/passbolt-mcp"));
        assert_eq!(
            config.passbolt.private_key_file,
            Path::new("/etc/passbolt-mcp/agent.asc")
        );
        assert_eq!(
            config.passbolt.passphrase_file.as_deref().unwrap(),
            Path::new("/run/secret")
        );
        assert!(config.passbolt.totp_secret_file.is_none());
        assert_eq!(
            config.server_key_path(),
            Path::new("/etc/passbolt-mcp/server-key.asc")
        );
    }

    #[test]
    fn access() {
        let access = Config::parse("[access]\nsecrets = true\ndeny_tools = [\"get_secret\"]")
            .unwrap()
            .access;
        assert!(access.secrets);
        assert!(!access.write);
        assert_eq!(access.deny_tools, ["get_secret"]);
    }

    #[test]
    fn empty() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.passbolt.server_url, "");
        assert!(!config.access.write && !config.access.secrets && !config.access.share);
        assert_eq!(config.access.allow_tools.len(), 0);
    }

    #[test]
    fn unknown_key() {
        assert!(Config::parse("[passbolt]\npassphrase = \"secret\"").is_err());
    }
}
