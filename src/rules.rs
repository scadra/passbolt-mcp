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

//! The MCP rules: what the agent may do with what the MCP user sees. A rule only takes away:
//! Passbolt still refuses what the permission of the MCP user does not allow.

use std::fmt;

use crate::config::AccessConfig;

/// What a tool needs from `[access]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The metadata: always allowed.
    Read,
    Secrets,
    Write,
    Share,
}

/// Why the rules refuse, naming the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `access.<key>` is false.
    Access(&'static str),
    /// `access.deny_tools` lists the tool.
    Denied(String),
    /// `access.allow_tools` does not list the tool.
    NotAllowed(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Access(key) => write!(f, "refused by the MCP rules: access.{key} is false"),
            Self::Denied(tool) => {
                write!(
                    f,
                    "refused by the MCP rules: access.deny_tools lists {tool}"
                )
            }
            Self::NotAllowed(tool) => write!(
                f,
                "refused by the MCP rules: access.allow_tools does not list {tool}"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

#[derive(Debug, Default)]
pub struct Rules {
    access: AccessConfig,
}

impl Rules {
    #[must_use]
    pub const fn new(access: AccessConfig) -> Self {
        Self { access }
    }

    /// Whether the agent gets `tool`, which needs `needs`.
    pub fn offers(&self, tool: &str, needs: Access) -> Result<(), Refusal> {
        if self.access.deny_tools.iter().any(|denied| denied == tool) {
            return Err(Refusal::Denied(tool.into()));
        }
        if !self.access.allow_tools.is_empty()
            && !self
                .access
                .allow_tools
                .iter()
                .any(|allowed| allowed == tool)
        {
            return Err(Refusal::NotAllowed(tool.into()));
        }
        let (allowed, key) = match needs {
            Access::Read => return Ok(()),
            Access::Secrets => (self.access.secrets, "secrets"),
            Access::Write => (self.access.write, "write"),
            Access::Share => (self.access.share, "share"),
        };
        if allowed {
            Ok(())
        } else {
            Err(Refusal::Access(key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_by_default() {
        let rules = Rules::default();
        assert_eq!(rules.offers("list_resources", Access::Read), Ok(()));
        assert_eq!(
            rules.offers("get_secret", Access::Secrets),
            Err(Refusal::Access("secrets"))
        );
        assert_eq!(
            rules.offers("create_resource", Access::Write),
            Err(Refusal::Access("write"))
        );
        assert_eq!(
            rules.offers("share_resource", Access::Share),
            Err(Refusal::Access("share"))
        );
    }

    #[test]
    fn allowed() {
        let rules = Rules::new(AccessConfig {
            secrets: true,
            write: true,
            ..AccessConfig::default()
        });
        assert_eq!(rules.offers("get_secret", Access::Secrets), Ok(()));
        assert_eq!(rules.offers("create_resource", Access::Write), Ok(()));
        assert!(rules.offers("share_resource", Access::Share).is_err());
    }

    #[test]
    fn allow_tools() {
        let rules = Rules::new(AccessConfig {
            allow_tools: vec!["list_resources".into(), "get_secret".into()],
            ..AccessConfig::default()
        });
        assert_eq!(rules.offers("list_resources", Access::Read), Ok(()));
        assert_eq!(
            rules.offers("list_folders", Access::Read),
            Err(Refusal::NotAllowed("list_folders".into()))
        );
        // Listed, but access.secrets is still false.
        assert_eq!(
            rules.offers("get_secret", Access::Secrets),
            Err(Refusal::Access("secrets"))
        );
    }

    #[test]
    fn deny_tools() {
        let rules = Rules::new(AccessConfig {
            write: true,
            allow_tools: vec!["delete_resource".into()],
            deny_tools: vec!["delete_resource".into()],
            ..AccessConfig::default()
        });
        assert_eq!(
            rules.offers("delete_resource", Access::Write),
            Err(Refusal::Denied("delete_resource".into()))
        );
    }

    #[test]
    fn names_the_rule() {
        assert_eq!(
            Refusal::Access("write").to_string(),
            "refused by the MCP rules: access.write is false"
        );
    }
}
