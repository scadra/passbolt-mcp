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

use rand::rngs::OsRng;
use rand::seq::SliceRandom;
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

/// `get_totp_code`: the current code, never the seed.
#[derive(Serialize, JsonSchema)]
pub struct TotpCode {
    pub id: String,
    pub code: String,
    /// How long the code stays valid.
    pub seconds_remaining: u64,
}

/// `check_password_pwned`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Pwned {
    pub id: String,
    /// The password is in a known breach: change it.
    pub pwned: bool,
    /// How many times Have I Been Pwned saw it.
    pub breaches: u64,
}

/// `generate_password`.
// Four independent character classes, as the arguments of a tool.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(default)]
pub struct GeneratePassword {
    /// 20 by default, from 8 to 128.
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for GeneratePassword {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
        }
    }
}

#[derive(Serialize, JsonSchema)]
pub struct GeneratedPassword {
    /// Not saved anywhere: give it to `create_resource` or `update_resource`.
    pub password: String,
}

const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!#$%&()*+,-./:;<=>?@[]^_{|}~";

/// A random password from the system randomness, with at least one character of each class.
pub fn generate(args: &GeneratePassword) -> Result<String, ToolError> {
    if !(8..=128).contains(&args.length) {
        return Err(ToolError::invalid_argument("length: from 8 to 128".into()));
    }
    let classes: Vec<&[u8]> = [
        (args.lowercase, LOWERCASE),
        (args.uppercase, UPPERCASE),
        (args.digits, DIGITS),
        (args.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter_map(|(wanted, class)| wanted.then_some(class))
    .collect();
    if classes.is_empty() {
        return Err(ToolError::invalid_argument(
            "choose at least one of lowercase, uppercase, digits, symbols".into(),
        ));
    }
    let all: Vec<u8> = classes.concat();
    let mut password: Vec<u8> = classes
        .iter()
        .map(|class| *class.choose(&mut OsRng).expect("a class"))
        .collect();
    password.extend((password.len()..args.length).map(|_| *all.choose(&mut OsRng).expect("chars")));
    password.shuffle(&mut OsRng);
    Ok(String::from_utf8(password).expect("ASCII"))
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
        .filter(|(name, value)| {
            value.is_string() && !matches!(name.as_str(), "totp" | "object_type")
        })
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
    fn generated() {
        let password = generate(&GeneratePassword::default()).unwrap();
        assert_eq!(password.len(), 20);
        for class in [LOWERCASE, UPPERCASE, DIGITS, SYMBOLS] {
            assert!(password.bytes().any(|byte| class.contains(&byte)));
        }
        assert_ne!(password, generate(&GeneratePassword::default()).unwrap());
    }

    #[test]
    fn digits_only() {
        let args = GeneratePassword {
            length: 8,
            lowercase: false,
            uppercase: false,
            symbols: false,
            ..GeneratePassword::default()
        };
        assert!(generate(&args)
            .unwrap()
            .bytes()
            .all(|byte| byte.is_ascii_digit()));
    }

    #[test]
    fn bad_arguments() {
        let short = GeneratePassword {
            length: 4,
            ..GeneratePassword::default()
        };
        assert!(generate(&short).is_err());
        let nothing = GeneratePassword {
            lowercase: false,
            uppercase: false,
            digits: false,
            symbols: false,
            ..GeneratePassword::default()
        };
        assert!(generate(&nothing).is_err());
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
            "the secret has no field pin; it has: description, password"
        );
        assert!(!error.message.contains("s3cret"));
    }
}
