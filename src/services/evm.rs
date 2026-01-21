use std::sync::Arc;

use anyhow::{anyhow, Result};
use ethers::prelude::*;
use ethers::types::{Address, U256};
use ethers::utils::{parse_ether, parse_units};

use crate::config::evm_networks::{get_evm_network_config, EvmNetworkConfig};

abigen!(
    IERC20,
    r#"[
        function decimals() view returns (uint8)
        function symbol() view returns (string)
        function transfer(address to, uint256 amount) returns (bool)
    ]"#
);

pub struct Erc20Meta {
    pub decimals: u8,
    pub symbol: Option<String>,
}

pub fn create_evm_provider(
    network: &str,
    rpc_override: Option<&str>,
) -> Result<(Provider<Http>, EvmNetworkConfig)> {
    let cfg = get_evm_network_config(network, rpc_override)?;
    let provider = Provider::<Http>::try_from(cfg.rpc_url.as_str())
        .map_err(|err| anyhow!("Invalid RPC URL: {err}"))?;
    Ok((provider, cfg))
}

pub async fn transfer_eth(
    provider: Provider<Http>,
    wallet: LocalWallet,
    to: &str,
    amount: &str,
    gas_price: Option<&str>,
    gas_limit: Option<u64>,
    chain_id: u64,
) -> Result<TxHash> {
    let to_addr: Address = to.parse().map_err(|err| anyhow!("Invalid recipient address: {err}"))?;
    let value = parse_ether(amount)?;

    let wallet = wallet.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    let mut tx = TransactionRequest::new().to(to_addr).value(value);
    if let Some(gp) = maybe_gas_price(gas_price)? {
        tx = tx.gas_price(gp);
    }
    if let Some(gl) = gas_limit {
        tx = tx.gas(gl);
    }

    let pending = client.send_transaction(tx, None).await?;
    let tx_hash = pending.tx_hash();
    let _receipt = pending.await?;
    Ok(tx_hash)
}

pub async fn transfer_erc20(
    provider: Provider<Http>,
    wallet: LocalWallet,
    token: &str,
    to: &str,
    amount: &str,
    decimals: Option<u8>,
    gas_price: Option<&str>,
    gas_limit: Option<u64>,
    chain_id: u64,
) -> Result<TxHash> {
    let token_addr: Address = token.parse().map_err(|err| anyhow!("Invalid token address: {err}"))?;
    let to_addr: Address = to.parse().map_err(|err| anyhow!("Invalid recipient address: {err}"))?;

    let wallet = wallet.with_chain_id(chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    let contract = IERC20::new(token_addr, client.clone());
    let token_decimals = match decimals {
        Some(v) => v,
        None => contract.decimals().call().await?,
    };
    let amount_units = parse_units(amount, token_decimals as u32)?.into();

    let mut call = contract.transfer(to_addr, amount_units);
    if let Some(gp) = maybe_gas_price(gas_price)? {
        call = call.gas_price(gp);
    }
    if let Some(gl) = gas_limit {
        call = call.gas(gl);
    }

    let pending = call.send().await?;
    let tx_hash = pending.tx_hash();
    let _receipt = pending.await?;
    Ok(tx_hash)
}

pub async fn get_erc20_meta(
    provider: Provider<Http>,
    token: &str,
) -> Result<Erc20Meta> {
    let token_addr: Address = token.parse().map_err(|err| anyhow!("Invalid token address: {err}"))?;
    let contract = IERC20::new(token_addr, provider.into());
    let decimals = contract.decimals().call().await?;
    let symbol = contract.symbol().call().await.ok();
    Ok(Erc20Meta { decimals, symbol })
}

fn maybe_gas_price(gwei: Option<&str>) -> Result<Option<U256>> {
    let Some(value) = gwei else { return Ok(None) };
    let parsed = parse_units(value, "gwei")?.into();
    Ok(Some(parsed))
}
