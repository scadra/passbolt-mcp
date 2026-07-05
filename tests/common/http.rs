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

//! The fake behind HTTP: what `NativeHttpClient` sends to `passbolt-mcp`'s server, the fake
//! answers, as if at `BASE_URL`.

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::Response;
use passbolt_sdk::HttpClient;

use super::{FakePassbolt, BASE_URL};

/// Serve `fake` on a free loopback port, until the test ends: its base URL.
pub async fn serve(fake: FakePassbolt) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().fallback(move |request: Request| {
        let fake = fake.clone();
        async move { answer(&fake, request).await }
    });
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}")
}

async fn answer(fake: &FakePassbolt, request: Request) -> Response<Body> {
    let method = request.method().as_str().to_string();
    let url = format!(
        "{BASE_URL}{}",
        request
            .uri()
            .path_and_query()
            .map_or("/", |path_and_query| path_and_query.as_str())
    );
    let headers: Vec<(String, String)> = request
        .headers()
        .iter()
        .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
        .collect();
    let headers: Vec<(&str, &str)> = headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let body = to_bytes(request.into_body(), usize::MAX).await.unwrap();
    let response = match (method.as_str(), body.is_empty()) {
        ("GET", _) => fake.get(&url, &headers).await,
        ("POST", _) => fake.post(&url, &headers, &body).await,
        ("PUT", _) => fake.put(&url, &headers, &body).await,
        ("DELETE", true) => fake.delete(&url, &headers).await,
        ("DELETE", false) => fake.delete_with_body(&url, &headers, &body).await,
        (method, _) => panic!("{method} {url}"),
    }
    .unwrap();
    let mut builder = Response::builder().status(response.status);
    for (name, value) in &response.headers {
        // The fake hands its cookie as NativeHttpClient exposes it.
        if name == "set-cookie-name" {
            builder = builder.header("set-cookie", format!("{value}; Path=/"));
        } else {
            builder = builder.header(name.as_str(), value.as_str());
        }
    }
    builder.body(Body::from(response.body)).unwrap()
}
