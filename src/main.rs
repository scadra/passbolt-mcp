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

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// An MCP server for Passbolt: an AI agent works with Passbolt through a user of its own.
#[derive(Parser)]
#[command(name = "passbolt-mcp", version, about)]
struct Cli {}

/// Logs go to stderr: in stdio, stdout carries the MCP messages.
fn init_logs() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("PASSBOLT_MCP_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();
}

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();
    init_logs();
}
