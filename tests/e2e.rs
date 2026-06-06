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

//! Against a Passbolt server: `PASSBOLT_MCP_E2E_CONFIG=passbolt-mcp.toml cargo test -- --ignored`,
//! after `passbolt-mcp verify`.

use std::path::Path;

use passbolt_mcp::config::Config;
use passbolt_mcp::session::Session;

fn config() -> Config {
    let path = std::env::var_os("PASSBOLT_MCP_E2E_CONFIG").expect("PASSBOLT_MCP_E2E_CONFIG");
    Config::load(Path::new(&path)).unwrap()
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn login() {
    let session = Session::open(&config()).await.unwrap();
    assert!(session.me().await.unwrap().username.is_some());
}

#[tokio::test]
#[ignore = "needs a Passbolt server"]
async fn login_again_after_the_session_ends() {
    let session = Session::open(&config()).await.unwrap();
    let before = session.client().await;
    before.logout().await;
    session.me().await.unwrap();
    assert!(!std::sync::Arc::ptr_eq(&before, &session.client().await));
}
