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

//! An MCP client talking to `passbolt-mcp` over stdio, against a Passbolt server:
//! `PASSBOLT_MCP_E2E_CONFIG=passbolt-mcp.toml cargo test -- --ignored`.

use std::path::Path;

use passbolt_mcp::config::Config;
use passbolt_mcp::session::Session;
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use serde_json::{json, Value};

fn config_path() -> String {
    std::env::var("PASSBOLT_MCP_E2E_CONFIG").expect("PASSBOLT_MCP_E2E_CONFIG")
}

/// Call `tool`, which must succeed.
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
    // Written in Passbolt, read through the MCP.
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

/// The e2e configuration with `[access]` added, next to it so that its relative paths hold.
struct ConfigWith(std::path::PathBuf);

impl ConfigWith {
    fn new(access: &str) -> Self {
        let original = Path::new(&config_path()).to_path_buf();
        let path = original.with_file_name(format!("e2e-{}.toml", std::process::id()));
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
    let config = ConfigWith::new("secrets = true");
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

    let generated = call(&client, "generate_password", json!({ "length": 24 })).await;
    let password = generated["password"].as_str().unwrap().to_string();
    assert_eq!(password.len(), 24);
    let page = call(&client, "list_resources", json!({ "limit": 50 })).await;
    let mut secret = None;
    for item in page["items"].as_array().unwrap() {
        let result = client
            .call_tool(
                CallToolRequestParams::new("get_secret")
                    .with_arguments(json!({ "id": item["id"] }).as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        if result.is_error == Some(false) {
            secret = Some(
                result.structured_content.unwrap()["value"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
            break;
        }
    }
    let secret = secret.expect("a resource with a password");
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

/// Call `tool`, which must fail: its error.
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

    // A dry run changes nothing.
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

/// The user to share with: `PASSBOLT_MCP_E2E_SHARE_WITH`, a username.
fn share_with() -> String {
    std::env::var("PASSBOLT_MCP_E2E_SHARE_WITH").expect("PASSBOLT_MCP_E2E_SHARE_WITH")
}

/// The ids of `aro` (`user` or `group`) in `get_permissions`.
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

    // On the folder and its content.
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

    // With access.share: the permissions of the folder.
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

    // Without: moving into a shared folder shares nothing.
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
