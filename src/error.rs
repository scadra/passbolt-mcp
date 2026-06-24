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

//! The error a tool returns to the agent: a message, and a code it can act on.

use passbolt_sdk::{ApiError, PassboltError};
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
use serde::Serialize;

use crate::rules::Refusal;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ToolError {
    /// `refused`, `forbidden`, `not_found`, `invalid_argument`, `auth`, `crypto`, `transport`,
    /// `api` or `internal`.
    pub code: &'static str,
    pub message: String,
    /// The HTTP status Passbolt answered, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl ToolError {
    const fn new(code: &'static str, message: String) -> Self {
        Self {
            code,
            message,
            status: None,
        }
    }
}

impl ToolError {
    #[must_use]
    pub const fn invalid_argument(message: String) -> Self {
        Self::new("invalid_argument", message)
    }

    /// What the permission of the MCP user does not allow, known before asking Passbolt.
    #[must_use]
    pub const fn forbidden(message: String) -> Self {
        Self::new("forbidden", message)
    }
}

impl From<Refusal> for ToolError {
    fn from(refusal: Refusal) -> Self {
        Self::new("refused", refusal.to_string())
    }
}

impl From<PassboltError> for ToolError {
    fn from(error: PassboltError) -> Self {
        let code = match &error {
            PassboltError::Api(ApiError { status: 403, .. }) => "forbidden",
            PassboltError::Api(ApiError { status: 404, .. }) | PassboltError::NotFound(_) => {
                "not_found"
            }
            PassboltError::Api(_) => "api",
            PassboltError::InvalidArgument(_) => "invalid_argument",
            PassboltError::Auth(_) => "auth",
            PassboltError::Crypto(_) => "crypto",
            PassboltError::Transport(_) => "transport",
            _ => "internal",
        };
        let status = match &error {
            PassboltError::Api(api) => Some(api.status),
            _ => None,
        };
        let message = if code == "forbidden" {
            format!("Passbolt refuses: the permission of the MCP user does not allow it ({error})")
        } else {
            error.to_string()
        };
        Self {
            code,
            message,
            status,
        }
    }
}

impl From<anyhow::Error> for ToolError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast::<PassboltError>() {
            Ok(error) => error.into(),
            Err(error) => match error.downcast::<Refusal>() {
                Ok(refusal) => refusal.into(),
                Err(error) => Self::new("internal", format!("{error:#}")),
            },
        }
    }
}

impl IntoCallToolResult for ToolError {
    fn into_call_tool_result(self) -> Result<CallToolResponse, rmcp::ErrorData> {
        let mut result = CallToolResult::error(vec![ContentBlock::text(&self.message)]);
        result.structured_content = serde_json::to_value(&self).ok();
        Ok(result.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(status: u16) -> PassboltError {
        PassboltError::Api(ApiError {
            status,
            message: "nope".into(),
            validation_errors: None,
        })
    }

    #[test]
    fn codes() {
        assert_eq!(ToolError::from(api(403)).code, "forbidden");
        assert_eq!(ToolError::from(api(404)).code, "not_found");
        assert_eq!(ToolError::from(api(500)).status, Some(500));
        assert_eq!(
            ToolError::from(PassboltError::InvalidArgument("x".into())).code,
            "invalid_argument"
        );
    }

    #[test]
    fn refusal_names_the_rule() {
        let error = ToolError::from(Refusal::OutOfScope);
        assert_eq!(error.code, "refused");
        assert_eq!(
            error.message,
            "refused by the MCP rules: outside scope.folders"
        );
    }

    #[test]
    fn through_anyhow() {
        assert_eq!(
            ToolError::from(anyhow::Error::from(api(403))).code,
            "forbidden"
        );
        assert_eq!(
            ToolError::from(anyhow::Error::from(Refusal::OutOfScope)).code,
            "refused"
        );
        assert_eq!(ToolError::from(anyhow::anyhow!("boom")).code, "internal");
    }

    #[test]
    fn is_an_error_result() {
        let CallToolResponse::Complete(result) = ToolError::from(Refusal::OutOfScope)
            .into_call_tool_result()
            .unwrap()
        else {
            panic!("not a complete result");
        };
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.structured_content.unwrap()["code"], "refused");
    }
}
