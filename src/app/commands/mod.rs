use anyhow::Result;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};

use crate::{
    app::{
        cli::{Cli, Command, CommitmentArg},
        cli_mode,
    },
    config::app_config::WmgrConfig,
};

pub mod balance;
pub mod config_cmd;
pub mod hash;
pub mod price;
pub mod send;
pub mod swap;

pub async fn run(cli: Cli) -> Result<()> {
    let mut cfg = WmgrConfig::load_from_cwd()?.unwrap_or_default();
    match cli.command {
        Some(Command::Repl) => cli_mode::run_repl(&mut cfg).await,
        Some(cmd) => run_command(cmd, &mut cfg).await,
        None => cli_mode::run_repl(&mut cfg).await,
    }
}

pub async fn run_command(cmd: Command, cfg: &mut WmgrConfig) -> Result<()> {
    match cmd {
        Command::Balance(args) => balance::handle_balance(args, cfg).await,
        Command::Send(cmd) => send::handle_send(cmd, cfg).await,
        Command::Price(args) => price::handle_price(args, cfg).await,
        Command::Buy(args) => swap::handle_buy(args, cfg).await,
        Command::Sell(args) => swap::handle_sell(args, cfg).await,
        Command::Config(args) => config_cmd::handle_config(args, cfg),
        Command::SelfHash => hash::handle_self_hash(cfg),
        Command::Repl => Ok(()),
    }
}

pub fn commitment_from_arg(arg: CommitmentArg) -> CommitmentConfig {
    let level = match arg {
        CommitmentArg::Processed => CommitmentLevel::Processed,
        CommitmentArg::Confirmed => CommitmentLevel::Confirmed,
        CommitmentArg::Finalized => CommitmentLevel::Finalized,
    };
    CommitmentConfig { commitment: level }
}
