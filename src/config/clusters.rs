use anyhow::{anyhow, Result};

pub struct ClusterConfig {
    pub name: String,
    pub rpc_url: String,
}

pub fn get_cluster_config(name: &str, override_url: Option<&str>) -> Result<ClusterConfig> {
    let key = name.trim();
    let default_url = match key {
        "mainnet-beta" => "https://api.mainnet-beta.solana.com",
        "devnet" => "https://api.devnet.solana.com",
        "testnet" => "https://api.testnet.solana.com",
        "localnet" => "http://127.0.0.1:8899",
        _ => return Err(anyhow!("Unknown cluster: {name}")),
    };

    Ok(ClusterConfig {
        name: key.to_string(),
        rpc_url: override_url.unwrap_or(default_url).to_string(),
    })
}

pub fn get_usdc_mint_for_cluster(name: &str) -> Result<&'static str> {
    match name.trim() {
        "mainnet-beta" => Ok("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        "devnet" => Ok("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"),
        "testnet" => Ok("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"),
        "localnet" => Ok("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        _ => Err(anyhow!("No USDC mint configured for cluster: {name}")),
    }
}
