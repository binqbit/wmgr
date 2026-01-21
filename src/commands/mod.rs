use anyhow::Result;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};

use crate::cli::{Cli, Command, CommitmentArg};

pub mod balance;
pub mod send;

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Send(cmd) => send::handle_send(cmd).await,
        Command::Balance(args) => balance::handle_balance(args).await,
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
