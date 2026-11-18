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

use passbolt_sdk::generator::{
    generate_passphrase, generate_password, generate_pin_code, generate_secret, secret_entropy,
    GeneratorType, PasswordPolicies, Strength, PIN_CODE_MAX_LENGTH, PIN_CODE_MIN_LENGTH,
};
use passbolt_sdk::PassboltError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use crate::error::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`get_secret`.")]
pub struct GetSecret {
    #[schemars(description = "The id of the resource.")]
    pub id: String,
    #[schemars(
        description = "The field of the secret: `password` by default, or `description`, the note of a v5 resource."
    )]
    pub field: Option<String>,
}

#[derive(Serialize, JsonSchema)]
#[schemars(description = "One field of the secret of a resource.")]
pub struct SecretField {
    pub id: String,
    pub field: String,
    #[schemars(description = "The secret itself: use it for the task, do not repeat it.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[schemars(
        description = "With `access.mask_secrets`, in place of the value: give it to `create_resource`, \
        `update_resource` or `run_with_secret`. It works on this server only, for 15 minutes."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Serialize, JsonSchema)]
#[schemars(description = "`show_secret`: what the agent learns, never the secret.")]
pub struct ShownSecret {
    pub id: String,
    pub field: String,
    #[schemars(description = "The client showed the secret to the person.")]
    pub shown: bool,
}

#[derive(Serialize, JsonSchema)]
#[schemars(description = "`get_totp_code`: the current code, never the seed.")]
pub struct TotpCode {
    pub id: String,
    pub code: String,
    #[schemars(description = "How long the code stays valid.")]
    pub seconds_remaining: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "`check_password_pwned`.")]
pub struct Pwned {
    pub id: String,
    #[schemars(description = "The password is in a known breach: change it.")]
    pub pwned: bool,
    #[schemars(description = "How many times Have I Been Pwned saw it.")]
    pub breaches: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Password,
    Passphrase,
    Pin,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[schemars(description = "`generate_password`.")]
#[serde(default)]
pub struct GeneratePassword {
    #[schemars(
        description = "`password`, `passphrase` or `pin`; the generator of the password policy of the \
        organization by default."
    )]
    pub kind: Option<Kind>,
    #[schemars(
        description = "Characters of a password, digits of a PIN code (4 to 12, 6 by default); the policy's by \
        default."
    )]
    pub length: Option<u32>,
    #[schemars(description = "Words of a passphrase; the policy's by default.")]
    pub words: Option<u32>,
    #[schemars(
        description = "`false` for letters and digits only, for a service that refuses symbols; the policy's \
        character sets by default."
    )]
    pub symbols: Option<bool>,
}

#[derive(Serialize, JsonSchema)]
pub struct GeneratedPassword {
    #[schemars(
        description = "Not saved anywhere: give it to `create_resource` or `update_resource`."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[schemars(
        description = "With `access.mask_secrets`, in place of the password: give it to `create_resource` \
        or `update_resource`. It works on this server only, for 15 minutes."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    pub kind: Kind,
    #[schemars(
        description = "As Passbolt shows it: `Very weak`, `Weak`, `Fair`, `Strong`, `Very strong`."
    )]
    pub strength: String,
    pub entropy_bits: f64,
    #[schemars(
        description = "Weaker than what the password policy of the organization generates: say so to the person."
    )]
    pub below_policy: bool,
}

pub struct Generated {
    pub secret: Zeroizing<String>,
    pub kind: Kind,
    pub strength: Strength,
    pub entropy_bits: f64,
    pub below_policy: bool,
}

const PIN_LENGTH: u32 = 6;

fn invalid(error: &PassboltError) -> ToolError {
    ToolError::invalid_argument(error.to_string())
}

pub fn generate(
    args: &GeneratePassword,
    policies: &PasswordPolicies,
) -> Result<Generated, ToolError> {
    let kind = args.kind.unwrap_or(match policies.default_generator {
        GeneratorType::Password => Kind::Password,
        GeneratorType::Passphrase => Kind::Passphrase,
    });
    let secret = Zeroizing::new(match kind {
        Kind::Password => {
            let mut settings = policies.password_generator_settings.clone();
            if let Some(length) = args.length {
                settings.length = length;
            }
            if args.symbols == Some(false) {
                settings.mask_char1 = false;
                settings.mask_char2 = false;
                settings.mask_char3 = false;
                settings.mask_char4 = false;
                settings.mask_char5 = false;
                settings.mask_parenthesis = false;
                settings.mask_emoji = false;
            }
            generate_password(&settings).map_err(|error| invalid(&error))?
        }
        Kind::Passphrase => {
            let mut settings = policies.passphrase_generator_settings.clone();
            if let Some(words) = args.words {
                settings.words = words;
            }
            generate_passphrase(&settings).map_err(|error| invalid(&error))?
        }
        Kind::Pin => {
            let length = args.length.unwrap_or(PIN_LENGTH);
            if !(PIN_CODE_MIN_LENGTH..=PIN_CODE_MAX_LENGTH).contains(&length) {
                return Err(ToolError::invalid_argument(format!(
                    "a PIN code has from {PIN_CODE_MIN_LENGTH} to {PIN_CODE_MAX_LENGTH} digits, not {length}"
                )));
            }
            generate_pin_code(length)
        }
    });
    let entropy_bits = secret_entropy(&secret);
    let strength = Strength::of(entropy_bits);
    let policy = generate_secret(policies).map_or(Strength::NotAvailable, |reference| {
        Strength::of(secret_entropy(&Zeroizing::new(reference)))
    });
    Ok(Generated {
        secret,
        kind,
        strength,
        entropy_bits: (entropy_bits * 10.0).round() / 10.0,
        below_policy: strength < policy,
    })
}

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

    fn args(kind: Kind) -> GeneratePassword {
        GeneratePassword {
            kind: Some(kind),
            ..GeneratePassword::default()
        }
    }

    #[test]
    fn the_policy_by_default() {
        let policies = PasswordPolicies::default();
        let generated = generate(&GeneratePassword::default(), &policies).unwrap();
        assert_eq!(generated.kind, Kind::Password);
        assert_eq!(generated.secret.chars().count(), 18);
        assert!(generated.strength >= Strength::Strong);
        assert!(!generated.below_policy);
        let other = generate(&GeneratePassword::default(), &policies).unwrap();
        assert_ne!(generated.secret, other.secret);
        let passphrases = PasswordPolicies {
            default_generator: GeneratorType::Passphrase,
            ..PasswordPolicies::default()
        };
        let generated = generate(&GeneratePassword::default(), &passphrases).unwrap();
        assert_eq!(generated.kind, Kind::Passphrase);
        assert_eq!(generated.secret.split(' ').count(), 9);
    }

    #[test]
    fn shorter_is_below_the_policy() {
        let short = GeneratePassword {
            length: Some(8),
            ..args(Kind::Password)
        };
        let generated = generate(&short, &PasswordPolicies::default()).unwrap();
        assert_eq!(generated.secret.len(), 8);
        assert!(generated.below_policy);
        assert_eq!(generated.strength, Strength::VeryWeak);
    }

    #[test]
    fn without_symbols() {
        let plain = GeneratePassword {
            length: Some(40),
            symbols: Some(false),
            ..args(Kind::Password)
        };
        let generated = generate(&plain, &PasswordPolicies::default()).unwrap();
        assert!(generated.secret.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn passphrase_and_pin() {
        let words = GeneratePassword {
            words: Some(5),
            ..args(Kind::Passphrase)
        };
        let generated = generate(&words, &PasswordPolicies::default()).unwrap();
        assert_eq!(generated.secret.split(' ').count(), 5);
        let pin = generate(&args(Kind::Pin), &PasswordPolicies::default()).unwrap();
        assert_eq!(pin.secret.len(), 6);
        assert!(pin.secret.bytes().all(|byte| byte.is_ascii_digit()));
        assert!(pin.below_policy);
    }

    #[test]
    fn bad_arguments() {
        let policies = PasswordPolicies::default();
        let long = GeneratePassword {
            length: Some(500),
            ..args(Kind::Password)
        };
        assert!(generate(&long, &policies).is_err());
        let pin = GeneratePassword {
            length: Some(3),
            ..args(Kind::Pin)
        };
        assert!(generate(&pin, &policies)
            .err()
            .unwrap()
            .message
            .contains("4 to 12"));
        let words = GeneratePassword {
            words: Some(1),
            ..args(Kind::Passphrase)
        };
        assert!(generate(&words, &policies).is_err());
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
