use std::fs::OpenOptions;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tracing::info;

use maestro_ai::presentation::cli::{execute, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    // Route logs to a file to avoid breaking the TUI graphical interface
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("maestro.log")?;

    let is_debug = std::env::var("MAESTRO_DEBUG").is_ok();
    let level = if is_debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_writer(Arc::new(log_file))
        .with_max_level(level)
        .with_target(false)
        .init();

    info!("Maestro initialized");

    let cli = Cli::parse();
    let _ = execute(cli).await?;

    Ok(())
}
