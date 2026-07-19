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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "What a write did, or would do with `dry_run`.")]
pub struct WriteResult {
    #[schemars(description = "Nothing was written. \
        Say what it would do, and do nothing.")]
    pub dry_run: bool,
    #[schemars(description = "What was done, or would be.")]
    pub action: String,
    #[schemars(description = "The id of what was created or changed.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl WriteResult {
    #[must_use]
    pub fn new(dry_run: bool, action: String, id: Option<String>) -> Self {
        let action = crate::untrusted::visible(&if dry_run {
            format!("would {action}")
        } else {
            action
        });
        Self {
            dry_run,
            action,
            id,
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(description = "`create_resource`.")]
pub struct CreateResource {
    #[schemars(description = "The name, as Passbolt shows it.")]
    pub name: String,
    #[schemars(description = "The username, or login.")]
    pub username: Option<String>,
    #[schemars(description = "The URIs; the first is the main one.")]
    #[serde(default)]
    pub uris: Vec<String>,
    #[schemars(description = "The description, in the metadata: not secret.")]
    pub description: Option<String>,
    #[schemars(description = "From `generate_password`, for instance.")]
    pub password: Option<String>,
    #[schemars(description = "The secret note of a v5 resource.")]
    pub note: Option<String>,
    #[schemars(description = "The folder to create it in, by path or id; the root when absent.")]
    pub folder: Option<String>,
    #[schemars(
        description = "Its type, by slug (`list_resource_types`); by default, from the fields given."
    )]
    pub resource_type: Option<String>,
    #[schemars(description = "Expiry of the password, RFC 3339.")]
    pub expiry: Option<String>,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(description = "`update_resource`: the fields given change, the others stay.")]
pub struct UpdateResource {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "The new name.")]
    pub name: Option<String>,
    #[schemars(description = "The new username.")]
    pub username: Option<String>,
    #[schemars(description = "The new URIs, all of them.")]
    pub uris: Option<Vec<String>>,
    #[schemars(description = "The new description.")]
    pub description: Option<String>,
    #[schemars(description = "The new password: encrypted again for everyone with access.")]
    pub password: Option<String>,
    #[schemars(description = "The new secret note of a v5 resource.")]
    pub note: Option<String>,
    #[schemars(description = "Expiry of the password, RFC 3339.")]
    pub expiry: Option<String>,
    #[schemars(description = "Remove the expiry.")]
    #[serde(default)]
    pub clear_expiry: bool,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

impl UpdateResource {
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`delete_resource`.")]
pub struct DeleteResource {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`move_resource`.")]
pub struct MoveResource {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "The folder to move it to, by path or id; the root when absent.")]
    pub folder: Option<String>,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`create_folder`.")]
pub struct CreateFolder {
    #[schemars(description = "The name of the folder.")]
    pub name: String,
    #[schemars(description = "The folder to create it in, by path or id; the root when absent.")]
    pub parent: Option<String>,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`rename_folder`.")]
pub struct RenameFolder {
    #[schemars(description = "The folder, by path or id.")]
    pub folder: String,
    #[schemars(description = "The new name.")]
    pub name: String,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`move_folder`.")]
pub struct MoveFolder {
    #[schemars(description = "The folder, by path or id.")]
    pub folder: String,
    #[schemars(description = "The folder to move it into, by path or id; the root when absent.")]
    pub parent: Option<String>,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`delete_folder`.")]
pub struct DeleteFolder {
    #[schemars(description = "The folder, by path or id.")]
    pub folder: String,
    #[schemars(
        description = "Also delete its subfolders and the resources the MCP user may delete in them; otherwise \
        they move up to the parent."
    )]
    #[serde(default)]
    pub with_content: bool,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`set_favorite`.")]
pub struct SetFavorite {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "`true` to mark it, `false` to unmark it.")]
    pub favorite: bool,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`add_comment`.")]
pub struct AddComment {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "The text of the comment.")]
    pub content: String,
    #[schemars(description = "The comment it answers.")]
    pub parent_id: Option<String>,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`delete_comment`: a comment of the MCP user.")]
pub struct DeleteComment {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "The id of the comment, from `list_comments`.")]
    pub comment_id: String,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

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
