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

//! What the secret tools return, with `access.secrets` only. A secret goes to the agent and
//! nowhere else: never to the logs, the errors or the audit file.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::ToolError;

/// `get_secret`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSecret {
    /// The id of the resource.
    pub id: String,
    /// The field of the secret: `password` by default, or `description`, the note of a v5 resource.
    pub field: Option<String>,
}

/// One field of the secret of a resource.
#[derive(Serialize, JsonSchema)]
pub struct SecretField {
    pub id: String,
    pub field: String,
    /// The secret itself: use it for the task, do not repeat it.
    pub value: String,
}

/// The `field` of `secret`: a string. The TOTP has its own tool, which never gives the seed.
pub fn field(secret: &Map<String, Value>, field: &str) -> Result<String, ToolError> {
    if field == "totp" {
        return Err(ToolError::invalid_argument(
            "the TOTP seed is never given: call get_totp_code for the current code".into(),
        ));
    }
    if let Some(Value::String(value)) = secret.get(field) {
        return Ok(value.clone());
    }
    let mut fields: Vec<&str> = secret
        .iter()
        .filter(|(name, value)| value.is_string() && name.as_str() != "totp")
        .map(|(name, _)| name.as_str())
        .collect();
    fields.sort_unstable();
    Err(ToolError::invalid_argument(format!(
        "the secret has no field {field}; it has: {}",
        fields.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret() -> Map<String, Value> {
        serde_json::from_value(serde_json::json!({
            "object_type": "PASSBOLT_SECRET_DATA",
            "password": "s3cret",
            "description": "the note",
            "totp": { "secret_key": "JBSWY3DPEHPK3PXP" }
        }))
        .unwrap()
    }

    #[test]
    fn a_field() {
        assert_eq!(field(&secret(), "password").unwrap(), "s3cret");
        assert_eq!(field(&secret(), "description").unwrap(), "the note");
    }

    #[test]
    fn never_the_seed() {
        let error = field(&secret(), "totp").unwrap_err();
        assert!(error.message.contains("get_totp_code"));
        assert!(!error.message.contains("JBSW"));
    }

    #[test]
    fn unknown_field_lists_names_not_values() {
        let error = field(&secret(), "pin").unwrap_err();
        assert_eq!(
            error.message,
            "the secret has no field pin; it has: description, object_type, password"
        );
        assert!(!error.message.contains("s3cret"));
    }
}
