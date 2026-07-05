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

//! `passbolt-mcp` against the fake Passbolt of the SDK tests: no server needed, run by the CI.

mod common;

use std::path::PathBuf;

use common::{data, http, FakePassbolt, ADA, BETTY, RESOURCE_ID, SERVER};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use serde_json::{json, Value};

/// A configuration directory for ada on the fake at `base_url`, with `extra` appended.
struct Setup {
    dir: tempfile::TempDir,
}

impl Setup {
    fn new(base_url: &str, pinned: bool, extra: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ada.asc"), ADA.private).unwrap();
        std::fs::write(dir.path().join("passphrase"), ADA.passphrase).unwrap();
        if pinned {
            std::fs::write(dir.path().join("server-key.asc"), SERVER.public).unwrap();
        }
        std::fs::write(
            dir.path().join("passbolt-mcp.toml"),
            format!(
                "[passbolt]\nserver_url = \"{base_url}\"\nprivate_key_file = \"ada.asc\"\n\
                 passphrase_file = \"passphrase\"\n{extra}"
            ),
        )
        .unwrap();
        Self { dir }
    }

    fn config(&self) -> PathBuf {
        self.dir.path().join("passbolt-mcp.toml")
    }

    /// Run `passbolt-mcp <args>`: success, stdout, stderr.
    async fn run(&self, args: &[&str]) -> (bool, String, String) {
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp"))
            .arg("--config")
            .arg(self.config())
            .args(args)
            .output()
            .await
            .unwrap();
        (
            output.status.success(),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

/// Ada, with the role user: the fixture makes her an administrator.
fn ada_as_user() -> Value {
    let mut ada = data::user("ada");
    ada["role"]["name"] = "user".into();
    ada
}

/// The fake Passbolt, ada a user who sees one resource and no folder, on HTTP: its URL.
async fn fake() -> (FakePassbolt, String) {
    let fake = FakePassbolt::new();
    fake.with_directory();
    fake.on("GET", "/users/me.json", ada_as_user());
    fake.on(
        "GET",
        "/resources.json",
        data::listed_resources(&[RESOURCE_ID]),
    );
    fake.on("GET", "/folders.json", json!([]));
    fake.on(
        "GET",
        &format!("/resources/{RESOURCE_ID}.json"),
        data::resource_v4(
            "password-and-description",
            r#"{"password":"s3cret-fake","description":"d"}"#,
        ),
    );
    let url = http::serve(fake.clone()).await;
    (fake, url)
}

#[tokio::test(flavor = "multi_thread")]
async fn check() {
    let (_fake, url) = fake().await;
    let setup = Setup::new(&url, true, "");
    let (ok, stdout, stderr) = setup.run(&["check"]).await;
    assert!(ok, "{stderr}");
    assert!(
        stdout.contains("Logged in as Ada Test <ada@passbolt.test>, role user"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Sees 1 resources and 0 folders"),
        "{stdout}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refuses_an_administrator() {
    let (fake, url) = fake().await;
    fake.on("GET", "/users/me.json", data::user("ada"));
    let (ok, _, stderr) = Setup::new(&url, true, "").run(&["check"]).await;
    assert!(!ok);
    assert!(stderr.contains("is an administrator"), "{stderr}");
    assert_eq!(
        fake.requests_to("POST", "/auth/logout.json").len(),
        1,
        "logged out"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn verify_pins_the_server_key() {
    let (fake, url) = fake().await;
    let setup = Setup::new(&url, false, "");
    let (ok, _, stderr) = setup.run(&["check"]).await;
    assert!(!ok);
    assert!(stderr.contains("run passbolt-mcp verify"), "{stderr}");
    let (ok, stdout, _) = setup.run(&["verify"]).await;
    assert!(ok);
    assert_eq!(
        stdout,
        format!("Pinned the server key {}\n", SERVER.fingerprint())
    );
    let (_, stdout, _) = setup.run(&["verify"]).await;
    assert!(stdout.contains("is pinned"));
    // The fake serves one client: a new session for the next process.
    fake.rotate_csrf_token("csrf-second-session");
    let (ok, _, stderr) = setup.run(&["check"]).await;
    assert!(ok, "{stderr}");
}

#[tokio::test(flavor = "multi_thread")]
async fn refuses_an_impostor() {
    let (fake, url) = fake().await;
    // It serves the real server key, but holds another.
    fake.impersonated_with(&BETTY);
    let (ok, _, stderr) = Setup::new(&url, true, "").run(&["check"]).await;
    assert!(!ok);
    assert!(stderr.contains("Server identity mismatch"), "{stderr}");
}

async fn client(setup: &Setup) -> RunningService<RoleClient, ()> {
    let config = setup.config();
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command.arg("--config").arg(&config).arg("serve");
        });
    ().serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}

async fn call(client: &RunningService<RoleClient, ()>, tool: &'static str, args: Value) -> Value {
    let result = client
        .call_tool(
            CallToolRequestParams::new(tool).with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    let content = result.structured_content.unwrap_or_default();
    assert_eq!(result.is_error, Some(false), "{tool}: {content}");
    content
}

#[tokio::test(flavor = "multi_thread")]
async fn tools() {
    let (_fake, url) = fake().await;
    let setup = Setup::new(&url, true, "[access]\nsecrets = true\n");
    let client = client(&setup).await;
    assert_eq!(call(&client, "whoami", json!({})).await["role"], "user");
    let page = call(&client, "list_resources", json!({})).await;
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["id"], RESOURCE_ID);
    let resource = call(&client, "get_resource", json!({ "id": RESOURCE_ID })).await;
    assert_eq!(resource["id"], RESOURCE_ID);
    let secret = call(&client, "get_secret", json!({ "id": RESOURCE_ID })).await;
    assert_eq!(secret["value"], "s3cret-fake");
    client.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn read_only_by_default() {
    let (_fake, url) = fake().await;
    let setup = Setup::new(&url, true, "");
    let client = client(&setup).await;
    let tools = client.list_all_tools().await.unwrap();
    assert!(tools.iter().any(|tool| tool.name == "list_resources"));
    assert!(!tools.iter().any(|tool| tool.name == "get_secret"));
    assert!(!tools.iter().any(|tool| tool.name == "delete_resource"));
    client.cancel().await.unwrap();
}
