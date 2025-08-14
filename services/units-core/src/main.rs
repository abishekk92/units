use anyhow::Result;
use clap::Parser;
use log::info;
use std::net::SocketAddr;
use tokio::signal;

mod config;
mod error;
mod json_rpc;
mod server;
mod service;

use config::Config;
use server::UnitsServer;

#[derive(Parser)]
#[command(name = "units-core-service")]
#[command(about = "UNITS Core service providing JSON-RPC and gRPC endpoints")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// JSON-RPC server address
    #[arg(long, default_value = "127.0.0.1:8080")]
    json_rpc_addr: SocketAddr,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&args.log_level)
    ).init();

    info!("Starting UNITS Core service");

    // Load configuration
    let config = Config::load(&args.config)?;
    info!("Configuration loaded from: {}", args.config);

    // Initialize server
    let server = UnitsServer::new(config).await?;
    info!("UNITS server initialized");

    // Start JSON-RPC server
    let json_rpc_server = server.start_json_rpc_server(args.json_rpc_addr).await?;
    info!("JSON-RPC server started on {}", args.json_rpc_addr);
    let handle = tokio::spawn(json_rpc_server);

    info!("UNITS Core service is running");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping servers...");

    // Gracefully shutdown server
    handle.abort();

    info!("UNITS Core service stopped");
    Ok(())
}