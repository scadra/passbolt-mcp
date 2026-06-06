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

//! The server key, pinned on first use by `passbolt-mcp verify`: at each login, the server must
//! prove it holds the private key, so a server pretending to be yours gets nothing.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use passbolt_sdk::auth::fetch_server_key;
use passbolt_sdk::{NativeHttpClient, NativeHttpConfig, OpenPgp};

use crate::config::{Config, PassboltConfig};

/// The HTTP client of every call to the server.
pub fn http_client(config: &PassboltConfig) -> Result<NativeHttpClient> {
    if config.tls_skip_verify {
        tracing::warn!("tls_skip_verify: the certificate of the server is not checked");
    }
    Ok(NativeHttpClient::new(&NativeHttpConfig {
        tls_skip_verify: config.tls_skip_verify,
        client_cert_pem: None,
        client_key_pem: None,
        proxy_url: None,
        resolve: config
            .resolve
            .iter()
            .map(|entry| parse_resolve(entry))
            .collect::<Result<_>>()?,
        timeout: Some(Duration::from_secs(60)),
    })?)
}

fn parse_resolve(entry: &str) -> Result<(String, SocketAddr)> {
    let mut parts = entry.splitn(3, ':');
    let (Some(host), Some(port), Some(address)) = (parts.next(), parts.next(), parts.next()) else {
        bail!("resolve: {entry} is not host:port:address");
    };
    let address = address.trim_start_matches('[').trim_end_matches(']');
    let address: IpAddr = address
        .parse()
        .with_context(|| format!("resolve: {address} is not an IP address"))?;
    let port = port
        .parse()
        .with_context(|| format!("resolve: {port} is not a port"))?;
    Ok((host.to_string(), SocketAddr::new(address, port)))
}

/// What `verify` found.
#[derive(Debug, PartialEq, Eq)]
pub enum Pinned {
    /// Pinned now, with this fingerprint.
    New(String),
    /// Already pinned, same key.
    Same(String),
}

/// Fetch the server key, and pin it unless one is pinned already. A key different from the
/// pinned one is an error: the server, or a server pretending to be it, changed its identity.
pub async fn verify(config: &Config) -> Result<Pinned> {
    let base_url = config.passbolt.server_url.trim_end_matches('/');
    if base_url.is_empty() {
        bail!("no server: set server_url in [passbolt]");
    }
    let openpgp = OpenPgp::default();
    let (fingerprint, armored_key) =
        fetch_server_key(&http_client(&config.passbolt)?, base_url, &openpgp).await?;
    let path = config.server_key_path();
    let pinned = match std::fs::read_to_string(&path) {
        Ok(pinned) => Some(openpgp.public_key_fingerprint(&pinned)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).context(format!("cannot read {}", path.display())),
    };
    let result = compare(pinned, fingerprint)?;
    if let Pinned::New(_) = result {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, armored_key)
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    Ok(result)
}

fn compare(pinned: Option<String>, served: String) -> Result<Pinned> {
    match pinned {
        None => Ok(Pinned::New(served)),
        Some(pinned) if pinned.eq_ignore_ascii_case(&served) => Ok(Pinned::Same(served)),
        Some(pinned) => bail!(
            "the server key changed: {pinned} is pinned, the server sends {served}. If the \
             server really changed its key, delete the pinned key and run verify again"
        ),
    }
}

/// The pinned key, for the login.
pub fn read(config: &Config) -> Result<String> {
    let path = config.server_key_path();
    std::fs::read_to_string(&path).with_context(|| {
        format!(
            "the server is not verified ({} is missing): run passbolt-mcp verify",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_use() {
        assert_eq!(
            compare(None, "AB12".into()).unwrap(),
            Pinned::New("AB12".into())
        );
    }

    #[test]
    fn same_key() {
        assert_eq!(
            compare(Some("ab12".into()), "AB12".into()).unwrap(),
            Pinned::Same("AB12".into())
        );
    }

    #[test]
    fn changed_key() {
        assert!(compare(Some("AB12".into()), "CD34".into()).is_err());
    }

    #[test]
    fn resolve() {
        assert_eq!(
            parse_resolve("passbolt.local:443:172.17.0.1").unwrap(),
            ("passbolt.local".into(), "172.17.0.1:443".parse().unwrap())
        );
        assert_eq!(
            parse_resolve("passbolt.local:8443:[::1]").unwrap().1,
            "[::1]:8443".parse().unwrap()
        );
        assert!(parse_resolve("passbolt.local:443").is_err());
    }

    #[test]
    fn not_verified() {
        let config = Config {
            dir: "/nonexistent".into(),
            ..Config::default()
        };
        assert!(read(&config)
            .unwrap_err()
            .to_string()
            .contains("run passbolt-mcp verify"));
    }
}
