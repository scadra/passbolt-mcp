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

//! The audit file: one JSON line per call of the agent, with the tool, the ids it named, the
//! decision of the rules and the result. Never a secret: only the arguments that name things.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rmcp::model::{CallToolResponse, JsonObject};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::config::AuditConfig;

/// The arguments written to the file: ids, paths, choices. Never passwords, notes,
/// descriptions, comments nor names, which people write and secrets may hide in.
const LOGGED: &[&str] = &[
    "id",
    "folder",
    "parent",
    "comment_id",
    "users",
    "groups",
    "permission",
    "field",
    "favorite",
    "with_content",
    "dry_run",
    "limit",
    "cursor",
];

#[derive(Default)]
pub struct Audit {
    file: Option<Mutex<File>>,
}

/// One line of the file.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub time: String,
    pub tool: String,
    pub arguments: Map<String, Value>,
    /// `ok`, `refused` (the rules), `error` (Passbolt or the arguments) or `protocol`.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl Audit {
    /// The file of `[audit]`, appended to; none when not set.
    pub fn open(config: &AuditConfig) -> Result<Self> {
        let Some(path) = &config.file else {
            return Ok(Self::default());
        };
        if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options
            .open(path)
            .with_context(|| format!("cannot open the audit file {}", path.display()))?;
        Ok(Self {
            file: Some(Mutex::new(file)),
        })
    }

    /// Write the line of a call; a failure to write is logged, not returned to the agent.
    pub fn record(
        &self,
        tool: &str,
        arguments: Option<&JsonObject>,
        response: &Result<CallToolResponse, rmcp::ErrorData>,
    ) {
        if self.file.is_some() {
            self.write(&entry(tool, arguments, response));
        }
    }

    /// Write the line of a request that is not a tool call, such as `resources/read`.
    pub fn record_request(&self, method: &str, arguments: Map<String, Value>, error: Option<&str>) {
        if self.file.is_some() {
            self.write(&Entry {
                time: now(),
                tool: method.into(),
                arguments,
                outcome: match error {
                    None => "ok",
                    Some("refused") => "refused",
                    Some(_) => "error",
                },
                code: error.map(String::from),
            });
        }
    }

    fn write(&self, entry: &Entry) {
        let Some(file) = &self.file else { return };
        let mut line = match serde_json::to_vec(entry) {
            Ok(line) => line,
            Err(error) => return tracing::warn!("audit: {error}"),
        };
        line.push(b'\n');
        let mut file = file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(error) = file.write_all(&line).and_then(|()| file.flush()) {
            tracing::warn!("audit: {error}");
        }
    }
}

/// The line of a call.
#[must_use]
pub fn entry(
    tool: &str,
    arguments: Option<&JsonObject>,
    response: &Result<CallToolResponse, rmcp::ErrorData>,
) -> Entry {
    let arguments = arguments
        .into_iter()
        .flatten()
        .filter(|(name, _)| LOGGED.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let (outcome, code) = match response {
        Ok(CallToolResponse::Complete(result)) if result.is_error == Some(true) => {
            let code = result
                .structured_content
                .as_ref()
                .and_then(|content| content.get("code"))
                .and_then(Value::as_str)
                .map(String::from);
            let outcome = if code.as_deref() == Some("refused") {
                "refused"
            } else {
                "error"
            };
            (outcome, code)
        }
        Ok(_) => ("ok", None),
        Err(error) => ("protocol", Some(error.message.to_string())),
    };
    Entry {
        time: now(),
        tool: tool.into(),
        arguments,
        outcome,
        code,
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use rmcp::model::{CallToolResult, ContentBlock};

    use super::*;

    fn arguments() -> JsonObject {
        serde_json::from_value(serde_json::json!({
            "id": "r1", "password": "s3cret", "note": "the pin is 1234",
            "name": "Staging DB", "description": "x", "content": "y", "dry_run": true
        }))
        .unwrap()
    }

    #[test]
    fn names_things_never_secrets() {
        let ok = Ok(CallToolResult::success(vec![ContentBlock::text("s3cret")]).into());
        let entry = entry("update_resource", Some(&arguments()), &ok);
        assert_eq!(entry.outcome, "ok");
        assert_eq!(
            Value::Object(entry.arguments.clone()),
            serde_json::json!({ "id": "r1", "dry_run": true })
        );
        let line = serde_json::to_string(&entry).unwrap();
        for secret in ["s3cret", "1234", "Staging"] {
            assert!(!line.contains(secret), "{secret}");
        }
    }

    #[test]
    fn outcomes() {
        let refused = crate::error::ToolError::from(crate::rules::Refusal::OutOfScope);
        let response =
            rmcp::handler::server::tool::IntoCallToolResult::into_call_tool_result(refused);
        let entry = entry("get_resource", None, &response);
        assert_eq!(
            (entry.outcome, entry.code.as_deref()),
            ("refused", Some("refused"))
        );
        let unknown = Err(rmcp::ErrorData::invalid_params("tool not found", None));
        assert_eq!(entry_outcome(&unknown), "protocol");
    }

    fn entry_outcome(response: &Result<CallToolResponse, rmcp::ErrorData>) -> &'static str {
        entry("x", None, response).outcome
    }

    #[test]
    fn appends_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs/audit.jsonl");
        let audit = Audit::open(&AuditConfig {
            file: Some(path.clone()),
        })
        .unwrap();
        let ok = Ok(CallToolResult::success(vec![]).into());
        audit.record("whoami", None, &ok);
        audit.record("list_folders", None, &ok);
        let text = std::fs::read_to_string(path).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text
            .lines()
            .last()
            .unwrap()
            .contains("\"tool\":\"list_folders\""));
    }
}
