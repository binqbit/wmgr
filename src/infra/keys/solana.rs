use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use solana_keypair::Keypair;
use solana_sdk::pubkey::Pubkey;
use zeroize::Zeroizing;

pub use super::generate::keypair_from_mnemonic;
use super::generate::{parse_wallet, WalletMaterial};

use super::svpi::get_data_from_svpi;
use crate::app::cli::SolanaKeyOptions;
use crate::config::mnemonics::get_mnemonic_profile;
use crate::utils::prompt::{prompt, prompt_hidden};

pub fn resolve_solana_keypair(opts: &SolanaKeyOptions) -> Result<Keypair> {
    resolve_solana_material(opts)?.into_keypair()
}

pub fn resolve_solana_address(opts: &SolanaKeyOptions) -> Result<Pubkey> {
    Ok(resolve_solana_material(opts)?.address())
}

fn resolve_solana_material(opts: &SolanaKeyOptions) -> Result<WalletMaterial> {
    let use_svpi = opts.svpi
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

    let sources = [use_svpi, has_keyfile, has_seed]
        .iter()
        .filter(|v| **v)
        .count();

    if sources == 0 {
        return Err(anyhow!("Provide --keyfile, --seed, or --svpi for Solana"));
    }
    if sources > 1 {
        return Err(anyhow!(
            "Use only one of --keyfile, --seed, or --svpi for Solana"
        ));
    }

    let derivation_path = opts
        .path
        .clone()
        .unwrap_or_else(|| get_mnemonic_profile(&opts.mnemo).to_string());
    let seed_passphrase = opts.seed_passphrase.as_deref().unwrap_or_default();

    if use_svpi {
        let name = match &opts.svpi_name {
            Some(v) => v.clone(),
            None => prompt("SVPI wallet name:")?,
        };
        let password = Zeroizing::new(match &opts.svpi_pass {
            Some(v) => v.clone(),
            None => prompt_hidden("SVPI password:")?,
        });
        let resp = get_data_from_svpi(
            &name,
            &password,
            opts.svpi_file.as_deref(),
            opts.svpi_cmd.as_deref(),
        )?;
        let data = Zeroizing::new(resp.data);
        return parse_wallet(&data, &derivation_path, seed_passphrase);
    }

    if has_keyfile {
        let path = opts.keyfile.as_ref().unwrap();
        return wallet_from_file(path, &derivation_path, seed_passphrase);
    }

    let mnemonic = opts.seed.as_ref().unwrap();
    keypair_from_mnemonic(mnemonic, &derivation_path, seed_passphrase).map(WalletMaterial::Keypair)
}

fn wallet_from_file(
    path: &Path,
    derivation_path: &str,
    passphrase: &str,
) -> Result<WalletMaterial> {
    let raw = Zeroizing::new(
        fs::read_to_string(path).with_context(|| format!("Failed to read {path:?}"))?,
    );
    parse_wallet(&raw, derivation_path, passphrase)
}
