use std::str::FromStr;

use anyhow::{anyhow, Result};
use raydium_amm_swap::amm::client::AmmSwapClient;
use raydium_amm_swap::consts::SOL_MINT;
use raydium_amm_swap::interface::{AmmPool, PoolKeys};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::app::cli::{PriceArgs, SwapToken};
use crate::app::commands::commitment_from_arg;
use crate::config::clusters::get_cluster_config;
use crate::config::raydium::{SOL_USDC_POOL_ID, USDC_MINT};

pub async fn handle_price(args: PriceArgs) -> Result<()> {
    let _ = args.key;
    let commitment = commitment_from_arg(args.rpc.commitment);
    let cluster = get_cluster_config(&args.rpc.cluster, args.rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);

    let rpc = RpcClient::new_with_commitment(cluster.rpc_url.clone(), commitment);
    let dummy = solana_keypair::Keypair::new();
    let client = AmmSwapClient::new(rpc, dummy);

    let pool_id = Pubkey::from_str(SOL_USDC_POOL_ID)
        .map_err(|err| anyhow!("Invalid Raydium pool id: {err}"))?;
    let pool_keys: PoolKeys<AmmPool> = client.fetch_pools_keys_by_id(&pool_id).await?;
    let pool = pool_keys
        .data
        .get(0)
        .ok_or_else(|| anyhow!("Raydium pool keys not found"))?;

    if !pool_mints_match(pool) {
        return Err(anyhow!("Raydium pool mints do not match SOL/USDC"));
    }

    let rpc_pool = client.get_rpc_pool_info(&pool_id).await?;
    let base_mint = pool.mint_a.address.as_str();
    let quote_mint = pool.mint_b.address.as_str();
    let base_decimals = u8::try_from(pool.mint_a.decimals)
        .map_err(|_| anyhow!("Base mint decimals out of range"))?;
    let quote_decimals = u8::try_from(pool.mint_b.decimals)
        .map_err(|_| anyhow!("Quote mint decimals out of range"))?;

    if rpc_pool.base_reserve == 0 || rpc_pool.quote_reserve == 0 {
        return Err(anyhow!("Pool reserves are empty"));
    }

    let price_quote_per_base = {
        let base = rpc_pool.base_reserve as f64 / 10f64.powi(base_decimals as i32);
        let quote = rpc_pool.quote_reserve as f64 / 10f64.powi(quote_decimals as i32);
        quote / base
    };
    let price_base_per_quote = 1.0 / price_quote_per_base;

    let token_mint = mint_for_token(args.token);
    let token_is_base = token_mint == base_mint;
    let token_is_quote = token_mint == quote_mint;
    if !token_is_base && !token_is_quote {
        return Err(anyhow!("Token mint not found in Raydium pool"));
    }

    let other = args.token.other();
    let (price_other_per_token, price_token_per_other) = if token_is_base {
        (price_quote_per_base, price_base_per_quote)
    } else {
        (price_base_per_quote, price_quote_per_base)
    };

    println!(
        "Price: {:.8} {} per {}",
        price_other_per_token,
        other.symbol(),
        args.token.symbol()
    );
    println!(
        "Inverse: {:.8} {} per {}",
        price_token_per_other,
        args.token.symbol(),
        other.symbol()
    );

    Ok(())
}

fn mint_for_token(token: SwapToken) -> &'static str {
    match token {
        SwapToken::Sol => SOL_MINT,
        SwapToken::Usdc => USDC_MINT,
    }
}

fn pool_mints_match(pool: &AmmPool) -> bool {
    let a = pool.mint_a.address.as_str();
    let b = pool.mint_b.address.as_str();
    (a == SOL_MINT && b == USDC_MINT) || (a == USDC_MINT && b == SOL_MINT)
}
