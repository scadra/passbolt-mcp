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

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::commands::RESOURCE;
use crate::error::ToolError;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateConfig {
    pub source: PathBuf,
    pub target: PathBuf,
    pub description: Option<String>,
}

impl TemplateConfig {
    pub fn check(&self, name: &str) -> Result<()> {
        ensure!(
            self.source.is_absolute() && self.target.is_absolute(),
            "templates.{name}: source and target are absolute paths"
        );
        ensure!(
            self.source != self.target,
            "templates.{name}: the target cannot be the source"
        );
        Ok(())
    }
}

pub fn check_all(templates: &BTreeMap<String, TemplateConfig>) -> Result<()> {
    for (name, template) in templates {
        template.check(name)?;
    }
    Ok(())
}

/// A `{{ passbolt:… }}` of a template: a resource and the field of its secret, or a handle.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Placeholder {
    Resource { id: String, field: String },
    Handle(String),
}

#[derive(Debug, PartialEq, Eq)]
enum Part<'a> {
    Text(&'a str),
    Secret(Placeholder),
}

fn parse(text: &str) -> Result<Vec<Part<'_>>, ToolError> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let inner = after[..end].trim();
        if let Some(reference) = inner.strip_prefix(RESOURCE) {
            parts.push(Part::Text(&rest[..start]));
            parts.push(Part::Secret(placeholder(reference)?));
        } else {
            parts.push(Part::Text(&rest[..start + 2 + end + 2]));
        }
        rest = &after[end + 2..];
    }
    parts.push(Part::Text(rest));
    Ok(parts)
}

fn placeholder(reference: &str) -> Result<Placeholder, ToolError> {
    if reference.starts_with("secret/") {
        return Ok(Placeholder::Handle(format!("{RESOURCE}{reference}")));
    }
    let (id, field) = reference.split_once('#').unwrap_or((reference, "password"));
    if id.is_empty() || field.is_empty() {
        return Err(ToolError::invalid_argument(format!(
            "{{{{ {RESOURCE}{reference} }}}}: write {{{{ {RESOURCE}<resource id> }}}}, \
             {{{{ {RESOURCE}<resource id>#<field> }}}} or a handle"
        )));
    }
    Ok(Placeholder::Resource {
        id: id.into(),
        field: field.into(),
    })
}

/// The placeholders of `text`, each once.
pub fn placeholders(text: &str) -> Result<Vec<Placeholder>, ToolError> {
    let mut found: Vec<Placeholder> = parse(text)?
        .into_iter()
        .filter_map(|part| match part {
            Part::Secret(placeholder) => Some(placeholder),
            Part::Text(_) => None,
        })
        .collect();
    found.sort();
    found.dedup();
    Ok(found)
}

/// `text` with each placeholder replaced by its value in `values`.
pub fn render(
    text: &str,
    values: &BTreeMap<Placeholder, Zeroizing<String>>,
) -> Result<Zeroizing<String>, ToolError> {
    let mut out = Zeroizing::new(String::with_capacity(text.len()));
    for part in parse(text)? {
        match part {
            Part::Text(text) => out.push_str(text),
            Part::Secret(placeholder) => {
                out.push_str(values.get(&placeholder).ok_or_else(|| {
                    ToolError::invalid_argument("a placeholder has no value".into())
                })?);
            }
        }
    }
    Ok(out)
}

/// Writes `content` to `target`, readable by its owner only, replacing it at once.
pub fn write(target: &Path, content: &str) -> Result<(), ToolError> {
    let fail = |error: std::io::Error| {
        ToolError::invalid_argument(format!(
            "{} could not be written: {error}",
            target.display()
        ))
    };
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(".passbolt-mcp-");
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600));
    let mut file = builder.tempfile_in(dir).map_err(fail)?;
    file.write_all(content.as_bytes()).map_err(fail)?;
    file.persist(target).map_err(|error| fail(error.error))?;
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`fill_template`.")]
pub struct FillTemplate {
    #[schemars(description = "The name of a template of the configuration, as `whoami` lists it.")]
    pub template: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "What `fill_template` wrote, never the secrets.")]
pub struct Filled {
    pub template: String,
    pub target: String,
    #[schemars(description = "How many different secrets went in.")]
    pub secrets: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TemplateView {
    pub name: String,
    pub description: Option<String>,
    #[schemars(
        description = "The file the agent writes, with `{{ passbolt:<resource id> }}` where a secret goes."
    )]
    pub source: String,
    #[schemars(description = "The file passbolt-mcp writes; the agent cannot read it.")]
    pub target: String,
}

impl TemplateView {
    #[must_use]
    pub fn new(name: &str, template: &TemplateConfig) -> Self {
        Self {
            name: name.into(),
            description: template.description.clone(),
            source: template.source.display().to_string(),
            target: template.target.display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found() {
        let text = "DB={{ passbolt:r1 }}\nUSER={{passbolt:r1#username}}\nX={{ other }}\n\
                    T={{ passbolt:secret/ab12 }}\nAGAIN={{ passbolt:r1 }}";
        assert_eq!(
            placeholders(text).unwrap(),
            [
                Placeholder::Resource {
                    id: "r1".into(),
                    field: "password".into()
                },
                Placeholder::Resource {
                    id: "r1".into(),
                    field: "username".into()
                },
                Placeholder::Handle("passbolt:secret/ab12".into()),
            ]
        );
        assert!(placeholders("{{ passbolt: }}").is_err());
        assert!(placeholders("{{ passbolt:r1# }}").is_err());
    }

    #[test]
    fn rendered() {
        let text = "a={{ passbolt:r1 }} b={{ other }} c={{passbolt:r1}} d={{ unclosed";
        let values = BTreeMap::from([(
            Placeholder::Resource {
                id: "r1".into(),
                field: "password".into(),
            },
            Zeroizing::new("s3cret".to_string()),
        )]);
        assert_eq!(
            render(text, &values).unwrap().as_str(),
            "a=s3cret b={{ other }} c=s3cret d={{ unclosed"
        );
        assert!(render("{{ passbolt:r2 }}", &values).is_err());
    }

    #[test]
    fn checked() {
        let template = |source: &str, target: &str| TemplateConfig {
            source: source.into(),
            target: target.into(),
            description: None,
        };
        assert!(template("/a/env.tpl", "/b/.env").check("x").is_ok());
        assert!(template("env.tpl", "/b/.env").check("x").is_err());
        assert!(template("/a/env", "/a/env").check("x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn written_for_the_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(".env");
        std::fs::write(&target, "old").unwrap();
        write(&target, "DB=s3cret").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "DB=s3cret");
        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
