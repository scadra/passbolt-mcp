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
