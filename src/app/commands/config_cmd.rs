use std::io::ErrorKind;

use anyhow::{anyhow, Result};

use crate::{
    app::cli::{ConfigArgs, ConfigCommand},
    config::app_config::{WmgrConfig, CONFIG_FILE_NAME},
};

pub fn handle_config(args: ConfigArgs, cfg: &mut WmgrConfig) -> Result<()> {
    match args.command {
        ConfigCommand::Show => {
            println!("config_file: {CONFIG_FILE_NAME}");
            println!();
            println!("svpi:");
            println!("{:15}{}", "--svpi:", cfg.is_svpi_mode());
            println!(
                "{:15}{}",
                "--svpi-name:",
                cfg.svpi_name.as_deref().unwrap_or("(not set)")
            );
            println!(
                "{:15}{}",
                "--svpi-file:",
                cfg.svpi_file.as_deref().unwrap_or("(not set)")
            );
            println!(
                "{:15}{}",
                "--svpi_cmd:",
                cfg.svpi_cmd.as_deref().unwrap_or("svpi")
            );

            println!();
            println!("solana:");
            println!(
                "{:15}{}",
                "--cluster:",
                cfg.solana_cluster.as_deref().unwrap_or("mainnet-beta")
            );
            println!(
                "{:15}{}",
                "--rpc:",
                cfg.solana_rpc.as_deref().unwrap_or("(not set)")
            );
            println!(
                "{:15}{}",
                "--commitment:",
                solana_commitment_label(cfg.solana_commitment)
            );
            println!(
                "{:15}{}",
                "--slippage:",
                cfg.slippage
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "0.1".to_string())
            );

            println!();
            println!("evm:");
            println!(
                "{:15}{}",
                "--network:",
                cfg.evm_network.as_deref().unwrap_or("mainnet")
            );
            println!(
                "{:15}{}",
                "--rpc:",
                cfg.evm_rpc.as_deref().unwrap_or("(not set)")
            );
            println!(
                "{:15}{}",
                "--gas-price:",
                cfg.evm_gas_price.as_deref().unwrap_or("(not set)")
            );
            println!(
                "{:15}{}",
                "--gas-limit:",
                cfg.evm_gas_limit
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "(not set)".to_string())
            );
            Ok(())
        }
        ConfigCommand::Set(args) => {
            let set_solana_rpc =
                args.cluster.is_some() || args.commitment.is_some() || args.slippage.is_some();
            let set_evm_rpc =
                args.network.is_some() || args.gas_price.is_some() || args.gas_limit.is_some();

            if args.svpi {
                cfg.mode = 1;
            }
            if args.no_svpi {
                cfg.mode = 0;
            }
            if let Some(cluster) = args.cluster {
                cfg.solana_cluster = Some(cluster);
            }
            if let Some(commitment) = args.commitment {
                cfg.solana_commitment = Some(match commitment {
                    crate::app::cli::CommitmentArg::Processed => 0,
                    crate::app::cli::CommitmentArg::Confirmed => 1,
                    crate::app::cli::CommitmentArg::Finalized => 2,
                });
            }
            if let Some(slippage) = args.slippage {
                cfg.slippage = Some(slippage);
            }
            if let Some(network) = args.network {
                cfg.evm_network = Some(network.as_str().to_string());
            }
            if let Some(gas_price) = args.gas_price {
                cfg.evm_gas_price = Some(gas_price);
            }
            if let Some(gas_limit) = args.gas_limit {
                cfg.evm_gas_limit = Some(gas_limit);
            }
            if let Some(cmd) = args.svpi_cmd {
                cfg.svpi_cmd = Some(cmd.display().to_string());
            }
            if let Some(file) = args.svpi_file {
                cfg.svpi_file = Some(file.display().to_string());
            }
            if let Some(name) = args.svpi_name {
                cfg.svpi_name = Some(name);
            }
            if let Some(rpc) = args.rpc {
                match (set_solana_rpc, set_evm_rpc) {
                    (true, false) => cfg.solana_rpc = Some(rpc),
                    (false, true) => cfg.evm_rpc = Some(rpc),
                    _ => {
                        cfg.solana_rpc = Some(rpc.clone());
                        cfg.evm_rpc = Some(rpc);
                    }
                }
            }

            cfg.save_to_cwd()
                .map_err(|err| anyhow!("Failed to write {CONFIG_FILE_NAME}: {err}"))?;

            println!("OK: saved {CONFIG_FILE_NAME}");
            Ok(())
        }
        ConfigCommand::Reset => {
            *cfg = WmgrConfig::default();

            let path = WmgrConfig::path_in_cwd()
                .map_err(|err| anyhow!("Failed to resolve {CONFIG_FILE_NAME} path: {err}"))?;
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => return Err(anyhow!("Failed to remove {CONFIG_FILE_NAME}: {err}")),
            }

            println!("OK: reset {CONFIG_FILE_NAME}");
            Ok(())
        }
    }
}

fn solana_commitment_label(value: Option<u8>) -> String {
    match value {
        None => "confirmed".to_string(),
        Some(0) => "processed".to_string(),
        Some(1) => "confirmed".to_string(),
        Some(2) => "finalized".to_string(),
        Some(other) => format!("unknown({other})"),
    }
}
