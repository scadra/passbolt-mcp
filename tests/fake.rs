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
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, ElicitRequestParams,
    ElicitResult, ElicitationAction, ElicitationCapability, FormElicitationCapability,
    Implementation, ProtocolVersion,
};
use rmcp::service::{ClientLifecycleMode, ClientServiceExt, RequestContext, RunningService};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{ClientHandler, ErrorData, RoleClient};
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
    client_with(setup, ()).await
}

async fn client_with<H: ClientHandler>(setup: &Setup, handler: H) -> RunningService<RoleClient, H> {
    client_on(setup, handler, ClientLifecycleMode::Initialize).await
}

async fn client_on<H: ClientHandler>(
    setup: &Setup,
    handler: H,
    lifecycle: ClientLifecycleMode,
) -> RunningService<RoleClient, H> {
    let config = setup.config();
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command.arg("--config").arg(&config).arg("serve");
        });
    handler
        .serve_with_lifecycle(TokioChildProcess::new(command).unwrap(), lifecycle)
        .await
        .unwrap()
}

async fn call_tool<H: ClientHandler>(
    client: &RunningService<RoleClient, H>,
    tool: &'static str,
    args: Value,
) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(tool).with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}

async fn call<H: ClientHandler>(
    client: &RunningService<RoleClient, H>,
    tool: &'static str,
    args: Value,
) -> Value {
    let result = call_tool(client, tool, args).await;
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

/// A client whose person answers the confirmations of `[confirm]`; `None`: it cannot ask.
struct Person {
    says: Option<ElicitationAction>,
    protocol: ProtocolVersion,
}

impl ClientHandler for Person {
    fn create_elicitation(
        &self,
        request: ElicitRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<ElicitResult, ErrorData>> + Send + '_ {
        let ElicitRequestParams::FormElicitationParams { message, .. } = request else {
            panic!("a form");
        };
        assert!(message.contains("get_secret"), "{message}");
        std::future::ready(Ok(ElicitResult::new(self.says.clone().unwrap())))
    }

    fn get_info(&self) -> ClientConfig {
        let mut capabilities = ClientCapabilities::default();
        if self.says.is_some() {
            capabilities.elicitation =
                Some(ElicitationCapability::new().with_form(FormElicitationCapability::default()));
        }
        ClientConfig::new(capabilities, Implementation::from_build_env())
            .with_protocol_version(self.protocol.clone())
    }
}

const CONFIRM_SECRETS: &str = "[access]\nsecrets = true\n[confirm]\ntools = [\"get_secret\"]\n";

/// `get_secret`, by a client of `protocol` whose person `says`: the secret or the error code, and
/// the outcomes of the audit file.
async fn confirmed(
    says: Option<ElicitationAction>,
    protocol: ProtocolVersion,
    extra: &str,
) -> (Result<String, String>, Vec<String>) {
    let (_fake, url) = fake().await;
    let audit = "[audit]\nfile = \"audit.jsonl\"\n";
    let setup = Setup::new(&url, true, &format!("{audit}{CONFIRM_SECRETS}{extra}"));
    // From 2026-07-28, no `initialize`: the client discovers the server.
    let lifecycle = if protocol.has_initialize() {
        ClientLifecycleMode::Initialize
    } else {
        ClientLifecycleMode::Discover {
            preferred_versions: vec![protocol.clone()],
        }
    };
    let client = client_on(&setup, Person { says, protocol }, lifecycle).await;
    let result = call_tool(&client, "get_secret", json!({ "id": RESOURCE_ID })).await;
    client.cancel().await.unwrap();
    let content = result.structured_content.unwrap_or_default();
    let outcome = if result.is_error == Some(true) {
        Err(content["code"].as_str().unwrap_or_default().to_string())
    } else {
        Ok(content["value"].as_str().unwrap_or_default().to_string())
    };
    let lines = std::fs::read_to_string(setup.dir.path().join("audit.jsonl")).unwrap();
    let outcomes = lines
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["outcome"].to_string())
        .map(|outcome| outcome.trim_matches('"').to_string())
        .collect();
    (outcome, outcomes)
}

#[tokio::test(flavor = "multi_thread")]
async fn confirm_with_input_required() {
    let accepted = confirmed(
        Some(ElicitationAction::Accept),
        ProtocolVersion::V_2026_07_28,
        "",
    );
    assert_eq!(
        accepted.await,
        (Ok("s3cret-fake".into()), vec!["asked".into(), "ok".into()])
    );
    let declined = confirmed(
        Some(ElicitationAction::Decline),
        ProtocolVersion::V_2026_07_28,
        "",
    );
    assert_eq!(
        declined.await,
        (
            Err("declined".into()),
            vec!["asked".into(), "declined".into()]
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn confirm_with_elicitation_requests() {
    let accepted = confirmed(
        Some(ElicitationAction::Accept),
        ProtocolVersion::V_2025_11_25,
        "",
    );
    assert_eq!(
        accepted.await,
        (Ok("s3cret-fake".into()), vec!["ok".into()])
    );
    let cancelled = confirmed(
        Some(ElicitationAction::Cancel),
        ProtocolVersion::V_2025_11_25,
        "",
    );
    assert_eq!(
        cancelled.await,
        (Err("declined".into()), vec!["declined".into()])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn confirm_without_elicitation() {
    let refused = confirmed(None, ProtocolVersion::V_2025_11_25, "");
    assert_eq!(refused.await.0, Err("unconfirmed".into()));
    let allowed = confirmed(
        None,
        ProtocolVersion::V_2025_11_25,
        "unsupported = \"allow\"\n",
    );
    assert_eq!(allowed.await.0, Ok("s3cret-fake".into()));
}
