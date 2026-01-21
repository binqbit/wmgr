mod cli;
mod commands;
mod config;
mod keys;
mod services;
mod utils;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    if let Err(err) = commands::run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
