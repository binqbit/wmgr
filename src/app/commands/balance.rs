use std::str::FromStr;

use anyhow::{anyhow, Result};
use ethers::signers::Signer as EvmSigner;
use ethers::types::Address;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as SolanaSigner;

use crate::app::cli::BalanceArgs;
use crate::app::commands::commitment_from_arg;
use crate::config::clusters::{get_cluster_config, get_usdc_mint_for_cluster};
use crate::infra::evm::{create_evm_provider, get_native_balance};
use crate::infra::keys::evm::resolve_evm_wallet;
use crate::infra::keys::solana::resolve_solana_keypair;
use crate::infra::solana::{create_rpc_client, get_balances};

pub async fn handle_balance(args: BalanceArgs) -> Result<()> {
    let BalanceArgs {
        address,
        network,
        rpc,
        cluster,
        commitment,
        key,
    } = args;

    match network {
        Some(network) => {
            let owner = if let Some(address) = address.as_deref() {
                Address::from_str(address).map_err(|err| anyhow!("Invalid address: {err}"))?
            } else {
                let wallet = resolve_evm_wallet(&key.into_evm())?;
                wallet.address()
            };

            let (provider, cfg) = create_evm_provider(network.as_str(), rpc.as_deref())?;
            println!("Using network: {}, RPC: {}", cfg.name, cfg.rpc_url);

            let (_raw, formatted) = get_native_balance(provider, owner).await?;
            println!("Address: {}", owner);
            println!("Native: {}", formatted);
            Ok(())
        }
        None => {
            let owner = if let Some(address) = address.as_deref() {
                Pubkey::from_str(address).map_err(|err| anyhow!("Invalid address: {err}"))?
            } else {
                let keypair = resolve_solana_keypair(&key.into_solana())?;
                keypair.pubkey()
            };

            let cluster = get_cluster_config(&cluster, rpc.as_deref())?;
            println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
            let commitment = commitment_from_arg(commitment);
            let client = create_rpc_client(&cluster.rpc_url, commitment);

            let mint_str = get_usdc_mint_for_cluster(&cluster.name)?;
            let mint =
                Pubkey::from_str(mint_str).map_err(|err| anyhow!("Invalid mint address: {err}"))?;

            let balances = get_balances(&client, &owner, &mint)?;
            println!("Address: {}", balances.address);
            println!("SOL: {}", balances.sol);
            println!("USDC: {}", balances.usdc);
            Ok(())
        }
    }
}
