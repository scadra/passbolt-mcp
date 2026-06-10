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
use passbolt_mcp::rules::Rules;
use passbolt_mcp::server::PassboltMcp;
use passbolt_mcp::server_key::{self, Pinned};
use passbolt_mcp::session::{self, Session};
use passbolt_sdk::{FolderQuery, ResourceQuery};
use rmcp::transport::stdio;
use rmcp::ServiceExt;
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
    let rules = Rules::load(config, &session).await?;
    print_rules(&rules, folders.len());
    let rights = rules.rights(resources.iter().map(|resource| {
        (
            resource.folder_parent_id.as_deref(),
            resource
                .permission
                .as_ref()
                .and_then(|permission| permission.permission_type)
                .unwrap_or_default(),
        )
    }));
    println!(
        "The agent can read {} resources, read the secret of {}, change {}, share {}",
        rights.read, rights.secrets, rights.change, rights.share
    );
    session.close().await;
    Ok(())
}

fn print_rules(rules: &Rules, folders: usize) {
    let access = rules.access();
    let yes_no = |allowed: bool, what: &str| {
        if allowed {
            what.to_string()
        } else {
            format!("no {what}")
        }
    };
    println!(
        "Rules: read, {}, {}, {}",
        yes_no(access.secrets, "secrets"),
        yes_no(access.write, "write"),
        yes_no(access.share, "share")
    );
    if !access.allow_tools.is_empty() {
        println!("Only the tools {}", access.allow_tools.join(", "));
    }
    if !access.deny_tools.is_empty() {
        println!("Never the tools {}", access.deny_tools.join(", "));
    }
    match rules.scope().folders() {
        None => println!("Scope: everything the MCP user sees"),
        Some(ids) => println!("Scope: {} of the {folders} folders", ids.len()),
    }
}

async fn serve(config: &Config) -> anyhow::Result<()> {
    let session = Arc::new(Session::open(config).await?);
    tracing::info!(
        "logged in as {}",
        session::display_name(&session.me().await?)
    );
    let rules = match Rules::load(config, &session).await {
        Ok(rules) => rules,
        Err(error) => {
            session.close().await;
            return Err(error);
        }
    };
    let keep_alive = session.keep_alive();
    let served = async {
        let server = PassboltMcp::new(Arc::clone(&session), Arc::new(rules))
            .serve(stdio())
            .await?;
        server.waiting().await?;
        anyhow::Ok(())
    };
    // Until the client closes stdin, or a signal.
    let result = tokio::select! {
        result = served => result,
        () = shutdown() => Ok(()),
    };
    keep_alive.abort();
    session.close().await;
    tracing::info!("logged out");
    result
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
