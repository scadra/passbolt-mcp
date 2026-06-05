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

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use passbolt_sdk::mfa::TotpProvider;
use passbolt_sdk::types::User;
use passbolt_sdk::{
    ApiError, AuthError, MfaProvider, NativeHttpClient, OpenPgp, PassboltError, PassboltSdk,
    SdkConfig,
};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::config::{Config, PassboltConfig};
use crate::{passphrase, server_key};

pub type Client = PassboltSdk<NativeHttpClient>;

const KEEP_ALIVE: Duration = Duration::from_secs(5 * 60);

/// What a login needs, kept to log in again.
struct Credentials {
    config: PassboltConfig,
    private_key: Zeroizing<String>,
    passphrase: SecretString,
    server_key: String,
    totp_secret: Option<SecretString>,
}

pub struct Session {
    credentials: Credentials,
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
            totp_secret: config
                .passbolt
                .totp_secret_file
                .as_deref()
                .map(passphrase::from_file)
                .transpose()?,
        };
        let client = login(&credentials).await?;
        let session = Self {
            credentials,
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
        self.call(|client| async move { client.get_me().await })
            .await
    }

    /// Keep the session alive between calls of the agent, as the browser extension does.
    pub fn keep_alive(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let session = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(KEEP_ALIVE);
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(error) = session
                    .call(|client| async move { client.keep_alive().await })
                    .await
                {
                    tracing::warn!("keep alive: {error:#}");
                }
            }
        })
    }

    /// Call the API; when the session expired, log in again and call once more.
    pub async fn call<T, F, Fut>(&self, call: F) -> Result<T>
    where
        T: Send,
        F: Fn(Arc<Client>) -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, PassboltError>> + Send,
    {
        let client = self.client().await;
        match call(client.clone()).await {
            Err(error) if is_expired(&error) => {
                self.login_again(&client).await?;
                Ok(call(self.client().await).await?)
            }
            result => Ok(result?),
        }
    }

    /// Log in again, unless a concurrent call did already: the others wait for this login.
    async fn login_again(&self, expired: &Arc<Client>) -> Result<()> {
        let mut client = self.client.write().await;
        if Arc::ptr_eq(&client, expired) {
            tracing::info!("the session expired: logging in again");
            *client = Arc::new(login(&self.credentials).await?);
        }
        drop(client);
        Ok(())
    }
}

const fn is_expired(error: &PassboltError) -> bool {
    matches!(
        error,
        PassboltError::Api(ApiError { status: 401, .. })
            | PassboltError::Auth(AuthError::SessionExpired)
    )
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
    // MFA, when enforced for the MCP user: the code of its TOTP secret.
    let mfa = credentials
        .totp_secret
        .as_ref()
        .map(|secret| -> Result<Box<dyn MfaProvider>> {
            Ok(Box::new(TotpProvider::new(
                secret.expose_secret(),
                0,
                Duration::from_secs(1),
            )?))
        })
        .transpose()?;
    let http = server_key::http_client(&credentials.config)?;
    PassboltSdk::login(http, &sdk_config, mfa, 3)
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
    fn expired() {
        let api = |status| {
            PassboltError::Api(ApiError {
                status,
                message: String::new(),
                validation_errors: None,
            })
        };
        assert!(is_expired(&api(401)));
        assert!(is_expired(&AuthError::SessionExpired.into()));
        assert!(!is_expired(&api(403)));
        assert!(!is_expired(&AuthError::MfaRequired.into()));
    }

    #[test]
    fn name() {
        assert_eq!(
            display_name(&user("user")),
            "MCP agent <mcp-agent@example.com>"
        );
    }
}
