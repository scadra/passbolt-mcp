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

use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};

async fn client() -> RunningService<RoleClient, ()> {
    let config = std::env::var("PASSBOLT_MCP_E2E_CONFIG").expect("PASSBOLT_MCP_E2E_CONFIG");
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
