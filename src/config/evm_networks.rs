use anyhow::{anyhow, Result};

pub struct EvmNetworkConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
}

pub fn get_evm_network_config(name: &str, override_rpc: Option<&str>) -> Result<EvmNetworkConfig> {
    let key = name.trim().to_lowercase();
    let (chain_id, default_url) = match key.as_str() {
        "mainnet" => (1, "https://cloudflare-eth.com"),
        "sepolia" => (11155111, "https://rpc.sepolia.org"),
        "holesky" => (17000, "https://ethereum-holesky.publicnode.com"),
        "polygon" => (137, "https://polygon-rpc.com"),
        "polygon_amoy" => (80002, "https://rpc-amoy.polygon.technology"),
        "bsc" => (56, "https://bsc-dataseed.binance.org"),
        "bsc_testnet" => (97, "https://data-seed-prebsc-1-s1.binance.org:8545"),
        "avalanche" => (43114, "https://api.avax.network/ext/bc/C/rpc"),
        "avalanche_fuji" => (43113, "https://api.avax-test.network/ext/bc/C/rpc"),
        "optimism" => (10, "https://mainnet.optimism.io"),
        "arbitrum" => (42161, "https://arb1.arbitrum.io/rpc"),
        _ => return Err(anyhow!("Unknown EVM network: {name}")),
    };

    Ok(EvmNetworkConfig {
        name: key,
        chain_id,
        rpc_url: override_rpc.unwrap_or(default_url).to_string(),
    })
}
