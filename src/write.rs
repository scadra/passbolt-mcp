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

//! The arguments and results of the write tools, with `access.write` only. Every write takes
//! `dry_run`: it then says what it would do, and does nothing.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a write did, or would do with `dry_run`.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct WriteResult {
    /// Nothing was written.
    pub dry_run: bool,
    /// What was done, or would be.
    pub action: String,
    /// The id of what was created or changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl WriteResult {
    #[must_use]
    pub fn new(dry_run: bool, action: String, id: Option<String>) -> Self {
        let action = if dry_run {
            format!("would {action}")
        } else {
            action
        };
        Self {
            dry_run,
            action,
            id,
        }
    }
}

/// `create_resource`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct CreateResource {
    pub name: String,
    pub username: Option<String>,
    #[serde(default)]
    pub uris: Vec<String>,
    pub description: Option<String>,
    /// From `generate_password`, for instance.
    pub password: Option<String>,
    /// The secret note of a v5 resource.
    pub note: Option<String>,
    /// The folder to create it in, by path or id; the root when absent.
    pub folder: Option<String>,
    /// Its type, by slug (`list_resource_types`); by default, from the fields given.
    pub resource_type: Option<String>,
    /// Expiry of the password, RFC 3339.
    pub expiry: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

/// Where a folder argument points: its path for the messages, its id for Passbolt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub id: Option<String>,
    pub name: String,
}

impl Place {
    #[must_use]
    pub fn root() -> Self {
        Self {
            id: None,
            name: "the root".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_says_would() {
        let result = WriteResult::new(true, "create GitHub in Agents/CI".into(), None);
        assert_eq!(result.action, "would create GitHub in Agents/CI");
        assert_eq!(
            WriteResult::new(false, "create GitHub".into(), Some("1".into())).action,
            "create GitHub"
        );
    }
}
