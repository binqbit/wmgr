mod alloc;
mod app;
mod config;
mod core;
mod infra;
mod utils;

use clap::Parser;

#[global_allocator]
static GLOBAL_ALLOCATOR: alloc::ZeroingAllocator = alloc::ZeroingAllocator;

#[tokio::main]
async fn main() {
    let cli = app::cli::Cli::parse();
    if let Err(err) = app::commands::run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
