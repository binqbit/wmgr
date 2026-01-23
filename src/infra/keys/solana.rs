use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use bip39::{Language, Mnemonic, Seed};
use solana_derivation_path::DerivationPath;
use solana_keypair::{seed_derivable::keypair_from_seed_and_derivation_path, Keypair};

use super::svpi::get_mnemonic_from_svpi;
use crate::app::cli::SolanaKeyOptions;
use crate::config::mnemonics::get_mnemonic_profile;
use crate::utils::prompt::{prompt, prompt_hidden};

pub fn resolve_solana_keypair(opts: &SolanaKeyOptions) -> Result<Keypair> {
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
    let seed_passphrase = opts.seed_passphrase.clone().unwrap_or_default();

    if use_svpi {
        let name = match &opts.svpi_name {
            Some(v) => v.clone(),
            None => prompt("SVPI wallet name:")?,
        };
        let password = match &opts.svpi_pass {
            Some(v) => v.clone(),
            None => prompt_hidden("SVPI password:")?,
        };
        let mnemonic = get_mnemonic_from_svpi(
            &name,
            &password,
            opts.svpi_file.as_deref(),
            opts.svpi_cmd.as_deref(),
        )?;
        return keypair_from_mnemonic(&mnemonic, &derivation_path, &seed_passphrase);
    }

    if has_keyfile {
        let path = opts.keyfile.as_ref().unwrap();
        return keypair_from_file(path);
    }

    let mnemonic = opts.seed.as_ref().unwrap();
    keypair_from_mnemonic(mnemonic, &derivation_path, &seed_passphrase)
}

pub fn keypair_from_mnemonic(
    mnemonic: &str,
    derivation_path: &str,
    passphrase: &str,
) -> Result<Keypair> {
    let mnemonic =
        Mnemonic::from_phrase(mnemonic, Language::English).context("Invalid BIP39 mnemonic")?;
    let seed = Seed::new(&mnemonic, passphrase);
    let path = DerivationPath::from_absolute_path_str(derivation_path)
        .context("Invalid derivation path")?;
    keypair_from_seed_and_derivation_path(seed.as_bytes(), Some(path))
        .map_err(|err| anyhow!("Failed to derive Solana keypair: {err}"))
}

pub fn keypair_from_file(path: &Path) -> Result<Keypair> {
    let raw = fs::read_to_string(path).with_context(|| format!("Failed to read {path:?}"))?;
    let data: Vec<u8> =
        serde_json::from_str(&raw).context("Keypair file must be a JSON array of numbers")?;
    if data.len() != 64 {
        return Err(anyhow!("Keypair file must contain 64 bytes"));
    }
    Keypair::try_from(data.as_slice()).map_err(|err| anyhow!("Failed to parse keypair: {err}"))
}
