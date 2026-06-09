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

//! `scope.folders`: the folders the agent may act in, with their subfolders.

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use passbolt_sdk::collect_subtree_ids;
use passbolt_sdk::types::Folder;

#[derive(Debug, Default)]
pub struct Scope {
    /// `None`: everything the MCP user sees.
    folders: Option<HashSet<String>>,
}

impl Scope {
    /// Resolve `scope.folders`, paths (`Agents/CI`) or ids, among the folders the MCP user sees.
    pub fn resolve(entries: &[String], folders: &[Folder]) -> Result<Self> {
        if entries.is_empty() {
            return Ok(Self::default());
        }
        let by_id: HashMap<&str, &Folder> = folders
            .iter()
            .map(|folder| (folder.id.as_str(), folder))
            .collect();
        let mut ids = HashSet::new();
        for entry in entries {
            let root = find(entry, folders, &by_id)?;
            ids.extend(collect_subtree_ids(&root.id, folders));
        }
        Ok(Self { folders: Some(ids) })
    }

    /// Whether a resource or folder in `folder_id` (`None`: at the root) is in the scope.
    #[must_use]
    pub fn contains(&self, folder_id: Option<&str>) -> bool {
        match (&self.folders, folder_id) {
            (None, _) => true,
            (Some(ids), Some(id)) => ids.contains(id),
            (Some(_), None) => false,
        }
    }

    /// The folder ids of the scope, `None` for everything.
    #[must_use]
    pub const fn folders(&self) -> Option<&HashSet<String>> {
        self.folders.as_ref()
    }
}

fn find<'a>(
    entry: &str,
    folders: &'a [Folder],
    by_id: &HashMap<&str, &'a Folder>,
) -> Result<&'a Folder> {
    if let Some(folder) = by_id.get(entry) {
        return Ok(folder);
    }
    let wanted = entry.trim_matches('/');
    let mut found = folders
        .iter()
        .filter(|folder| path(folder, by_id) == wanted);
    match (found.next(), found.next()) {
        (Some(folder), None) => Ok(folder),
        (Some(_), Some(_)) => bail!("scope.folders: several folders are {entry}: give its id"),
        (None, _) => bail!("scope.folders: {entry} is not a folder the MCP user sees"),
    }
}

/// `Parent/Child`, from the highest parent the MCP user sees.
fn path(folder: &Folder, by_id: &HashMap<&str, &Folder>) -> String {
    let mut names = vec![folder.name.as_str()];
    let mut current = folder;
    while let Some(parent) = current
        .folder_parent_id
        .as_deref()
        .and_then(|id| by_id.get(id))
    {
        if names.len() > by_id.len() {
            break;
        }
        names.push(&parent.name);
        current = parent;
    }
    names.reverse();
    names.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(id: &str, name: &str, parent: Option<&str>) -> Folder {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": name, "folder_parent_id": parent
        }))
        .unwrap()
    }

    fn folders() -> Vec<Folder> {
        vec![
            folder("agents", "Agents", None),
            folder("ci", "CI", Some("agents")),
            folder("deploy", "Deploy", Some("ci")),
            folder("rotation", "Rotation", Some("agents")),
            folder("ops", "Ops", None),
            folder("ops-ci", "CI", Some("ops")),
        ]
    }

    #[test]
    fn everything() {
        let scope = Scope::resolve(&[], &folders()).unwrap();
        assert!(scope.contains(None));
        assert!(scope.contains(Some("ops")));
    }

    #[test]
    fn by_path_with_subfolders() {
        let scope = Scope::resolve(&["Agents/CI".into()], &folders()).unwrap();
        assert!(scope.contains(Some("ci")));
        assert!(scope.contains(Some("deploy")));
        assert!(!scope.contains(Some("agents")));
        assert!(!scope.contains(Some("rotation")));
        assert!(!scope.contains(Some("ops-ci")));
        assert!(!scope.contains(None));
    }

    #[test]
    fn by_id() {
        let scope = Scope::resolve(&["ops-ci".into(), "/Agents/".into()], &folders()).unwrap();
        assert!(scope.contains(Some("ops-ci")));
        assert!(scope.contains(Some("rotation")));
        assert!(!scope.contains(Some("ops")));
    }

    #[test]
    fn unknown() {
        assert!(Scope::resolve(&["Agents/Prod".into()], &folders()).is_err());
    }

    #[test]
    fn ambiguous() {
        let mut folders = folders();
        folders.push(folder("ci-2", "CI", Some("agents")));
        assert!(Scope::resolve(&["Agents/CI".into()], &folders).is_err());
    }
}
