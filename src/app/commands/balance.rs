use std::str::FromStr;

use anyhow::{anyhow, Result};
use ethers::signers::Signer as EvmSigner;
use ethers::types::Address;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as SolanaSigner;

use crate::app::cli::BalanceArgs;
use crate::app::commands::commitment_from_arg;
use crate::app::defaults::{
    apply_evm_key_defaults, apply_solana_key_defaults, resolve_balance_solana_defaults,
};
use crate::config::app_config::WmgrConfig;
use crate::config::clusters::{get_cluster_config, get_usdc_mint_for_cluster};
use crate::infra::evm::{create_evm_provider, get_native_balance};
use crate::infra::keys::evm::resolve_evm_wallet;
use crate::infra::keys::solana::resolve_solana_keypair;
use crate::infra::solana::{create_rpc_client, get_balances};

pub async fn handle_balance(args: BalanceArgs, cfg: &WmgrConfig) -> Result<()> {
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
                let key = apply_evm_key_defaults(key.into_evm(), cfg);
                let wallet = resolve_evm_wallet(&key)?;
                wallet.address()
            };

            let rpc = rpc.or_else(|| cfg.evm_rpc.clone());
            let (provider, evm_cfg) = create_evm_provider(network.as_str(), rpc.as_deref())?;
            println!("Using network: {}, RPC: {}", evm_cfg.name, evm_cfg.rpc_url);

            let (_raw, formatted) = get_native_balance(provider, owner).await?;
            println!("Address: {}", owner);
            println!("Native: {}", formatted);
            Ok(())
        }
        None => {
            let sol = resolve_balance_solana_defaults(cluster, rpc, commitment, cfg);

            let owner = if let Some(address) = address.as_deref() {
                Pubkey::from_str(address).map_err(|err| anyhow!("Invalid address: {err}"))?
            } else {
                let key = apply_solana_key_defaults(key.into_solana(), cfg);
                let keypair = resolve_solana_keypair(&key)?;
                keypair.pubkey()
            };

            let cluster = get_cluster_config(&sol.cluster, sol.rpc.as_deref())?;
            println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
            let commitment = commitment_from_arg(sol.commitment);
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
