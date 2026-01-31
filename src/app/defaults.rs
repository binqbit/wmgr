use std::path::PathBuf;

use crate::{
    app::cli::{
        CommitmentArg, EvmKeyOptions, EvmNetworkArg, EvmTxOptions, SolanaKeyOptions,
        SolanaRpcOptions,
    },
    config::app_config::WmgrConfig,
};

const DEFAULT_SOLANA_CLUSTER: &str = "mainnet-beta";
const DEFAULT_SOLANA_COMMITMENT: CommitmentArg = CommitmentArg::Confirmed;
const DEFAULT_SLIPPAGE: f64 = 0.1;

pub fn apply_solana_key_defaults(mut opts: SolanaKeyOptions, cfg: &WmgrConfig) -> SolanaKeyOptions {
    let uses_svpi = opts.svpi
        || opts.svpi_name.is_some()
        || opts.svpi_file.is_some()
        || opts.svpi_cmd.is_some()
        || opts.svpi_pass.is_some();
    let has_keyfile = opts.keyfile.is_some();
    let has_seed = opts
        .seed
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !(uses_svpi || has_keyfile || has_seed) && cfg.is_svpi_mode() {
        opts.svpi = true;
    }

    let uses_svpi = opts.svpi
        || opts.svpi_name.is_some()
        || opts.svpi_file.is_some()
        || opts.svpi_cmd.is_some()
        || opts.svpi_pass.is_some();

    if uses_svpi {
        if opts.svpi_cmd.is_none() {
            if let Some(cmd) = cfg.svpi_cmd.as_deref() {
                opts.svpi_cmd = Some(PathBuf::from(cmd));
            }
        }
        if opts.svpi_file.is_none() {
            if let Some(file) = cfg.svpi_file.as_deref() {
                opts.svpi_file = Some(PathBuf::from(file));
            }
        }
        if opts.svpi_name.is_none() {
            if let Some(name) = cfg.svpi_name.as_deref() {
                opts.svpi_name = Some(name.to_string());
            }
        }
    }

    opts
}

pub fn apply_evm_key_defaults(mut opts: EvmKeyOptions, cfg: &WmgrConfig) -> EvmKeyOptions {
    let uses_svpi = opts.svpi
        || opts.svpi_name.is_some()
        || opts.svpi_file.is_some()
        || opts.svpi_cmd.is_some()
        || opts.svpi_pass.is_some();
    let has_privkey = opts.privkey.is_some();
    let has_privkey_file = opts.privkey_file.is_some();
    let has_seed = opts
        .seed
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !(uses_svpi || has_privkey || has_privkey_file || has_seed) && cfg.is_svpi_mode() {
        opts.svpi = true;
    }

    let uses_svpi = opts.svpi
        || opts.svpi_name.is_some()
        || opts.svpi_file.is_some()
        || opts.svpi_cmd.is_some()
        || opts.svpi_pass.is_some();

    if uses_svpi {
        if opts.svpi_cmd.is_none() {
            if let Some(cmd) = cfg.svpi_cmd.as_deref() {
                opts.svpi_cmd = Some(PathBuf::from(cmd));
            }
        }
        if opts.svpi_file.is_none() {
            if let Some(file) = cfg.svpi_file.as_deref() {
                opts.svpi_file = Some(PathBuf::from(file));
            }
        }
        if opts.svpi_name.is_none() {
            if let Some(name) = cfg.svpi_name.as_deref() {
                opts.svpi_name = Some(name.to_string());
            }
        }
    }

    opts
}

pub struct ResolvedSolanaRpcOptions {
    pub cluster: String,
    pub rpc: Option<String>,
    pub commitment: CommitmentArg,
}

pub fn resolve_solana_rpc_defaults(
    opts: SolanaRpcOptions,
    cfg: &WmgrConfig,
) -> ResolvedSolanaRpcOptions {
    ResolvedSolanaRpcOptions {
        cluster: opts
            .cluster
            .or_else(|| cfg.solana_cluster.clone())
            .unwrap_or_else(|| DEFAULT_SOLANA_CLUSTER.to_string()),
        rpc: opts.rpc.or_else(|| cfg.solana_rpc.clone()),
        commitment: opts
            .commitment
            .or_else(|| commitment_from_cfg(cfg))
            .unwrap_or(DEFAULT_SOLANA_COMMITMENT),
    }
}

pub fn resolve_balance_solana_defaults(
    cluster: Option<String>,
    rpc: Option<String>,
    commitment: Option<CommitmentArg>,
    cfg: &WmgrConfig,
) -> ResolvedSolanaRpcOptions {
    ResolvedSolanaRpcOptions {
        cluster: cluster
            .or_else(|| cfg.solana_cluster.clone())
            .unwrap_or_else(|| DEFAULT_SOLANA_CLUSTER.to_string()),
        rpc: rpc.or_else(|| cfg.solana_rpc.clone()),
        commitment: commitment
            .or_else(|| commitment_from_cfg(cfg))
            .unwrap_or(DEFAULT_SOLANA_COMMITMENT),
    }
}

pub struct ResolvedEvmTxOptions {
    pub network: EvmNetworkArg,
    pub rpc: Option<String>,
    pub gas_price: Option<String>,
    pub gas_limit: Option<u64>,
}

pub fn resolve_evm_tx_defaults(opts: EvmTxOptions, cfg: &WmgrConfig) -> ResolvedEvmTxOptions {
    ResolvedEvmTxOptions {
        network: opts
            .network
            .or_else(|| evm_network_from_cfg(cfg))
            .unwrap_or(EvmNetworkArg::Mainnet),
        rpc: opts.rpc.or_else(|| cfg.evm_rpc.clone()),
        gas_price: opts.gas_price.or_else(|| cfg.evm_gas_price.clone()),
        gas_limit: opts.gas_limit.or(cfg.evm_gas_limit),
    }
}

pub fn resolve_slippage(slippage: Option<f64>, cfg: &WmgrConfig) -> f64 {
    slippage.or(cfg.slippage).unwrap_or(DEFAULT_SLIPPAGE)
}

fn commitment_from_cfg(cfg: &WmgrConfig) -> Option<CommitmentArg> {
    match cfg.solana_commitment? {
        0 => Some(CommitmentArg::Processed),
        1 => Some(CommitmentArg::Confirmed),
        2 => Some(CommitmentArg::Finalized),
        _ => None,
    }
}

fn evm_network_from_cfg(cfg: &WmgrConfig) -> Option<EvmNetworkArg> {
    let raw = cfg.evm_network.as_deref()?.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.to_ascii_lowercase().as_str() {
        "mainnet" => Some(EvmNetworkArg::Mainnet),
        "sepolia" => Some(EvmNetworkArg::Sepolia),
        "holesky" => Some(EvmNetworkArg::Holesky),
        "polygon" => Some(EvmNetworkArg::Polygon),
        "polygon_amoy" | "polygon-amoy" => Some(EvmNetworkArg::PolygonAmoy),
        "bsc" => Some(EvmNetworkArg::Bsc),
        "bsc_testnet" | "bsc-testnet" => Some(EvmNetworkArg::BscTestnet),
        "avalanche" => Some(EvmNetworkArg::Avalanche),
        "avalanche_fuji" | "avalanche-fuji" => Some(EvmNetworkArg::AvalancheFuji),
        "optimism" => Some(EvmNetworkArg::Optimism),
        "arbitrum" => Some(EvmNetworkArg::Arbitrum),
        _ => None,
    }
}
