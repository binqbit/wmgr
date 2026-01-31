use anyhow::Result;

use crate::app::cli::{
    SendCommand, SendErc20Args, SendEthArgs, SendKind, SendSolArgs, SendUsdcArgs,
};
use crate::app::commands::commitment_from_arg;
use crate::app::defaults::{
    apply_evm_key_defaults, apply_solana_key_defaults, resolve_evm_tx_defaults,
    resolve_solana_rpc_defaults,
};
use crate::config::app_config::WmgrConfig;
use crate::config::clusters::{get_cluster_config, get_usdc_mint_for_cluster};
use crate::infra::evm::{create_evm_provider, get_erc20_meta, transfer_erc20, transfer_eth};
use crate::infra::keys::evm::resolve_evm_wallet;
use crate::infra::keys::solana::resolve_solana_keypair;
use crate::infra::solana::{create_rpc_client, transfer_sol, transfer_spl_token};

pub async fn handle_send(cmd: SendCommand, cfg: &WmgrConfig) -> Result<()> {
    match cmd.kind {
        SendKind::Sol(args) => send_sol(args, cfg).await,
        SendKind::Usdc(args) => send_usdc(args, cfg).await,
        SendKind::Eth(args) => send_eth(args, cfg).await,
        SendKind::Erc20(args) => send_erc20(args, cfg).await,
    }
}

async fn send_sol(args: SendSolArgs, cfg: &WmgrConfig) -> Result<()> {
    let SendSolArgs {
        to,
        amount,
        key,
        rpc,
    } = args;
    let key = apply_solana_key_defaults(key, cfg);
    let rpc = resolve_solana_rpc_defaults(rpc, cfg);
    let keypair = resolve_solana_keypair(&key)?;
    let cluster = get_cluster_config(&rpc.cluster, rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
    let commitment = commitment_from_arg(rpc.commitment);
    let client = create_rpc_client(&cluster.rpc_url, commitment);

    let sig = transfer_sol(&client, &keypair, &to, &amount)?;
    println!("SUCCESS: SOL sent. Signature: {sig}");
    Ok(())
}

async fn send_usdc(args: SendUsdcArgs, cfg: &WmgrConfig) -> Result<()> {
    let SendUsdcArgs {
        to,
        amount,
        key,
        rpc,
    } = args;
    let key = apply_solana_key_defaults(key, cfg);
    let rpc = resolve_solana_rpc_defaults(rpc, cfg);
    let keypair = resolve_solana_keypair(&key)?;
    let cluster = get_cluster_config(&rpc.cluster, rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
    let commitment = commitment_from_arg(rpc.commitment);
    let client = create_rpc_client(&cluster.rpc_url, commitment);

    let mint = get_usdc_mint_for_cluster(&cluster.name)?;
    let sig = transfer_spl_token(&client, &keypair, &to, &amount, mint)?;
    println!("SUCCESS: USDC sent. Signature: {sig}");
    Ok(())
}

async fn send_eth(args: SendEthArgs, cfg: &WmgrConfig) -> Result<()> {
    let SendEthArgs {
        to,
        amount,
        key,
        tx,
    } = args;
    let key = apply_evm_key_defaults(key, cfg);
    let tx = resolve_evm_tx_defaults(tx, cfg);
    let wallet = resolve_evm_wallet(&key)?;
    let (provider, evm_cfg) = create_evm_provider(tx.network.as_str(), tx.rpc.as_deref())?;

    let tx_hash = transfer_eth(
        provider,
        wallet,
        &to,
        &amount,
        tx.gas_price.as_deref(),
        tx.gas_limit,
        evm_cfg.chain_id,
    )
    .await?;
    println!(
        "SUCCESS: Sent native token on {} (chainId {}). Tx hash: {tx_hash}",
        evm_cfg.name, evm_cfg.chain_id
    );
    Ok(())
}

async fn send_erc20(args: SendErc20Args, cfg: &WmgrConfig) -> Result<()> {
    let SendErc20Args {
        token,
        to,
        amount,
        decimals,
        key,
        tx,
    } = args;
    let key = apply_evm_key_defaults(key, cfg);
    let tx = resolve_evm_tx_defaults(tx, cfg);
    let wallet = resolve_evm_wallet(&key)?;
    let (provider, evm_cfg) = create_evm_provider(tx.network.as_str(), tx.rpc.as_deref())?;

    let meta = get_erc20_meta(provider.clone(), &token).await.ok();
    let decimals = decimals.or_else(|| meta.as_ref().map(|m| m.decimals));

    let tx_hash = transfer_erc20(
        provider,
        wallet,
        &token,
        &to,
        &amount,
        decimals,
        tx.gas_price.as_deref(),
        tx.gas_limit,
        evm_cfg.chain_id,
    )
    .await?;

    let label = meta
        .and_then(|m| m.symbol)
        .unwrap_or_else(|| "token".to_string());
    println!(
        "SUCCESS: Sent {} {} on {} (chainId {}). Tx hash: {tx_hash}",
        amount, label, evm_cfg.name, evm_cfg.chain_id
    );
    Ok(())
}
