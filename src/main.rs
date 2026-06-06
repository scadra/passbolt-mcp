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

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use passbolt_mcp::config::Config;
use passbolt_mcp::server_key::{self, Pinned};
use passbolt_mcp::session::{self, Session};
use passbolt_sdk::{FolderQuery, ResourceQuery};
use tracing_subscriber::EnvFilter;

/// An MCP server for Passbolt: an AI agent works with Passbolt through a user of its own.
#[derive(Parser)]
#[command(name = "passbolt-mcp", version, about)]
struct Cli {
    /// The configuration file [default: passbolt-mcp.toml in the configuration directory]
    #[arg(long, env = "PASSBOLT_MCP_CONFIG", global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Log in as the MCP user and serve the agent (the default)
    Serve,
    /// Pin the server key: run it once, before the first login
    Verify,
    /// Log in as the MCP user and tell who it is and what it sees
    Check,
}

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
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logs();
    let config = Config::find(cli.config.as_deref())?;
    match cli.command {
        Some(Command::Verify) => match server_key::verify(&config).await? {
            Pinned::New(fingerprint) => println!("Pinned the server key {fingerprint}"),
            Pinned::Same(fingerprint) => println!("The server key {fingerprint} is pinned"),
        },
        Some(Command::Check) => check(&config).await?,
        None | Some(Command::Serve) => serve(&config).await?,
    }
    Ok(())
}

async fn check(config: &Config) -> anyhow::Result<()> {
    let session = Session::open(config).await?;
    let me = session.me().await?;
    let role = me.role.as_ref().and_then(|role| role.name.as_deref());
    println!(
        "Logged in as {}, role {}",
        session::display_name(&me),
        role.unwrap_or("unknown")
    );
    let resources = session
        .call(|client| async move { client.get_resources(ResourceQuery::default()).await })
        .await?;
    let folders = session
        .call(|client| async move { client.get_folders(FolderQuery::default()).await })
        .await?;
    println!(
        "Sees {} resources and {} folders",
        resources.len(),
        folders.len()
    );
    session.close().await;
    Ok(())
}

async fn serve(config: &Config) -> anyhow::Result<()> {
    let session = Arc::new(Session::open(config).await?);
    tracing::info!(
        "logged in as {}",
        session::display_name(&session.me().await?)
    );
    let keep_alive = session.keep_alive();
    shutdown().await;
    keep_alive.abort();
    session.close().await;
    tracing::info!("logged out");
    Ok(())
}

/// Ctrl-C, or SIGTERM from a service manager.
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
