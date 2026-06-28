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

use anyhow::Result;
use passbolt_sdk::FolderQuery;

use crate::config::{AccessConfig, Config};
use crate::scope::Scope;
use crate::session::Session;

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
    /// Outside `scope.folders`.
    OutOfScope,
    /// Over `limits.<key>`.
    RateLimited(&'static str),
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
            Self::OutOfScope => write!(f, "refused by the MCP rules: outside scope.folders"),
            Self::RateLimited(key) => write!(
                f,
                "refused by the MCP rules: over limits.{key}, try again in a minute"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Passbolt permission types.
const UPDATE: i32 = 7;
const OWNER: i32 = 15;

/// What the agent can do with the resources the MCP user sees: Passbolt and the rules together.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rights {
    /// In the scope: names, usernames, URIs, descriptions.
    pub read: usize,
    pub secrets: usize,
    /// Shared with the MCP user as update or owner, with `access.write`.
    pub change: usize,
    /// Shared with the MCP user as owner, with `access.share`.
    pub share: usize,
}

#[derive(Debug, Default)]
pub struct Rules {
    access: AccessConfig,
    scope: Scope,
}

impl Rules {
    #[must_use]
    pub const fn new(access: AccessConfig, scope: Scope) -> Self {
        Self { access, scope }
    }

    /// The rules of `config`, the scope resolved among the folders the MCP user sees.
    pub async fn load(config: &Config, session: &Session) -> Result<Self> {
        let scope = if config.scope.folders.is_empty() {
            Scope::default()
        } else {
            let folders = session
                .call(|client| async move { client.get_folders(FolderQuery::default()).await })
                .await?;
            Scope::resolve(&config.scope.folders, &folders)?
        };
        Ok(Self::new(config.access.clone(), scope))
    }

    #[must_use]
    pub const fn access(&self) -> &AccessConfig {
        &self.access
    }

    #[must_use]
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The rights of the agent on resources, given by their folder and the permission type of
    /// the MCP user.
    pub fn rights<'a>(
        &self,
        resources: impl IntoIterator<Item = (Option<&'a str>, i32)>,
    ) -> Rights {
        let mut rights = Rights::default();
        for (folder, permission) in resources {
            if !self.scope.contains(folder) {
                continue;
            }
            rights.read += 1;
            rights.secrets += usize::from(self.access.secrets);
            rights.change += usize::from(self.access.write && permission >= UPDATE);
            rights.share += usize::from(self.access.share && permission >= OWNER);
        }
        rights
    }

    /// Whether the agent may act on a resource or folder in `folder_id` (`None`: the root).
    pub fn reaches(&self, folder_id: Option<&str>) -> Result<(), Refusal> {
        if self.scope.contains(folder_id) {
            Ok(())
        } else {
            Err(Refusal::OutOfScope)
        }
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
        let rules = Rules::new(
            AccessConfig {
                secrets: true,
                write: true,
                ..AccessConfig::default()
            },
            Scope::default(),
        );
        assert_eq!(rules.offers("get_secret", Access::Secrets), Ok(()));
        assert_eq!(rules.offers("create_resource", Access::Write), Ok(()));
        assert!(rules.offers("share_resource", Access::Share).is_err());
    }

    #[test]
    fn allow_tools() {
        let rules = Rules::new(
            AccessConfig {
                allow_tools: vec!["list_resources".into(), "get_secret".into()],
                ..AccessConfig::default()
            },
            Scope::default(),
        );
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
        let rules = Rules::new(
            AccessConfig {
                write: true,
                allow_tools: vec!["delete_resource".into()],
                deny_tools: vec!["delete_resource".into()],
                ..AccessConfig::default()
            },
            Scope::default(),
        );
        assert_eq!(
            rules.offers("delete_resource", Access::Write),
            Err(Refusal::Denied("delete_resource".into()))
        );
    }

    #[test]
    fn scope() {
        assert_eq!(Rules::default().reaches(None), Ok(()));
        let folder: passbolt_sdk::types::Folder =
            serde_json::from_value(serde_json::json!({ "id": "ci", "name": "CI" })).unwrap();
        let rules = Rules::new(
            AccessConfig::default(),
            Scope::resolve(&["CI".into()], &[folder]).unwrap(),
        );
        assert_eq!(rules.reaches(Some("ci")), Ok(()));
        assert_eq!(rules.reaches(Some("ops")), Err(Refusal::OutOfScope));
        assert_eq!(rules.reaches(None), Err(Refusal::OutOfScope));
    }

    #[test]
    fn rights() {
        let resources = [(None, 1), (None, 7), (None, 15), (None, 15)];
        assert_eq!(
            Rules::default().rights(resources),
            Rights {
                read: 4,
                ..Rights::default()
            }
        );
        let rules = Rules::new(
            AccessConfig {
                secrets: true,
                write: true,
                share: true,
                ..AccessConfig::default()
            },
            Scope::default(),
        );
        assert_eq!(
            rules.rights(resources),
            Rights {
                read: 4,
                secrets: 4,
                change: 3,
                share: 2
            }
        );
    }

    #[test]
    fn rights_in_the_scope() {
        let folder: passbolt_sdk::types::Folder =
            serde_json::from_value(serde_json::json!({ "id": "ci", "name": "CI" })).unwrap();
        let rules = Rules::new(
            AccessConfig {
                write: true,
                ..AccessConfig::default()
            },
            Scope::resolve(&["CI".into()], &[folder]).unwrap(),
        );
        assert_eq!(
            rules.rights([(Some("ci"), 7), (Some("ops"), 15), (None, 15)]),
            Rights {
                read: 1,
                change: 1,
                ..Rights::default()
            }
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
