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

//! `[confirm]`: the person says yes before a tool runs, through the client (MCP elicitation).
//!
//! Up to protocol 2025-11-25, the server asks the client (`elicitation/create`) during the call.
//! From 2026-07-28, which forbids requests from the server, the call answers `input_required`
//! with the question, and the client calls again with the answer and the `requestState` given.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::RngCore;
use rmcp::model::{
    CallToolRequestParams, ElicitRequest, ElicitRequestParams, ElicitResult, ElicitationAction,
    ElicitationSchema, InputRequest, InputRequiredResult, JsonObject, ProtocolVersion,
};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde_json::Value;

use crate::config::{ConfirmConfig, Unsupported};
use crate::error::ToolError;
use crate::untrusted;

/// The key of the question in `inputRequests`, and of the answer in `inputResponses`.
const KEY: &str = "confirm";
/// How long the person has to answer.
const WAIT: Duration = Duration::from_secs(300);
/// Never shown to the person: they are secrets.
const HIDDEN: &[&str] = &["password", "note"];

/// What to do with a call.
#[derive(Debug)]
pub enum Gate {
    Go,
    /// Answer `input_required`: the client asks the person, then calls again.
    Ask(InputRequiredResult),
    Refuse(ToolError),
}

#[derive(Debug, Default)]
pub struct Confirm {
    config: ConfirmConfig,
    /// The questions asked with `input_required`, by `requestState`: the call they are about.
    pending: Mutex<HashMap<String, Pending>>,
}

#[derive(Debug)]
struct Pending {
    call: String,
    asked: Instant,
}

impl Confirm {
    #[must_use]
    pub fn new(config: ConfirmConfig) -> Self {
        Self {
            config,
            pending: Mutex::default(),
        }
    }

    /// The tools of `[confirm]`.
    #[must_use]
    pub fn tools(&self) -> &[String] {
        &self.config.tools
    }

    /// What `passbolt-mcp check` says of `[confirm]`: names that are no tool.
    #[must_use]
    pub fn warnings(&self, tools: &[String]) -> Vec<String> {
        self.config
            .tools
            .iter()
            .filter(|name| !tools.contains(name))
            .map(|name| format!("confirm.tools lists {name}, which is no tool"))
            .collect()
    }

    #[must_use]
    pub fn asks(&self, tool: &str) -> bool {
        self.config.tools.iter().any(|name| name == tool)
    }

    /// Whether the call may run: asks the person first when `[confirm]` lists the tool.
    pub async fn gate(
        &self,
        request: &CallToolRequestParams,
        context: &RequestContext<RoleServer>,
    ) -> Gate {
        let tool = request.name.as_ref();
        let arguments = request.arguments.as_ref();
        if !self.asks(tool) || dry_run(arguments) {
            return Gate::Go;
        }
        let can_ask = context
            .client_capabilities()
            .and_then(|capabilities| capabilities.elicitation)
            .is_some_and(|elicitation| elicitation.form.is_some() || elicitation.url.is_none());
        if !can_ask {
            return match self.config.unsupported {
                Unsupported::Allow => Gate::Go,
                Unsupported::Refuse => Gate::Refuse(ToolError::unconfirmed(format!(
                    "{tool} needs the confirmation of the person ([confirm]), and this client \
                     cannot ask for it"
                ))),
            };
        }
        let question = question(tool, arguments);
        if context
            .protocol_version()
            .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28)
        {
            return self.round_trip(request, question);
        }
        match context
            .peer
            .create_elicitation_with_timeout(question, Some(WAIT))
            .await
        {
            Ok(result) => answer(tool, &result),
            Err(error) => {
                tracing::warn!("confirm {tool}: {error}");
                Gate::Refuse(ToolError::unconfirmed(format!(
                    "{tool} needs the confirmation of the person, which did not come"
                )))
            }
        }
    }

    /// From protocol 2026-07-28: the answer of the person when the call carries it, the question
    /// otherwise.
    fn round_trip(&self, request: &CallToolRequestParams, question: ElicitRequestParams) -> Gate {
        let tool = request.name.as_ref();
        let call = call(tool, request.arguments.as_ref());
        let response = request
            .input_responses
            .as_ref()
            .and_then(|responses| responses.get(KEY));
        // Only an answer to a question of this server, about this very call, counts.
        if let (Some(state), Some(response)) = (&request.request_state, response) {
            if self.answered(state, &call) {
                return serde_json::from_value::<ElicitResult>(response.clone()).map_or_else(
                    |_| {
                        Gate::Refuse(ToolError::invalid_argument(format!(
                            "inputResponses.{KEY} is not an answer to the confirmation"
                        )))
                    },
                    |result| answer(tool, &result),
                );
            }
        }
        let state = self.ask(call);
        let requests = BTreeMap::from([(
            KEY.to_string(),
            InputRequest::Elicitation(ElicitRequest::new(question)),
        )]);
        Gate::Ask(InputRequiredResult::new(Some(requests), Some(state)))
    }

    /// A new question about `call`: its `requestState`.
    fn ask(&self, call: String) -> String {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        let state = bytes.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        let mut pending = self.pending();
        pending.retain(|_, question| question.asked.elapsed() < WAIT);
        pending.insert(
            state.clone(),
            Pending {
                call,
                asked: Instant::now(),
            },
        );
        drop(pending);
        state
    }

    /// Whether `state` is a question about `call`, still open; it is closed then.
    fn answered(&self, state: &str, call: &str) -> bool {
        let mut pending = self.pending();
        let open = pending
            .get(state)
            .is_some_and(|asked| asked.call == call && asked.asked.elapsed() < WAIT);
        if open {
            pending.remove(state);
        }
        open
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, HashMap<String, Pending>> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn dry_run(arguments: Option<&JsonObject>) -> bool {
    arguments
        .and_then(|arguments| arguments.get("dry_run"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The tool and its arguments, to tie an answer to the call it was asked for.
fn call(tool: &str, arguments: Option<&JsonObject>) -> String {
    serde_json::to_string(&(tool, arguments)).unwrap_or_default()
}

/// The question: the tool and its arguments, without the secrets nor hidden characters. They
/// come from the agent: quoted, never taken as the words of `passbolt-mcp`.
fn question(tool: &str, arguments: Option<&JsonObject>) -> ElicitRequestParams {
    let mut message = format!("The agent asks to run {tool} in Passbolt");
    let shown: Vec<String> = arguments
        .into_iter()
        .flatten()
        .filter(|(name, _)| !HIDDEN.contains(&name.as_str()) && name.as_str() != "dry_run")
        .map(|(name, value)| format!("{name} = {}", untrusted::visible(&value.to_string())))
        .collect();
    if !shown.is_empty() {
        let _ = write!(message, ", with {}", shown.join(", "));
    }
    message.push_str(". Allow it?");
    ElicitRequestParams::FormElicitationParams {
        meta: None,
        message,
        requested_schema: ElicitationSchema::new(BTreeMap::new()),
    }
}

fn answer(tool: &str, result: &ElicitResult) -> Gate {
    match result.action {
        ElicitationAction::Accept => Gate::Go,
        _ => Gate::Refuse(ToolError::declined(format!(
            "the person declined {tool}: do not try again unless they ask"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(arguments: &Value) -> String {
        let ElicitRequestParams::FormElicitationParams { message, .. } =
            question("update_resource", arguments.as_object())
        else {
            unreachable!()
        };
        message
    }

    #[test]
    fn the_question_hides_the_secrets() {
        let text = message(&serde_json::json!({
            "id": "8f0b",
            "password": "hunter2",
            "note": "the PIN is 1234",
            "name": "Staging\u{200b} DB",
        }));
        assert!(text.contains("update_resource"));
        assert!(text.contains("id = \"8f0b\""));
        assert!(text.contains("name = \"Staging DB\""));
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("1234"));
    }

    #[test]
    fn unknown_tools() {
        let confirm = Confirm::new(ConfirmConfig {
            tools: vec!["get_secret".into(), "get_secrets".into()],
            ..ConfirmConfig::default()
        });
        assert_eq!(
            confirm.warnings(&["get_secret".into()]),
            ["confirm.tools lists get_secrets, which is no tool"]
        );
    }

    #[test]
    fn dry_runs_are_not_asked() {
        assert!(dry_run(serde_json::json!({ "dry_run": true }).as_object()));
        assert!(!dry_run(serde_json::json!({ "id": "8f0b" }).as_object()));
        assert!(!dry_run(None));
    }

    #[test]
    fn the_answer_is_tied_to_the_call() {
        let a = serde_json::json!({ "id": "a" });
        let b = serde_json::json!({ "id": "b" });
        assert_ne!(
            call("delete_resource", a.as_object()),
            call("delete_resource", b.as_object())
        );
        assert_ne!(
            call("delete_resource", a.as_object()),
            call("delete_folder", a.as_object())
        );
    }
}
