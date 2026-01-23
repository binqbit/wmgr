use anyhow::Result;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};

use crate::app::cli::{Cli, Command, CommitmentArg};

pub mod balance;
pub mod price;
pub mod send;
pub mod swap;

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Balance(args) => balance::handle_balance(args).await,
        Command::Send(cmd) => send::handle_send(cmd).await,
        Command::Price(args) => price::handle_price(args).await,
        Command::Buy(args) => swap::handle_buy(args).await,
        Command::Sell(args) => swap::handle_sell(args).await,
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
