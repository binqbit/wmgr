use std::str::FromStr;

use anyhow::{anyhow, Result};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::cli::BalanceArgs;
use crate::commands::commitment_from_arg;
use crate::config::clusters::{get_cluster_config, get_usdc_mint_for_cluster};
use crate::keys::solana::resolve_solana_keypair;
use crate::services::solana::{create_rpc_client, get_balances};

pub async fn handle_balance(args: BalanceArgs) -> Result<()> {
    let owner = if let Some(address) = args.address.as_deref() {
        Pubkey::from_str(address).map_err(|err| anyhow!("Invalid address: {err}"))?
    } else {
        let keypair = resolve_solana_keypair(&args.key)?;
        keypair.pubkey()
    };

    let cluster = get_cluster_config(&args.rpc.cluster, args.rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
    let commitment = commitment_from_arg(args.rpc.commitment);
    let client = create_rpc_client(&cluster.rpc_url, commitment);

    let mint_str = get_usdc_mint_for_cluster(&cluster.name)?;
    let mint = Pubkey::from_str(mint_str).map_err(|err| anyhow!("Invalid mint address: {err}"))?;

    let balances = get_balances(&client, &owner, &mint)?;
    println!("Address: {}", balances.address);
    println!("SOL: {}", balances.sol);
    println!("USDC: {}", balances.usdc);
    Ok(())
}
