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

use std::collections::BTreeSet;

use passbolt_sdk::permission::Grant;
use passbolt_sdk::types::Group;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ToolError;

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(description = "`share_resource` and `simulate_share`.")]
pub struct ShareResource {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(description = "Ids of users, from `search_aros`.")]
    #[serde(default)]
    pub users: Vec<String>,
    #[schemars(description = "Ids of groups, from `search_aros`.")]
    #[serde(default)]
    pub groups: Vec<String>,
    #[schemars(description = "`read`, `update`, `owner`, or `none` to revoke their access.")]
    pub permission: String,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(description = "`share_folder`.")]
pub struct ShareFolder {
    #[schemars(description = "The folder, by path or id.")]
    pub folder: String,
    #[schemars(description = "Ids of users, from `search_aros`.")]
    #[serde(default)]
    pub users: Vec<String>,
    #[schemars(description = "Ids of groups, from `search_aros`.")]
    #[serde(default)]
    pub groups: Vec<String>,
    #[schemars(description = "`read`, `update`, `owner`, or `none` to revoke their access.")]
    pub permission: String,
    #[schemars(
        description = "Also on the subfolders and resources the MCP user owns, as the browser extension does."
    )]
    #[serde(default = "yes")]
    pub with_content: bool,
    #[schemars(description = "Say what it would do, and do nothing.")]
    #[serde(default)]
    pub dry_run: bool,
}

const fn yes() -> bool {
    true
}

pub fn permission_type(permission: &str) -> Result<i32, ToolError> {
    match permission {
        "read" => Ok(1),
        "update" => Ok(7),
        "owner" => Ok(15),
        "none" => Ok(-1),
        _ => Err(ToolError::invalid_argument(format!(
            "permission: read, update, owner or none, not {permission}"
        ))),
    }
}

pub fn describe(users: usize, groups: usize, permission: &str) -> Result<String, ToolError> {
    let plural =
        |count: usize, what: &str| format!("{count} {what}{}", if count == 1 { "" } else { "s" });
    let whom = match (users, groups) {
        (0, 0) => return Err(ToolError::invalid_argument("give users or groups".into())),
        (users, 0) => plural(users, "user"),
        (0, groups) => plural(groups, "group"),
        (users, groups) => format!("{} and {}", plural(users, "user"), plural(groups, "group")),
    };
    Ok(if permission == "none" {
        format!("revoke the access of {whom} to")
    } else {
        format!("give {permission} to {whom} on")
    })
}

#[must_use]
pub fn target(
    mut grants: Vec<Grant>,
    users: &[String],
    groups: &[String],
    permission_type: i32,
) -> Vec<Grant> {
    let aros = users
        .iter()
        .map(|id| ("User", id))
        .chain(groups.iter().map(|id| ("Group", id)));
    for (aro, id) in aros {
        grants.retain(|grant| !(grant.aro == aro && &grant.aro_foreign_key == id));
        if permission_type != -1 {
            grants.push(Grant {
                aro: aro.into(),
                aro_foreign_key: id.clone(),
                permission_type,
            });
        }
    }
    grants
}

fn users_of(grants: &[Grant], groups: &[Group]) -> BTreeSet<String> {
    let mut users = BTreeSet::new();
    for grant in grants {
        if grant.aro == "Group" {
            let members = groups
                .iter()
                .filter(|group| group.id == grant.aro_foreign_key)
                .flat_map(|group| &group.groups_users)
                .filter_map(|member| member.user_id.clone());
            users.extend(members);
        } else {
            users.insert(grant.aro_foreign_key.clone());
        }
    }
    users
}

#[must_use]
pub fn gaining(before: &[Grant], after: &[Grant], groups: &[Group]) -> Vec<String> {
    let before = users_of(before, groups);
    users_of(after, groups)
        .into_iter()
        .filter(|user| !before.contains(user))
        .collect()
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "`simulate_share`.")]
pub struct Simulation {
    #[schemars(description = "What the share would do.")]
    pub action: String,
    #[schemars(
        description = "The users who would get access, group members included: `First Last <username>`."
    )]
    pub gaining_access: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions() {
        assert_eq!(permission_type("read").unwrap(), 1);
        assert_eq!(permission_type("owner").unwrap(), 15);
        assert_eq!(permission_type("none").unwrap(), -1);
        assert!(permission_type("admin").is_err());
    }

    #[test]
    fn descriptions() {
        assert_eq!(
            describe(1, 2, "read").unwrap(),
            "give read to 1 user and 2 groups on"
        );
        assert_eq!(
            describe(2, 0, "none").unwrap(),
            "revoke the access of 2 users to"
        );
        assert!(describe(0, 0, "read").is_err());
    }

    fn grant(aro: &str, id: &str, permission_type: i32) -> Grant {
        Grant {
            aro: aro.into(),
            aro_foreign_key: id.into(),
            permission_type,
        }
    }

    #[test]
    fn targets() {
        let current = vec![grant("User", "me", 15), grant("Group", "ops", 1)];
        assert_eq!(
            target(current.clone(), &["ada".into()], &["ops".into()], 7),
            [
                grant("User", "me", 15),
                grant("User", "ada", 7),
                grant("Group", "ops", 7)
            ]
        );
        assert_eq!(
            target(current, &[], &["ops".into()], -1),
            [grant("User", "me", 15)]
        );
    }

    #[test]
    fn gaining_access() {
        let ops: Group = serde_json::from_value(serde_json::json!({
            "id": "ops", "name": "Ops",
            "groups_users": [{ "user_id": "ada" }, { "user_id": "bob" }]
        }))
        .unwrap();
        let before = [grant("User", "me", 15), grant("User", "ada", 1)];
        let after = [
            grant("User", "me", 15),
            grant("User", "ada", 1),
            grant("Group", "ops", 1),
            grant("User", "cid", 7),
        ];
        assert_eq!(gaining(&before, &after, &[ops]), ["bob", "cid"]);
        assert_eq!(gaining(&after, &before, &[]), Vec::<String>::new());
    }

    #[test]
    fn with_content_by_default() {
        let args: ShareFolder =
            serde_json::from_value(serde_json::json!({ "folder": "A", "permission": "read" }))
                .unwrap();
        assert!(args.with_content);
    }
}
