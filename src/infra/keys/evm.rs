use std::fs;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder};

use super::svpi::get_data_from_svpi;
use crate::app::cli::EvmKeyOptions;
use crate::utils::prompt::{prompt, prompt_hidden};

pub const DEFAULT_EVM_PATH: &str = "m/44'/60'/0'/0/0";

pub fn resolve_evm_wallet(opts: &EvmKeyOptions) -> Result<LocalWallet> {
    let use_svpi = opts.svpi
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

    let sources = [use_svpi, has_privkey, has_privkey_file, has_seed]
        .iter()
        .filter(|v| **v)
        .count();

    if sources == 0 {
        return Err(anyhow!(
            "Provide --privkey, --privkey-file, --seed, or --svpi for EVM"
        ));
    }
    if sources > 1 {
        return Err(anyhow!(
            "Use only one of --privkey, --privkey-file, --seed, or --svpi for EVM"
        ));
    }

    if has_privkey {
        let normalized = normalize_privkey(opts.privkey.as_ref().unwrap())?;
        return LocalWallet::from_str(&normalized)
            .map_err(|err| anyhow!("Invalid private key: {err}"));
    }

    if has_privkey_file {
        let path = opts.privkey_file.as_ref().unwrap();
        let raw = fs::read_to_string(path).with_context(|| format!("Failed to read {path:?}"))?;
        let normalized = normalize_privkey(&raw)?;
        return LocalWallet::from_str(&normalized)
            .map_err(|err| anyhow!("Invalid private key: {err}"));
    }

    let derivation_path = opts.path.as_deref().unwrap_or(DEFAULT_EVM_PATH);
    let seed_passphrase = opts.seed_passphrase.clone().unwrap_or_default();

    if use_svpi {
        let name = match &opts.svpi_name {
            Some(v) => v.clone(),
            None => prompt("SVPI wallet name (EVM):")?,
        };
        let password = match &opts.svpi_pass {
            Some(v) => v.clone(),
            None => prompt_hidden("SVPI password:")?,
        };
        let data = get_data_from_svpi(
            &name,
            &password,
            opts.svpi_file.as_deref(),
            opts.svpi_cmd.as_deref(),
        )?;
        if looks_like_hex_privkey(&data) {
            let normalized = normalize_privkey(&data)?;
            return LocalWallet::from_str(&normalized)
                .map_err(|err| anyhow!("Invalid private key: {err}"));
        }
        return wallet_from_mnemonic(&data, derivation_path, &seed_passphrase);
    }

    let mnemonic = opts.seed.as_ref().unwrap();
    wallet_from_mnemonic(mnemonic, derivation_path, &seed_passphrase)
}

pub fn wallet_from_mnemonic(
    mnemonic: &str,
    derivation_path: &str,
    passphrase: &str,
) -> Result<LocalWallet> {
    let mut builder = MnemonicBuilder::<English>::default();
    builder = builder.phrase(mnemonic).password(passphrase);
    builder = builder
        .derivation_path(derivation_path)
        .map_err(|err| anyhow!("Invalid derivation path: {err}"))?;
    builder
        .build()
        .map_err(|err| anyhow!("Failed to derive EVM wallet: {err}"))
}

fn normalize_privkey(hex: &str) -> Result<String> {
    let trimmed = hex.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Private key is required"));
    }
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if without_prefix.len() != 64 || !without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!(
            "Private key must be 64 hex chars (with or without 0x prefix)"
        ));
    }
    Ok(format!("0x{}", without_prefix.to_lowercase()))
}

fn looks_like_hex_privkey(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    without_prefix.len() == 64 && without_prefix.chars().all(|c| c.is_ascii_hexdigit())
}
