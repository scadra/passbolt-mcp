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

/// `update_resource`: the fields given change, the others stay.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct UpdateResource {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub uris: Option<Vec<String>>,
    pub description: Option<String>,
    /// The new password: encrypted again for everyone with access.
    pub password: Option<String>,
    pub note: Option<String>,
    /// Expiry of the password, RFC 3339.
    pub expiry: Option<String>,
    /// Remove the expiry.
    #[serde(default)]
    pub clear_expiry: bool,
    #[serde(default)]
    pub dry_run: bool,
}

impl UpdateResource {
    /// The names of the fields that change, never their values.
    #[must_use]
    pub fn changed(&self) -> Vec<&'static str> {
        [
            ("name", self.name.is_some()),
            ("username", self.username.is_some()),
            ("uris", self.uris.is_some()),
            ("description", self.description.is_some()),
            ("password", self.password.is_some()),
            ("note", self.note.is_some()),
            ("expiry", self.expiry.is_some() || self.clear_expiry),
        ]
        .into_iter()
        .filter_map(|(field, changed)| changed.then_some(field))
        .collect()
    }
}

/// `delete_resource`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteResource {
    pub id: String,
    #[serde(default)]
    pub dry_run: bool,
}

/// `move_resource`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveResource {
    pub id: String,
    /// The folder to move it to, by path or id; the root when absent.
    pub folder: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

/// `create_folder`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateFolder {
    pub name: String,
    /// The folder to create it in, by path or id; the root when absent.
    pub parent: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

/// `rename_folder`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameFolder {
    /// The folder, by path or id.
    pub folder: String,
    pub name: String,
    #[serde(default)]
    pub dry_run: bool,
}

/// `move_folder`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveFolder {
    /// The folder, by path or id.
    pub folder: String,
    /// The folder to move it into, by path or id; the root when absent.
    pub parent: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

/// `delete_folder`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteFolder {
    /// The folder, by path or id.
    pub folder: String,
    /// Also delete its subfolders and the resources the MCP user may delete in them; otherwise
    /// they move up to the parent.
    #[serde(default)]
    pub with_content: bool,
    #[serde(default)]
    pub dry_run: bool,
}

/// `set_favorite`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetFavorite {
    /// The id of the resource.
    pub id: String,
    /// `true` to mark it, `false` to unmark it.
    pub favorite: bool,
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
    fn changed_fields() {
        let update = UpdateResource {
            password: Some("s3cret".into()),
            clear_expiry: true,
            ..UpdateResource::default()
        };
        assert_eq!(update.changed(), ["password", "expiry"]);
    }

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
