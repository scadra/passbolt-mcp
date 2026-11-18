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

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use zeroize::Zeroizing;

use crate::commands::{env_value, EnvValue, SECRET};
use crate::error::ToolError;
use crate::vault::scrub;

const BODY_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    pub base_url: String,
    pub secret: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default = "default_methods")]
    pub methods: Vec<String>,
    pub description: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_methods() -> Vec<String> {
    vec!["GET".into()]
}

const fn default_timeout() -> u64 {
    30
}

impl ApiConfig {
    pub fn check(&self, name: &str) -> Result<()> {
        let base = self.base()?;
        let loopback = base
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
        ensure!(
            base.scheme() == "https" || (base.scheme() == "http" && loopback),
            "apis.{name}.base_url: https only, or http on a loopback address"
        );
        ensure!(
            base.username().is_empty() && base.password().is_none(),
            "apis.{name}.base_url: no credentials in the URL; put them in Passbolt"
        );
        ensure!(
            self.headers.values().any(|value| value.contains(SECRET)),
            "apis.{name}.headers: put {SECRET} in a header, where the secret goes"
        );
        if let Some(secret) = &self.secret {
            match env_value(secret) {
                EnvValue::Resource(id) if !id.is_empty() => {}
                _ => bail!("apis.{name}.secret: passbolt:<resource id>"),
            }
        }
        for method in &self.methods {
            ensure!(
                matches!(
                    method.as_str(),
                    "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE"
                ),
                "apis.{name}.methods: {method} is no HTTP method"
            );
        }
        Ok(())
    }

    fn base(&self) -> Result<Url> {
        let mut base = Url::parse(&self.base_url).context("base_url")?;
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        Ok(base)
    }

    /// The URL of `path` under `base_url`, never outside it.
    pub fn url(&self, path: &str, query: &BTreeMap<String, String>) -> Result<Url, ToolError> {
        let base = self
            .base()
            .map_err(|error| ToolError::invalid_argument(error.to_string()))?;
        let outside =
            || ToolError::invalid_argument(format!("{path}: not a path under {}", self.base_url));
        if path.starts_with('/') || path.contains("://") || path.contains('\\') {
            return Err(outside());
        }
        let mut url = base.join(path).map_err(|_| outside())?;
        if url.origin() != base.origin() || !url.path().starts_with(base.path()) {
            return Err(outside());
        }
        url.set_fragment(None);
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        Ok(url)
    }

    #[must_use]
    pub const fn takes_secret(&self) -> bool {
        self.secret.is_none()
    }
}

pub fn check_all(apis: &BTreeMap<String, ApiConfig>) -> Result<()> {
    for (name, api) in apis {
        api.check(name)?;
    }
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`call_api`.")]
pub struct CallApi {
    #[schemars(description = "The name of an API of the configuration, as `whoami` lists it.")]
    pub api: String,
    #[schemars(description = "`GET` by default; one of the methods the API allows.")]
    pub method: Option<String>,
    #[schemars(
        description = "The path under the base URL of the API, such as `repos/acme/app/issues`."
    )]
    pub path: String,
    #[schemars(description = "Query parameters, by name.")]
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    #[schemars(description = "The body of the request, sent as JSON.")]
    pub body: Option<serde_json::Value>,
    #[schemars(
        description = "A handle from `get_secret` or `generate_password`, or the id of a resource, whose password \
        goes in the request; for the APIs that take one."
    )]
    pub secret: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "The answer of the API; the secret is replaced with `[secret]`.")]
pub struct ApiResponse {
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[schemars(description = "Where a redirect points: it is not followed.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub body: String,
    #[schemars(description = "The body was longer, and was cut.")]
    pub truncated: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ApiView {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub methods: Vec<String>,
    #[schemars(description = "It needs `secret`: a handle or a resource id.")]
    pub takes_secret: bool,
}

impl ApiView {
    #[must_use]
    pub fn new(name: &str, api: &ApiConfig) -> Self {
        Self {
            name: name.into(),
            base_url: api.base_url.clone(),
            description: api.description.clone(),
            methods: api.methods.clone(),
            takes_secret: api.takes_secret(),
        }
    }
}

pub async fn send(
    api: &ApiConfig,
    method: &str,
    url: Url,
    body: Option<&serde_json::Value>,
    secret: &Zeroizing<String>,
) -> Result<ApiResponse, ToolError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(api.timeout_seconds))
        .build()
        .map_err(|error| ToolError::invalid_argument(error.to_string()))?;
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|error| ToolError::invalid_argument(error.to_string()))?;
    let mut request = client.request(method, url);
    for (name, value) in &api.headers {
        let value = Zeroizing::new(value.replace(SECRET, secret));
        let mut header = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| ToolError::invalid_argument(format!("the header {name} is not valid")))?;
        header.set_sensitive(true);
        request = request.header(name.as_str(), header);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.map_err(|error| {
        ToolError::invalid_argument(scrub(
            &format!("the API did not answer: {}", error.without_url()),
            std::slice::from_ref(secret),
        ))
    })?;
    let header = |name| {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(String::from)
    };
    let status = response.status().as_u16();
    let content_type = header(reqwest::header::CONTENT_TYPE);
    let location = header(reqwest::header::LOCATION);
    let bytes = response.bytes().await.map_err(|error| {
        ToolError::invalid_argument(format!("the API answer could not be read: {error}"))
    })?;
    let truncated = bytes.len() > BODY_LIMIT;
    let text =
        Zeroizing::new(String::from_utf8_lossy(&bytes[..bytes.len().min(BODY_LIMIT)]).into_owned());
    let secrets = std::slice::from_ref(secret);
    Ok(ApiResponse {
        status,
        content_type,
        location: location.map(|location| scrub(&location, secrets)),
        body: scrub(&text, secrets),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(base_url: &str) -> ApiConfig {
        ApiConfig {
            base_url: base_url.into(),
            secret: None,
            headers: BTreeMap::from([("Authorization".into(), "Bearer {secret}".into())]),
            methods: default_methods(),
            description: None,
            timeout_seconds: 5,
        }
    }

    #[test]
    fn checked() {
        assert!(api("https://api.github.com").check("gh").is_ok());
        assert!(api("http://127.0.0.1:8080/v1").check("local").is_ok());
        assert!(api("http://api.example.com").check("plain").is_err());
        assert!(api("https://u:p@api.example.com").check("creds").is_err());
        let mut no_header = api("https://api.example.com");
        no_header.headers.clear();
        assert!(no_header.check("x").is_err());
        let mut bad = api("https://api.example.com");
        bad.secret = Some("hunter2".into());
        assert!(bad.check("x").is_err());
        bad.secret = Some("passbolt:r1".into());
        assert!(bad.check("x").is_ok());
        bad.methods = vec!["TRACE".into()];
        assert!(bad.check("x").is_err());
    }

    #[test]
    fn urls_stay_under_the_base() {
        let gh = api("https://api.example.com/v1");
        let none = BTreeMap::new();
        assert_eq!(
            gh.url("repos/acme", &none).unwrap().as_str(),
            "https://api.example.com/v1/repos/acme"
        );
        let query = BTreeMap::from([("state".to_string(), "open&x=1".to_string())]);
        assert_eq!(
            gh.url("issues", &query).unwrap().as_str(),
            "https://api.example.com/v1/issues?state=open%26x%3D1"
        );
        for path in [
            "/admin",
            "../admin",
            "a/../../admin",
            "//evil.example.com/x",
            "https://evil.example.com",
            "%2e%2e/admin",
            "..\\admin",
        ] {
            match gh.url(path, &none) {
                Ok(url) => assert!(
                    url.as_str().starts_with("https://api.example.com/v1/"),
                    "{path}: {url}"
                ),
                Err(error) => assert!(error.message.contains("not a path under")),
            }
        }
    }
}
