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

//! The session of the MCP user.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use passbolt_sdk::types::User;
use passbolt_sdk::{NativeHttpClient, OpenPgp, PassboltSdk, SdkConfig};
use secrecy::SecretString;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::config::{Config, PassboltConfig};
use crate::{passphrase, server_key};

pub type Client = PassboltSdk<NativeHttpClient>;

/// What a login needs.
struct Credentials {
    config: PassboltConfig,
    private_key: Zeroizing<String>,
    passphrase: SecretString,
    server_key: String,
}

pub struct Session {
    client: RwLock<Arc<Client>>,
}

impl Session {
    /// Log in as the MCP user.
    pub async fn open(config: &Config) -> Result<Self> {
        let path = &config.passbolt.private_key_file;
        let credentials = Credentials {
            config: config.passbolt.clone(),
            private_key: Zeroizing::new(
                std::fs::read_to_string(path)
                    .with_context(|| format!("cannot read the private key {}", path.display()))?,
            ),
            passphrase: passphrase::read(&config.passbolt)?,
            server_key: server_key::read(config)?,
        };
        let client = login(&credentials).await?;
        let session = Self {
            client: RwLock::new(Arc::new(client)),
        };
        let me = session.me().await?;
        if is_admin(&me) {
            session.client().await.logout().await;
            bail!(
                "{} is an administrator: the MCP user must have the role user",
                display_name(&me)
            );
        }
        Ok(session)
    }

    pub async fn client(&self) -> Arc<Client> {
        self.client.read().await.clone()
    }

    /// The MCP user, with its role.
    pub async fn me(&self) -> Result<User> {
        Ok(self.client().await.get_me().await?)
    }
}

fn is_admin(user: &User) -> bool {
    user.role
        .as_ref()
        .and_then(|role| role.name.as_deref())
        .is_some_and(|name| name.eq_ignore_ascii_case("admin"))
}

/// `First Last <username>`.
#[must_use]
pub fn display_name(user: &User) -> String {
    let profile = user.profile.as_ref();
    let name = [
        profile.and_then(|profile| profile.first_name.as_deref()),
        profile.and_then(|profile| profile.last_name.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    format!("{name} <{}>", user.username.as_deref().unwrap_or_default())
}

async fn login(credentials: &Credentials) -> Result<Client> {
    let sdk_config = SdkConfig {
        server_url: credentials.config.server_url.clone(),
        private_key_armored: credentials.private_key.to_string(),
        passphrase: credentials.passphrase.clone(),
        mfa_mode: "none".into(),
        mfa_totp_token: String::new(),
        mfa_totp_offset: 0,
        mfa_retrys: 3,
        tls_skip_verify: credentials.config.tls_skip_verify,
        tls_client_cert: String::new(),
        tls_client_private_key: String::new(),
        proxy: String::new(),
        server_public_key: credentials.server_key.clone(),
        metadata_key_fingerprint: String::new(),
        server_verify_enc_token: String::new(),
        server_verify_token: String::new(),
        openpgp: OpenPgp::default(),
    };
    let http = server_key::http_client(&credentials.config)?;
    PassboltSdk::login(http, &sdk_config, None, 3)
        .await
        .context("cannot log in as the MCP user")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(role: &str) -> User {
        serde_json::from_value(serde_json::json!({
            "id": "8bb80df6-b7e6-4d1f-9d55-f0cd5a0e5a3b",
            "username": "mcp-agent@example.com",
            "role": { "name": role },
            "profile": { "first_name": "MCP", "last_name": "agent" }
        }))
        .unwrap()
    }

    #[test]
    fn admin() {
        assert!(is_admin(&user("admin")));
        assert!(!is_admin(&user("user")));
    }

    #[test]
    fn name() {
        assert_eq!(
            display_name(&user("user")),
            "MCP agent <mcp-agent@example.com>"
        );
    }
}
