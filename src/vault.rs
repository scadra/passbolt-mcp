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

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

use crate::error::ToolError;

pub const HANDLE: &str = "passbolt:secret/";
pub const SCRUBBED: &str = "[secret]";
const TTL: Duration = Duration::from_secs(15 * 60);

pub struct Vault {
    ttl: Duration,
    handles: Mutex<HashMap<String, (Zeroizing<String>, Instant)>>,
}

impl Default for Vault {
    fn default() -> Self {
        Self::new(TTL)
    }
}

impl Vault {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            handles: Mutex::default(),
        }
    }

    pub fn put(&self, value: String) -> String {
        let mut bytes = [0u8; 16];
        OsRng.fill_bytes(&mut bytes);
        let handle = bytes.iter().fold(String::from(HANDLE), |mut handle, byte| {
            let _ = write!(handle, "{byte:02x}");
            handle
        });
        let now = Instant::now();
        let mut handles = self.handles.lock().expect("vault");
        handles.retain(|_, (_, expires)| *expires > now);
        handles.insert(handle.clone(), (Zeroizing::new(value), now + self.ttl));
        handle
    }

    pub fn get(&self, handle: &str) -> Result<Zeroizing<String>, ToolError> {
        let handles = self.handles.lock().expect("vault");
        match handles.get(handle) {
            Some((value, expires)) if *expires > Instant::now() => Ok(value.clone()),
            _ => Err(ToolError::invalid_argument(format!(
                "{handle}: no such handle, or expired: get a new one"
            ))),
        }
    }

    pub fn reveal(&self, value: Option<String>) -> Result<Option<String>, ToolError> {
        match value {
            Some(handle) if handle.starts_with(HANDLE) => {
                Ok(Some(self.get(&handle)?.as_str().to_owned()))
            }
            value => Ok(value),
        }
    }
}

#[must_use]
pub fn scrub(text: &str, secrets: &[Zeroizing<String>]) -> String {
    let mut forms: Vec<Zeroizing<String>> = Vec::new();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        let bytes = secret.as_bytes();
        forms.push(secret.clone());
        for engine in [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD] {
            forms.push(Zeroizing::new(engine.encode(bytes)));
        }
        forms.push(Zeroizing::new(percent_encoded(bytes)));
        forms.push(Zeroizing::new(hex(bytes)));
    }
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    let mut text = text.to_owned();
    for form in &forms {
        text = text.replace(form.as_str(), SCRUBBED);
    }
    text
}

fn percent_encoded(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) {
            out.push(char::from(*byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles() {
        let vault = Vault::default();
        let handle = vault.put("s3cret".into());
        assert!(handle.starts_with(HANDLE));
        assert_eq!(handle.len(), HANDLE.len() + 32);
        assert!(!handle.contains("s3cret"));
        assert_eq!(vault.get(&handle).unwrap().as_str(), "s3cret");
        assert_ne!(handle, vault.put("s3cret".into()));
    }

    #[test]
    fn unknown_or_expired() {
        let vault = Vault::new(Duration::ZERO);
        let handle = vault.put("s3cret".into());
        let error = vault.get(&handle).unwrap_err();
        assert!(error.message.contains("expired"));
        assert!(!error.message.contains("s3cret"));
        assert!(Vault::default().get("passbolt:secret/00").is_err());
    }

    #[test]
    fn reveal() {
        let vault = Vault::default();
        let handle = vault.put("s3cret".into());
        assert_eq!(vault.reveal(Some(handle)).unwrap().unwrap(), "s3cret");
        assert_eq!(
            vault.reveal(Some("plain".into())).unwrap().unwrap(),
            "plain"
        );
        assert_eq!(vault.reveal(None).unwrap(), None);
    }

    #[test]
    fn scrubbed() {
        let secret = Zeroizing::new("p@ss word/1".to_string());
        let text = format!(
            "raw {} b64 {} url {} pct {} hex {}",
            secret.as_str(),
            STANDARD.encode(secret.as_bytes()),
            URL_SAFE_NO_PAD.encode(secret.as_bytes()),
            percent_encoded(secret.as_bytes()),
            hex(secret.as_bytes()),
        );
        assert_eq!(
            scrub(&text, &[secret]),
            "raw [secret] b64 [secret] url [secret] pct [secret] hex [secret]"
        );
        assert_eq!(
            scrub("nothing", &[Zeroizing::new(String::new())]),
            "nothing"
        );
    }
}
