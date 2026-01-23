use anyhow::Result;

use crate::app::cli::{
    SendCommand, SendErc20Args, SendEthArgs, SendKind, SendSolArgs, SendUsdcArgs,
};
use crate::app::commands::commitment_from_arg;
use crate::config::clusters::{get_cluster_config, get_usdc_mint_for_cluster};
use crate::infra::evm::{create_evm_provider, get_erc20_meta, transfer_erc20, transfer_eth};
use crate::infra::keys::evm::resolve_evm_wallet;
use crate::infra::keys::solana::resolve_solana_keypair;
use crate::infra::solana::{create_rpc_client, transfer_sol, transfer_spl_token};

pub async fn handle_send(cmd: SendCommand) -> Result<()> {
    match cmd.kind {
        SendKind::Sol(args) => send_sol(args).await,
        SendKind::Usdc(args) => send_usdc(args).await,
        SendKind::Eth(args) => send_eth(args).await,
        SendKind::Erc20(args) => send_erc20(args).await,
    }
}

async fn send_sol(args: SendSolArgs) -> Result<()> {
    let keypair = resolve_solana_keypair(&args.key)?;
    let cluster = get_cluster_config(&args.rpc.cluster, args.rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
    let commitment = commitment_from_arg(args.rpc.commitment);
    let client = create_rpc_client(&cluster.rpc_url, commitment);

    let sig = transfer_sol(&client, &keypair, &args.to, &args.amount)?;
    println!("SUCCESS: SOL sent. Signature: {sig}");
    Ok(())
}

async fn send_usdc(args: SendUsdcArgs) -> Result<()> {
    let keypair = resolve_solana_keypair(&args.key)?;
    let cluster = get_cluster_config(&args.rpc.cluster, args.rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);
    let commitment = commitment_from_arg(args.rpc.commitment);
    let client = create_rpc_client(&cluster.rpc_url, commitment);

    let mint = get_usdc_mint_for_cluster(&cluster.name)?;
    let sig = transfer_spl_token(&client, &keypair, &args.to, &args.amount, mint)?;
    println!("SUCCESS: USDC sent. Signature: {sig}");
    Ok(())
}

async fn send_eth(args: SendEthArgs) -> Result<()> {
    let wallet = resolve_evm_wallet(&args.key)?;
    let (provider, cfg) = create_evm_provider(args.tx.network.as_str(), args.tx.rpc.as_deref())?;

    let tx_hash = transfer_eth(
        provider,
        wallet,
        &args.to,
        &args.amount,
        args.tx.gas_price.as_deref(),
        args.tx.gas_limit,
        cfg.chain_id,
    )
    .await?;
    println!(
        "SUCCESS: Sent native token on {} (chainId {}). Tx hash: {tx_hash}",
        cfg.name, cfg.chain_id
    );
    Ok(())
}

async fn send_erc20(args: SendErc20Args) -> Result<()> {
    let wallet = resolve_evm_wallet(&args.key)?;
    let (provider, cfg) = create_evm_provider(args.tx.network.as_str(), args.tx.rpc.as_deref())?;

    let meta = get_erc20_meta(provider.clone(), &args.token).await.ok();
    let decimals = args.decimals.or_else(|| meta.as_ref().map(|m| m.decimals));

    let tx_hash = transfer_erc20(
        provider,
        wallet,
        &args.token,
        &args.to,
        &args.amount,
        decimals,
        args.tx.gas_price.as_deref(),
        args.tx.gas_limit,
        cfg.chain_id,
    )
    .await?;

    let label = meta
        .and_then(|m| m.symbol)
        .unwrap_or_else(|| "token".to_string());
    println!(
        "SUCCESS: Sent {} {} on {} (chainId {}). Tx hash: {tx_hash}",
        args.amount, label, cfg.name, cfg.chain_id
    );
    Ok(())
}
