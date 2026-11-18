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

use std::path::Path;

use passbolt_mcp::config::Config;
use passbolt_mcp::session::Session;
use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, ClientConfig, ElicitRequestParams, ElicitResult,
    ElicitationAction, ElicitationCapability, FormElicitationCapability, Implementation,
    ProtocolVersion,
};
use rmcp::service::{ClientLifecycleMode, ClientServiceExt, RequestContext, RunningService};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{ClientHandler, ErrorData, RoleClient, ServiceExt};
use serde_json::{json, Value};

fn config_path() -> String {
    std::env::var("PASSBOLT_MCP_E2E_CONFIG").expect("PASSBOLT_MCP_E2E_CONFIG")
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

async fn client() -> RunningService<RoleClient, ()> {
    let config = config_path();
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command.args(["--config", &config, "serve"]);
        });
    ().serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn tools() {
    let client = client().await;
    let info = client.peer_info().unwrap();
    assert_eq!(info.server_info.as_ref().unwrap().name, "passbolt-mcp");
    let tools = client.list_all_tools().await.unwrap();
    assert!(tools.iter().any(|tool| tool.name == "whoami"));
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn whoami() {
    let client = client().await;
    let result = client
        .call_tool(CallToolRequestParams::new("whoami"))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let whoami = result.structured_content.unwrap();
    assert_eq!(whoami["role"], "user");
    assert_eq!(whoami["write"], false);
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn unknown_tool() {
    let client = client().await;
    assert!(client
        .call_tool(CallToolRequestParams::new("delete_everything"))
        .await
        .is_err());
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn read_tools() {
    let client = client().await;
    let page = call(&client, "list_resources", json!({ "limit": 1 })).await;
    assert!(page["total"].as_u64().unwrap() >= 1);
    let id = page["items"][0]["id"].clone();
    let resource = call(&client, "get_resource", json!({ "id": id })).await;
    assert_eq!(resource["id"], id);
    assert!(resource.get("password").is_none() && resource.get("secret").is_none());
    let permissions = call(&client, "get_permissions", json!({ "resource": id })).await;
    assert_ne!(permissions["permissions"].as_array().unwrap().len(), 0);
    call(&client, "list_folders", json!({})).await;
    call(&client, "folder_tree", json!({})).await;
    let types = call(&client, "list_resource_types", json!({})).await;
    assert_ne!(types["resource_types"].as_array().unwrap().len(), 0);
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn comments() {
    let client = client().await;
    let page = call(&client, "list_resources", json!({ "limit": 1 })).await;
    let id = page["items"][0]["id"].as_str().unwrap().to_string();
    let session = Session::open(&Config::load(Path::new(&config_path())).unwrap())
        .await
        .unwrap();
    let comment = session
        .client()
        .await
        .create_comment(&id, "written by the e2e tests", None)
        .await
        .unwrap();
    let comments = call(&client, "list_comments", json!({ "id": id })).await;
    let found = comments["comments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|found| found["id"] == comment.id.as_str())
        .cloned();
    session
        .client()
        .await
        .delete_comment(&comment.id)
        .await
        .unwrap();
    session.close().await;
    let found = found.expect("the comment");
    assert_eq!(found["content"], "written by the e2e tests");
    assert!(found["author"].as_str().unwrap().contains('@'));
    client.cancel().await.unwrap();
}

struct ConfigWith(std::path::PathBuf);

static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl ConfigWith {
    fn new(access: &str) -> Self {
        let original = Path::new(&config_path()).to_path_buf();
        let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = original.with_file_name(format!("e2e-{}-{next}.toml", std::process::id()));
        let text = std::fs::read_to_string(&original).unwrap();
        std::fs::write(&path, format!("{text}\n[access]\n{access}\n")).unwrap();
        Self(path)
    }
}

impl Drop for ConfigWith {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn no_secrets_by_default() {
    let client = client().await;
    let tools = client.list_all_tools().await.unwrap();
    for tool in [
        "get_secret",
        "get_totp_code",
        "check_password_pwned",
        "generate_password",
    ] {
        assert!(!tools.iter().any(|found| found.name == tool), "{tool}");
    }
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn secrets_stay_out_of_the_logs() {
    let config = ConfigWith::new("secrets = true\nwrite = true");
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command
                .args(["--config", config.0.to_str().unwrap(), "serve"])
                .env("PASSBOLT_MCP_LOG", "trace");
        });
    let (transport, stderr) = TokioChildProcess::builder(command)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = stderr.unwrap();
    let logs = tokio::spawn(async move {
        let mut logs = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut stderr, &mut logs)
            .await
            .unwrap();
        logs
    });
    let client = ().serve(transport).await.unwrap();

    let generated = call(
        &client,
        "generate_password",
        json!({ "kind": "password", "length": 24 }),
    )
    .await;
    let password = generated["password"].as_str().unwrap().to_string();
    assert_eq!(password.len(), 24);
    let root = format!("mcp-e2e-logs-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let stored = "Logs-e2e-never-in-them-42";
    let id = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": stored, "folder": root }),
    )
    .await["id"]
        .clone();
    let secret = call(&client, "get_secret", json!({ "id": id })).await["value"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(secret, stored);
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();

    let logs = logs.await.unwrap();
    assert!(logs.lines().count() > 10, "trace logs");
    assert!(!logs.contains(&secret), "the secret is in the logs");
    assert!(
        !logs.contains(&password),
        "the generated password is in the logs"
    );
}

async fn client_with(config: &ConfigWith) -> RunningService<RoleClient, ()> {
    let path = config.0.to_str().unwrap().to_string();
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command.args(["--config", &path, "serve"]);
        });
    ().serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}

async fn refused(
    client: &RunningService<RoleClient, ()>,
    tool: &'static str,
    args: Value,
) -> Value {
    let result = client
        .call_tool(
            CallToolRequestParams::new(tool).with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true), "{tool} succeeded");
    result.structured_content.unwrap()
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn no_writes_by_default() {
    let client = client().await;
    let tools = client.list_all_tools().await.unwrap();
    for tool in [
        "create_resource",
        "update_resource",
        "delete_resource",
        "delete_folder",
    ] {
        assert!(!tools.iter().any(|found| found.name == tool), "{tool}");
    }
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn writes() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-{}", std::process::id());
    let sub = format!("{root}/sub");

    let folder = call(&client, "create_folder", json!({ "name": root })).await;
    assert!(folder["id"].is_string());
    call(
        &client,
        "create_folder",
        json!({ "name": "sub", "parent": root }),
    )
    .await;
    let created = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "username": "agent", "password": "Pass-e2e-1", "folder": root }),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let dry = call(
        &client,
        "delete_resource",
        json!({ "id": id, "dry_run": true }),
    )
    .await;
    assert_eq!(dry["dry_run"], true);
    call(&client, "get_resource", json!({ "id": id })).await;

    call(
        &client,
        "update_resource",
        json!({ "id": id, "username": "agent-2" }),
    )
    .await;
    assert_eq!(
        call(&client, "get_resource", json!({ "id": id })).await["username"],
        "agent-2"
    );
    call(&client, "move_resource", json!({ "id": id, "folder": sub })).await;
    assert_eq!(
        call(&client, "get_resource", json!({ "id": id })).await["folder"],
        sub.as_str()
    );

    call(
        &client,
        "set_favorite",
        json!({ "id": id, "favorite": true }),
    )
    .await;
    assert_eq!(
        call(&client, "get_resource", json!({ "id": id })).await["favorite"],
        true
    );
    let comment = call(
        &client,
        "add_comment",
        json!({ "id": id, "content": "e2e" }),
    )
    .await;
    call(
        &client,
        "delete_comment",
        json!({ "id": id, "comment_id": comment["id"] }),
    )
    .await;

    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    assert_eq!(
        refused(&client, "get_resource", json!({ "id": id })).await["code"],
        "not_found"
    );
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn writes_stay_in_the_scope() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-scope-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    client.cancel().await.unwrap();

    let scoped = ConfigWith::new(&format!("write = true\n[scope]\nfolders = [\"{root}\"]"));
    let client = client_with(&scoped).await;
    let outside = refused(
        &client,
        "create_resource",
        json!({ "name": "outside", "password": "x-Y-1z" }),
    )
    .await;
    assert_eq!(outside["code"], "refused");
    assert_eq!(
        outside["message"],
        "refused by the MCP rules: outside scope.folders"
    );
    let inside = call(
        &client,
        "create_resource",
        json!({ "name": "inside", "password": "x-Y-1z", "folder": root }),
    )
    .await;
    assert!(inside["id"].is_string());
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

fn share_with() -> String {
    std::env::var("PASSBOLT_MCP_E2E_SHARE_WITH").expect("PASSBOLT_MCP_E2E_SHARE_WITH")
}

fn holders(permissions: &Value, aro: &str) -> Vec<(String, String)> {
    permissions["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|permission| permission["aro"] == aro)
        .map(|permission| {
            (
                permission["id"].as_str().unwrap().to_string(),
                permission["permission"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn sharing() {
    let config = ConfigWith::new("write = true\nshare = true");
    let client = client_with(&config).await;
    let username = share_with();
    let found = call(&client, "search_aros", json!({ "search": username })).await;
    let user = found["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aro| {
            aro["name"]
                .as_str()
                .unwrap()
                .contains(&format!("<{username}>"))
        })
        .expect("the user to share with")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let root = format!("mcp-e2e-share-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let created = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": "Sh4re-e2e", "folder": root }),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let simulation = call(
        &client,
        "simulate_share",
        json!({ "id": id, "users": [user], "permission": "read" }),
    )
    .await;
    let gaining = simulation["gaining_access"].as_array().unwrap();
    assert!(gaining
        .iter()
        .any(|name| name.as_str().unwrap().contains(&username)));
    let unchanged = call(&client, "get_permissions", json!({ "resource": id })).await;
    assert!(!holders(&unchanged, "user")
        .iter()
        .any(|(holder, _)| holder == &user));

    call(
        &client,
        "share_resource",
        json!({ "id": id, "users": [user], "permission": "read" }),
    )
    .await;
    let shared = call(&client, "get_permissions", json!({ "resource": id })).await;
    assert!(holders(&shared, "user").contains(&(user.clone(), "read".into())));
    call(
        &client,
        "share_resource",
        json!({ "id": id, "users": [user], "permission": "none" }),
    )
    .await;
    let revoked = call(&client, "get_permissions", json!({ "resource": id })).await;
    assert!(!holders(&revoked, "user")
        .iter()
        .any(|(holder, _)| holder == &user));

    call(
        &client,
        "share_folder",
        json!({ "folder": root, "users": [user], "permission": "update" }),
    )
    .await;
    let folder = call(&client, "get_permissions", json!({ "folder": root })).await;
    assert!(holders(&folder, "user").contains(&(user.clone(), "update".into())));
    let content = call(&client, "get_permissions", json!({ "resource": id })).await;
    assert!(holders(&content, "user").contains(&(user.clone(), "update".into())));

    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn moves_share_only_with_access_share() {
    let sharing = ConfigWith::new("write = true\nshare = true");
    let client = client_with(&sharing).await;
    let username = share_with();
    let found = call(&client, "search_aros", json!({ "search": username })).await;
    let user = found["items"][0]["id"].as_str().unwrap().to_string();
    let root = format!("mcp-e2e-move-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    call(
        &client,
        "share_folder",
        json!({ "folder": root, "users": [user], "permission": "read" }),
    )
    .await;
    let make = |name: &str| json!({ "name": format!("{root}-{name}"), "password": "Mv-e2e-1" });
    let shared = call(&client, "create_resource", make("shared")).await["id"].clone();
    let kept = call(&client, "create_resource", make("kept")).await["id"].clone();

    let moved = call(
        &client,
        "move_resource",
        json!({ "id": shared, "folder": root }),
    )
    .await;
    assert!(moved["action"]
        .as_str()
        .unwrap()
        .ends_with("with the permissions of the folder"));
    let permissions = call(&client, "get_permissions", json!({ "resource": shared })).await;
    assert!(holders(&permissions, "user")
        .iter()
        .any(|(holder, _)| holder == &user));
    client.cancel().await.unwrap();

    let writing = ConfigWith::new("write = true");
    let client = client_with(&writing).await;
    let moved = call(
        &client,
        "move_resource",
        json!({ "id": kept, "folder": root }),
    )
    .await;
    assert!(moved["action"]
        .as_str()
        .unwrap()
        .ends_with("its permissions unchanged"));
    let permissions = call(&client, "get_permissions", json!({ "resource": kept })).await;
    assert!(!holders(&permissions, "user")
        .iter()
        .any(|(holder, _)| holder == &user));
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn audit_without_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("audit.jsonl");
    let config = ConfigWith::new(&format!(
        "write = true\nsecrets = true\n[audit]\nfile = \"{}\"",
        audit.display()
    ));
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-audit-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let password = "Audit-me-NOT-7";
    let created = call(
        &client,
        "create_resource",
        json!({ "name": "audited", "password": password, "note": "a secret note", "folder": root }),
    )
    .await;
    let id = created["id"].clone();
    let secret = call(&client, "get_secret", json!({ "id": id })).await;
    assert_eq!(secret["value"], password);
    refused(&client, "get_secret", json!({ "id": id, "field": "pin" })).await;
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();

    let lines = std::fs::read_to_string(&audit).unwrap();
    let entries: Vec<Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tools: Vec<&str> = entries
        .iter()
        .map(|entry| entry["tool"].as_str().unwrap())
        .collect();
    assert_eq!(
        tools,
        [
            "create_folder",
            "create_resource",
            "get_secret",
            "get_secret",
            "delete_folder"
        ]
    );
    assert_eq!(entries[2]["arguments"]["id"], id);
    assert_eq!(entries[3]["outcome"], "error");
    for secret in [password, "a secret note", "audited"] {
        assert!(!lines.contains(secret), "{secret} is in the audit file");
    }
}

async fn read(client: &RunningService<RoleClient, ()>, uri: &str) -> Value {
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new(uri))
        .await
        .unwrap();
    let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &result.contents[0]
    else {
        panic!("not text");
    };
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn resources() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-resources-{}", std::process::id());
    let folder = call(&client, "create_folder", json!({ "name": root })).await["id"].clone();
    let created = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": "Res-e2e-1", "folder": root }),
    )
    .await;
    let folder_uri = format!("passbolt://folder/{}", folder.as_str().unwrap());

    let listed = client.list_all_resources().await.unwrap();
    assert!(listed.iter().any(|resource| resource.uri == folder_uri));
    let contents = read(&client, &folder_uri).await;
    assert_eq!(contents["folder"]["path"], root.as_str());
    assert_eq!(contents["resources"][0]["id"], created["id"]);
    let resource = read(
        &client,
        &format!("passbolt://resource/{}", created["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(resource["name"], format!("{root}-db"));
    assert!(resource.get("password").is_none());
    assert!(client
        .read_resource(rmcp::model::ReadResourceRequestParams::new(
            "https://example.com"
        ))
        .await
        .is_err());

    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn prompts() {
    let client = client().await;
    let names: Vec<String> = client
        .list_all_prompts()
        .await
        .unwrap()
        .into_iter()
        .map(|prompt| prompt.name)
        .collect();
    assert_eq!(names, ["audit_password_hygiene"]);
    let prompt = client
        .get_prompt(rmcp::model::GetPromptRequestParams::new(
            "audit_password_hygiene",
        ))
        .await
        .unwrap();
    let text = serde_json::to_string(&prompt.messages).unwrap();
    assert!(text.contains("list_resources"));
    assert!(text.contains("access.secrets is false"));
    client.cancel().await.unwrap();

    let config = ConfigWith::new("write = true\nsecrets = true");
    let client = client_with(&config).await;
    let prompts = client.list_all_prompts().await.unwrap();
    assert!(prompts
        .iter()
        .any(|prompt| prompt.name == "rotate_password"));
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn secret_tools() {
    let config = ConfigWith::new("write = true\nsecrets = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-secrets-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let created = call(
        &client,
        "create_resource",
        json!({
            "name": format!("{root}-weak"),
            "username": "agent",
            "password": "password",
            "note": "the secret note",
            "description": "not secret",
            "uris": ["https://a.example.com", "https://b.example.com"],
            "expiry": "2031-01-01T00:00:00Z",
            "folder": root,
        }),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();
    let resource = call(&client, "get_resource", json!({ "id": id })).await;
    assert_eq!(resource["description"], "not secret");
    assert_eq!(
        resource["uris"],
        json!(["https://a.example.com", "https://b.example.com"])
    );
    assert!(resource["expiry"]
        .as_str()
        .unwrap()
        .starts_with("2031-01-01"));

    let secret = call(&client, "get_secret", json!({ "id": id })).await;
    assert_eq!(secret["value"], "password");
    let note = call(
        &client,
        "get_secret",
        json!({ "id": id, "field": "description" }),
    )
    .await;
    assert_eq!(note["value"], "the secret note");
    let pwned = call(&client, "check_password_pwned", json!({ "id": id })).await;
    assert_eq!(pwned["pwned"], true);
    assert!(pwned["breaches"].as_u64().unwrap() > 1000);

    let generated = call(
        &client,
        "generate_password",
        json!({ "kind": "password", "length": 32 }),
    )
    .await["password"]
        .as_str()
        .unwrap()
        .to_string();
    call(
        &client,
        "update_resource",
        json!({ "id": id, "password": generated, "clear_expiry": true }),
    )
    .await;
    assert_eq!(
        call(&client, "get_secret", json!({ "id": id })).await["value"],
        generated.as_str()
    );
    assert_eq!(
        call(
            &client,
            "get_secret",
            json!({ "id": id, "field": "description" })
        )
        .await["value"],
        "the secret note"
    );
    assert!(call(&client, "get_resource", json!({ "id": id })).await["expiry"].is_null());
    let safe = call(&client, "check_password_pwned", json!({ "id": id })).await;
    assert_eq!(safe["pwned"], false);

    let dry = call(
        &client,
        "delete_resource",
        json!({ "id": id, "dry_run": true }),
    )
    .await;
    assert_eq!(dry["dry_run"], true);
    call(&client, "delete_resource", json!({ "id": id })).await;
    assert_eq!(
        refused(&client, "get_resource", json!({ "id": id })).await["code"],
        "not_found"
    );
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn totp() {
    let config = ConfigWith::new("write = true\nsecrets = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-totp-{}", std::process::id());
    let folder = call(&client, "create_folder", json!({ "name": root })).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let session = Session::open(&Config::load(Path::new(&config_path())).unwrap())
        .await
        .unwrap();
    let totp = passbolt_sdk::resource::create::create_resource(
        &*session.client().await,
        &passbolt_sdk::resource::create::NewResource {
            name: format!("{root}-totp"),
            totp: Some(passbolt_sdk::types::TotpData {
                secret_key: Some("JBSWY3DPEHPK3PXP".into()),
                algorithm: Some("SHA1".into()),
                digits: Some(6),
                period: Some(30),
                ..Default::default()
            }),
            folder_parent_id: Some(folder),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    session.close().await;
    let code = call(&client, "get_totp_code", json!({ "id": totp.id })).await;
    let digits = code["code"].as_str().unwrap();
    assert!(digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_digit()));
    assert!((1..=30).contains(&code["seconds_remaining"].as_u64().unwrap()));
    assert!(!code.to_string().contains("JBSWY3DPEHPK3PXP"));
    let seed = refused(
        &client,
        "get_secret",
        json!({ "id": totp.id, "field": "totp" }),
    )
    .await;
    assert!(!seed.to_string().contains("JBSWY3DPEHPK3PXP"));

    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn policy_generator() {
    let session = Session::open(&Config::load(Path::new(&config_path())).unwrap())
        .await
        .unwrap();
    let policies = session
        .client()
        .await
        .get_password_policies()
        .await
        .unwrap();
    session.close().await;
    let config = ConfigWith::new("secrets = true");
    let client = client_with(&config).await;

    let default = call(&client, "generate_password", json!({})).await;
    assert_eq!(default["below_policy"], false);
    let password = default["password"].as_str().unwrap();
    match default["kind"].as_str().unwrap() {
        "password" => assert_eq!(
            password.chars().count(),
            policies.password_generator_settings.length as usize
        ),
        "passphrase" => assert_eq!(
            password
                .split(&policies.passphrase_generator_settings.word_separator)
                .count(),
            policies.passphrase_generator_settings.words as usize
        ),
        kind => panic!("{kind}"),
    }
    assert!(default["entropy_bits"].as_f64().unwrap() > 60.0);

    let short = call(
        &client,
        "generate_password",
        json!({ "kind": "password", "length": 8, "symbols": false }),
    )
    .await;
    assert_eq!(short["below_policy"], true);
    assert_eq!(short["strength"], "Very weak");
    assert!(short["password"]
        .as_str()
        .unwrap()
        .chars()
        .all(|c| c.is_ascii_alphanumeric()));
    let words = call(
        &client,
        "generate_password",
        json!({ "kind": "passphrase", "words": 12 }),
    )
    .await;
    assert_eq!(words["kind"], "passphrase");
    assert!(words["entropy_bits"].as_f64().unwrap() > 128.0);
    let pin = call(
        &client,
        "generate_password",
        json!({ "kind": "pin", "length": 8 }),
    )
    .await;
    assert_eq!(pin["password"].as_str().unwrap().len(), 8);
    let refused = refused(
        &client,
        "generate_password",
        json!({ "kind": "pin", "length": 20 }),
    )
    .await;
    assert!(refused["message"].as_str().unwrap().contains("4 to 12"));
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn masked_secrets() {
    let root = format!("mcp-e2e-masked-{}", std::process::id());
    let password = format!("Masked-e2e-{}-never-seen", std::process::id());
    let writer = client_with(&ConfigWith::new("write = true")).await;
    call(&writer, "create_folder", json!({ "name": root })).await;
    let id = call(
        &writer,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": password, "folder": root }),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    writer.cancel().await.unwrap();

    let config = ConfigWith::new(&format!(
        "write = true\nsecrets = true\nmask_secrets = true\n\
         [commands.show]\n\
         description = \"prints the password, as is and in base64\"\n\
         run = [\"sh\", \"-c\", \"echo \\\"$PW\\\"; printf %s \\\"$PW\\\" | base64; echo {{word}}\"]\n\
         env = {{ PW = \"{{secret}}\" }}\n\
         [commands.length]\n\
         run = [\"sh\", \"-c\", \"printf %s \\\"$PW\\\" | wc -c\"]\n\
         env = {{ PW = \"passbolt:{id}\" }}\n"
    ));
    let client = client_with(&config).await;
    let me = call(&client, "whoami", json!({})).await;
    assert_eq!(me["mask_secrets"], true);
    assert_eq!(me["commands"][0]["name"], "length");
    assert_eq!(me["commands"][1]["args"], json!(["word"]));
    assert_eq!(me["commands"][1]["takes_secret"], true);

    let secret = call(&client, "get_secret", json!({ "id": id })).await;
    assert!(secret["value"].is_null());
    let handle = secret["handle"].as_str().unwrap().to_string();
    assert!(handle.starts_with("passbolt:secret/"));

    for given in [handle.as_str(), id.as_str()] {
        let output = call(
            &client,
            "run_with_secret",
            json!({ "command": "show", "secret": given, "args": { "word": "hello" } }),
        )
        .await;
        assert_eq!(output["stdout"], "[secret]\n[secret]\nhello\n");
        assert_eq!(output["exit_code"], 0);
        assert!(!output.to_string().contains(&password));
    }
    let length = call(&client, "run_with_secret", json!({ "command": "length" })).await;
    assert_eq!(
        length["stdout"].as_str().unwrap().trim(),
        password.len().to_string()
    );

    let generated = call(
        &client,
        "generate_password",
        json!({ "kind": "password", "length": 31 }),
    )
    .await;
    assert!(generated["password"].is_null());
    call(
        &client,
        "update_resource",
        json!({ "id": id, "password": generated["handle"] }),
    )
    .await;
    let length = call(&client, "run_with_secret", json!({ "command": "length" })).await;
    assert_eq!(length["stdout"].as_str().unwrap().trim(), "31");

    let unknown = refused(
        &client,
        "run_with_secret",
        json!({ "command": "show", "secret": handle, "args": { "word": "x", "shell": "rm" } }),
    )
    .await;
    assert!(unknown["message"].as_str().unwrap().contains("shell"));
    let no_secret = refused(
        &client,
        "run_with_secret",
        json!({ "command": "length", "secret": handle }),
    )
    .await;
    assert!(no_secret["message"]
        .as_str()
        .unwrap()
        .contains("takes no secret"));
    let expired = refused(
        &client,
        "update_resource",
        json!({ "id": id, "password": "passbolt:secret/00" }),
    )
    .await;
    assert!(expired["message"].as_str().unwrap().contains("expired"));

    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn folders() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-folders-{}", std::process::id());
    let dry = call(
        &client,
        "create_folder",
        json!({ "name": root, "dry_run": true }),
    )
    .await;
    assert_eq!(dry["dry_run"], true);
    assert_eq!(
        call(&client, "list_folders", json!({ "search": root })).await["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    call(&client, "create_folder", json!({ "name": root })).await;
    call(
        &client,
        "create_folder",
        json!({ "name": "a", "parent": root }),
    )
    .await;
    call(
        &client,
        "create_folder",
        json!({ "name": "b", "parent": root }),
    )
    .await;
    let created = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-in-a"), "password": "F-e2e-1x", "folder": format!("{root}/a") }),
    )
    .await;
    let id = created["id"].clone();

    call(
        &client,
        "rename_folder",
        json!({ "folder": format!("{root}/a"), "name": "c" }),
    )
    .await;
    call(
        &client,
        "move_folder",
        json!({ "folder": format!("{root}/c"), "parent": format!("{root}/b") }),
    )
    .await;
    let resource = call(&client, "get_resource", json!({ "id": id })).await;
    assert_eq!(resource["folder"], format!("{root}/b/c"));
    let listed = call(
        &client,
        "list_resources",
        json!({ "folder": format!("{root}/b/c") }),
    )
    .await;
    assert_eq!(listed["items"][0]["id"], id);
    let found = call(
        &client,
        "list_resources",
        json!({ "search": format!("{root}-IN-A") }),
    )
    .await;
    assert_eq!(found["total"], 1);
    let tree = call(&client, "folder_tree", json!({})).await;
    let tree = tree.to_string();
    assert!(tree.contains(&root) && tree.contains("\"c\""));

    call(
        &client,
        "delete_folder",
        json!({ "folder": format!("{root}/b/c") }),
    )
    .await;
    let moved = call(&client, "get_resource", json!({ "id": id })).await;
    assert!(moved["folder"].is_null(), "{moved}");
    call(&client, "delete_resource", json!({ "id": id })).await;
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn allow_and_deny_tools() {
    let config = ConfigWith::new(
        "secrets = true\nallow_tools = [\"whoami\", \"list_resources\", \"get_secret\"]\n\
         deny_tools = [\"get_secret\"]",
    );
    let client = client_with(&config).await;
    let mut names: Vec<String> = client
        .list_all_tools()
        .await
        .unwrap()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["list_resources", "whoami"]);
    for hidden in ["get_secret", "list_folders"] {
        let error = client
            .call_tool(CallToolRequestParams::new(hidden))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("tool not found"),
            "{hidden}: {error}"
        );
    }
    call(&client, "list_resources", json!({ "limit": 1 })).await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn limits() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-limits-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let id = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": "Lim-e2e-1", "folder": root }),
    )
    .await["id"]
        .clone();
    client.cancel().await.unwrap();

    let limited =
        ConfigWith::new("secrets = true\n[limits]\ncalls_per_minute = 3\nsecrets_per_minute = 1");
    let client = client_with(&limited).await;
    call(&client, "get_secret", json!({ "id": id })).await;
    let secrets = refused(&client, "get_secret", json!({ "id": id })).await;
    assert_eq!(
        secrets["message"],
        "refused by the MCP rules: over limits.secrets_per_minute, try again in a minute"
    );
    call(&client, "whoami", json!({})).await;
    call(&client, "whoami", json!({})).await;
    let calls = refused(&client, "whoami", json!({})).await;
    assert_eq!(
        calls["message"],
        "refused by the MCP rules: over limits.calls_per_minute, try again in a minute"
    );
    client.cancel().await.unwrap();

    let client = client_with(&config).await;
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn scope_on_reads() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-reads-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let inside = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-in"), "password": "In-e2e-1", "folder": root }),
    )
    .await["id"]
        .clone();
    let outside = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-out"), "password": "Out-e2e-1" }),
    )
    .await["id"]
        .clone();
    client.cancel().await.unwrap();

    let scoped = ConfigWith::new(&format!("secrets = true\n[scope]\nfolders = [\"{root}\"]"));
    let client = client_with(&scoped).await;
    let whoami = call(&client, "whoami", json!({})).await;
    assert_eq!(whoami["scope_folders"], 1);
    let listed = call(&client, "list_resources", json!({ "limit": 200 })).await;
    let ids: Vec<&Value> = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| &item["id"])
        .collect();
    assert_eq!(ids, [&inside]);
    for tool in ["get_resource", "get_secret"] {
        let refusal = refused(&client, tool, json!({ "id": outside })).await;
        assert_eq!(refusal["code"], "refused", "{tool}");
    }
    assert_eq!(
        call(&client, "get_secret", json!({ "id": inside })).await["value"],
        "In-e2e-1"
    );
    let folders = call(&client, "list_folders", json!({})).await;
    assert_eq!(folders["items"].as_array().unwrap().len(), 1);
    client.cancel().await.unwrap();

    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    call(&client, "delete_resource", json!({ "id": outside })).await;
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

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
        assert!(!message.contains("Conf-e2e-1"), "{message}");
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

/// `get_secret` of `id` as a person who says `says`, or cannot be asked.
async fn ask(
    config: &ConfigWith,
    id: &Value,
    says: Option<ElicitationAction>,
    protocol: ProtocolVersion,
) -> Result<String, String> {
    let lifecycle = if protocol.has_initialize() {
        ClientLifecycleMode::Initialize
    } else {
        ClientLifecycleMode::Discover {
            preferred_versions: vec![protocol.clone()],
        }
    };
    let path = config.0.to_str().unwrap().to_string();
    let command =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp")).configure(|command| {
            command.args(["--config", &path, "serve"]);
        });
    let client = Person { says, protocol }
        .serve_with_lifecycle(TokioChildProcess::new(command).unwrap(), lifecycle)
        .await
        .unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_secret")
                .with_arguments(json!({ "id": id }).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    client.cancel().await.unwrap();
    let content = result.structured_content.unwrap_or_default();
    if result.is_error == Some(true) {
        Err(content["code"].as_str().unwrap().to_string())
    } else {
        Ok(content["value"].as_str().unwrap().to_string())
    }
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn confirm() {
    let config = ConfigWith::new("write = true");
    let client = client_with(&config).await;
    let root = format!("mcp-e2e-confirm-{}", std::process::id());
    call(&client, "create_folder", json!({ "name": root })).await;
    let id = call(
        &client,
        "create_resource",
        json!({ "name": format!("{root}-db"), "password": "Conf-e2e-1", "folder": root }),
    )
    .await["id"]
        .clone();
    client.cancel().await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("audit.jsonl");
    let confirming = ConfigWith::new(&format!(
        "secrets = true\n[audit]\nfile = \"{}\"\n[confirm]\ntools = [\"get_secret\"]",
        audit.display()
    ));
    let cases = [
        (
            Some(ElicitationAction::Accept),
            ProtocolVersion::V_2025_11_25,
            Ok("Conf-e2e-1"),
        ),
        (
            Some(ElicitationAction::Decline),
            ProtocolVersion::V_2025_11_25,
            Err("declined"),
        ),
        (
            Some(ElicitationAction::Accept),
            ProtocolVersion::V_2026_07_28,
            Ok("Conf-e2e-1"),
        ),
        (
            Some(ElicitationAction::Cancel),
            ProtocolVersion::V_2026_07_28,
            Err("declined"),
        ),
        (None, ProtocolVersion::V_2025_11_25, Err("unconfirmed")),
    ];
    for (says, protocol, expected) in cases {
        assert_eq!(
            ask(&confirming, &id, says.clone(), protocol.clone()).await,
            expected.map(String::from).map_err(String::from),
            "{says:?} on {protocol}"
        );
    }
    let outcomes: Vec<String> = std::fs::read_to_string(&audit)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["outcome"].to_string())
        .collect();
    assert_eq!(
        outcomes,
        [
            "\"ok\"",
            "\"declined\"",
            "\"asked\"",
            "\"ok\"",
            "\"asked\"",
            "\"declined\"",
            "\"declined\""
        ]
    );

    let client = client_with(&config).await;
    call(
        &client,
        "delete_folder",
        json!({ "folder": root, "with_content": true }),
    )
    .await;
    client.cancel().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn over_http() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let address = format!("127.0.0.1:{port}");
    let mut server = tokio::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp"))
        .args(["--config", &config_path(), "serve", "--http", &address])
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    for _ in 0..600 {
        if tokio::net::TcpStream::connect(&address).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let transport =
        rmcp::transport::StreamableHttpClientTransport::from_uri(format!("http://{address}/mcp"));
    let client = ().serve(transport).await.unwrap();
    let whoami = call(&client, "whoami", json!({})).await;
    assert_eq!(whoami["role"], "user");
    call(&client, "list_resources", json!({ "limit": 1 })).await;
    client.cancel().await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(&address).await.unwrap();
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\nOrigin: https://evil.example\r\n\
         Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut stream, &mut response)
        .await
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    server.kill().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn check_command() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_passbolt-mcp"))
        .args(["--config", &config_path(), "check"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("Logged in as") && text.contains("role user"),
        "{text}"
    );
    assert!(
        text.contains("Scope: everything the MCP user sees"),
        "{text}"
    );
}
