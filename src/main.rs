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

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use passbolt_mcp::audit::Audit;
use passbolt_mcp::config::Config;
use passbolt_mcp::limits::Limits;
use passbolt_mcp::rules::Rules;
use passbolt_mcp::server::PassboltMcp;
use passbolt_mcp::server_key::{self, Pinned};
use passbolt_mcp::session::{self, Session};
use passbolt_sdk::{FolderQuery, ResourceQuery};
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
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
    /// Log in as the MCP user and serve the agent, over stdio (the default)
    Serve {
        /// Serve over HTTP instead, at `http://ADDRESS/mcp`; a loopback address only
        #[arg(long, value_name = "ADDRESS")]
        http: Option<SocketAddr>,
    },
    /// Pin the server key: run it once, before the first login
    Verify,
    /// Log in as the MCP user and tell who it is and what it sees
    Check,
}

/// Logs go to stderr: in stdio, stdout carries the MCP messages.
///
/// `rmcp` stays at `info`, whatever `PASSBOLT_MCP_LOG` asks: at `debug`, it logs the messages it
/// sends, secrets included.
fn init_logs() {
    let filter = EnvFilter::try_from_env("PASSBOLT_MCP_LOG")
        .unwrap_or_else(|_| EnvFilter::new("warn"))
        .add_directive("rmcp=info".parse().expect("a directive"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
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
        Some(Command::Serve { http }) => serve(&config, http).await?,
        None => serve(&config, None).await?,
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
    let shared: Vec<_> = resources
        .iter()
        .map(|resource| {
            (
                resource.folder_parent_id.as_deref(),
                resource
                    .permission
                    .as_ref()
                    .and_then(|permission| permission.permission_type)
                    .unwrap_or_default(),
            )
        })
        .collect();
    let rights = rules.rights(shared.iter().copied());
    println!(
        "The agent can read {} resources, read the secret of {}, change {}, share {}",
        rights.read, rights.secrets, rights.change, rights.share
    );
    let folder_ids = folders.iter().map(|folder| folder.id.as_str());
    for warning in rules.warnings(shared.iter().copied(), folder_ids) {
        println!("Warning: {warning}");
    }
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

async fn serve(config: &Config, http: Option<SocketAddr>) -> anyhow::Result<()> {
    if let Some(address) = http {
        anyhow::ensure!(
            address.ip().is_loopback(),
            "--http {address}: serve on a loopback address only, such as 127.0.0.1:8808"
        );
    }
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
    let audit = match Audit::open(&config.audit) {
        Ok(audit) => audit,
        Err(error) => {
            session.close().await;
            return Err(error);
        }
    };
    let server = PassboltMcp::new(
        Arc::clone(&session),
        Arc::new(rules),
        Arc::new(audit),
        Arc::new(Limits::new(&config.limits)),
    );
    let running = async {
        match http {
            None => {
                server.serve(stdio()).await?.waiting().await?;
            }
            Some(address) => serve_http(address, server).await?,
        }
        anyhow::Ok(())
    };
    // Until the client closes stdin, or a signal.
    let result = tokio::select! {
        result = running => result,
        () = shutdown() => Ok(()),
    };
    keep_alive.abort();
    session.close().await;
    tracing::info!("logged out");
    result
}

/// Every HTTP client shares the session of the MCP user. Requests from a web page (with an
/// `Origin`) are refused, and only loopback host names are accepted: a page cannot reach it.
async fn serve_http(address: SocketAddr, server: PassboltMcp) -> anyhow::Result<()> {
    let service: StreamableHttpService<PassboltMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::default(),
            StreamableHttpServerConfig::default().enforce_origin_validation(),
        );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("serving on http://{address}/mcp");
    axum::serve(listener, router).await?;
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
