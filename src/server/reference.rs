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

//! `docs/tools-reference.md`, written from the schemas of the tools.

use std::fmt::Write;

use serde_json::Value;

use super::{needs, PassboltMcp};
use crate::rules::Access;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/tools-reference.md");

const SECTIONS: [(Access, &str, &str); 4] = [
    (Access::Read, "Reading", "Always offered."),
    (Access::Secrets, "Secrets", "With `access.secrets = true`."),
    (Access::Write, "Writing", "With `access.write = true`."),
    (Access::Share, "Sharing", "With `access.share = true`."),
];

/// The type of a property, as the schema gives it.
fn kind(schema: &Value) -> String {
    if let Some(name) = schema["$ref"].as_str() {
        return name.rsplit('/').next().unwrap_or(name).to_string();
    }
    if let Some(values) = schema["enum"].as_array() {
        let values: Vec<String> = values
            .iter()
            .filter(|value| !value.is_null())
            .map(|value| format!("`{}`", value.as_str().unwrap_or_default()))
            .collect();
        return values.join(" \\| ");
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(variants) = schema[key].as_array() {
            let kinds: Vec<String> = variants
                .iter()
                .filter(|variant| variant["type"] != "null")
                .map(kind)
                .collect();
            return kinds.join(" \\| ");
        }
    }
    let name = match &schema["type"] {
        Value::String(name) => name.as_str(),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .find(|name| *name != "null")
            .unwrap_or_default(),
        _ => "",
    };
    match name {
        "array" => format!("array of {}", kind(&schema["items"])),
        "integer" | "number" => "number".to_string(),
        name => name.to_string(),
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

fn render() -> String {
    let mut tools = PassboltMcp::tool_router().list_all();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::from(
        "---\ntitle: Tools reference\nsidebar_position: 5\n---\n\n\
         <!-- Written by `UPDATE_DOCS=1 cargo test tools_reference`: do not edit. -->\n\n\
         # Tools reference\n\n\
         Every tool and its arguments, from the schemas the agent gets. \
         [Tools](tools.md) says how they work together.\n",
    );
    for (access, title, offered) in SECTIONS {
        let _ = write!(out, "\n## {title}\n\n{offered}\n");
        for tool in tools
            .iter()
            .filter(|tool| needs(&tool.name) == Some(access))
        {
            let _ = write!(out, "\n### `{}`\n\n", tool.name);
            if let Some(description) = &tool.description {
                let _ = writeln!(out, "{}", one_line(description));
            }
            let hints = tool.annotations.as_ref().map(|hints| {
                [
                    (hints.read_only_hint, "Read only"),
                    (hints.destructive_hint, "Destructive"),
                ]
                .into_iter()
                .filter(|(hint, _)| *hint == Some(true))
                .map(|(_, name)| name)
                .collect::<Vec<_>>()
            });
            if let Some(hints) = hints.filter(|hints| !hints.is_empty()) {
                let _ = writeln!(out, "\n*{}.*", hints.join(", "));
            }
            let schema = tool.input_schema.as_ref();
            let required: Vec<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .map(|names| names.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                if !properties.is_empty() {
                    out.push_str("\n| Argument | Type | Description |\n|---|---|---|\n");
                }
                for (name, property) in properties {
                    let name = if required.contains(&name.as_str()) {
                        format!("`{name}` (required)")
                    } else {
                        format!("`{name}`")
                    };
                    let description = one_line(property["description"].as_str().unwrap_or(""));
                    let _ = writeln!(out, "| {name} | {} | {description} |", kind(property));
                }
            }
        }
    }
    out
}

#[test]
fn tools_reference() {
    let reference = render();
    if std::env::var_os("UPDATE_DOCS").is_some() {
        std::fs::write(PATH, reference).unwrap();
        return;
    }
    let written = std::fs::read_to_string(PATH).unwrap_or_default();
    assert!(
        written == reference,
        "docs/tools-reference.md is out of date: UPDATE_DOCS=1 cargo test tools_reference"
    );
}
