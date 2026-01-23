mod app;
mod config;
mod core;
mod infra;
mod utils;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = app::cli::Cli::parse();
    if let Err(err) = app::commands::run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
