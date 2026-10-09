use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Result};
use bip39::{Language, Mnemonic};
use serde::Deserialize;
use solana_keypair::Keypair;
use solana_sdk::pubkey::Pubkey;
use zeroize::Zeroizing;

use super::{
    generate::{parse_wallet, WalletMaterial},
    svpi::get_data_from_svpi,
};
use crate::{config::mnemonics::get_mnemonic_profile, utils::prompt::prompt_hidden};

/// Load each named wallet once, then borrow its signer or public address for
/// the program operation. Deliberately does not implement Debug.
pub struct ProgramWallets<'a> {
    file: Option<&'a Path>,
    command: Option<&'a Path>,
    password: Option<&'a str>,
    wallets: HashMap<String, WalletMaterial>,
}

impl<'a> ProgramWallets<'a> {
    pub fn new(
        file: Option<&'a Path>,
        command: Option<&'a Path>,
        password: Option<&'a str>,
    ) -> Self {
        Self {
            file,
            command,
            password,
            wallets: HashMap::new(),
        }
    }

    pub fn load(&mut self, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            bail!("SVPI wallet name is required");
        }
        if self.wallets.contains_key(name) {
            return Ok(());
        }
        let password = Zeroizing::new(match self.password {
            Some(password) => password.to_owned(),
            None => prompt_hidden(&format!("SVPI password for '{name}':"))?,
        });
        let response = get_data_from_svpi(name, &password, self.file, self.command)
            .map_err(|error| anyhow!("Failed to load SVPI wallet '{name}': {error}"))?;
        drop(password);
        let data = Zeroizing::new(response.data);
        let wallet = parse_program_wallet(&data, name)
            .map_err(|error| anyhow!("Invalid Solana wallet in SVPI entry '{name}': {error}"))?;
        self.wallets.insert(name.to_owned(), wallet);
        Ok(())
    }

    pub fn signer(&self, name: &str) -> Result<&Keypair> {
        match self.wallet(name)? {
            WalletMaterial::Keypair(keypair) | WalletMaterial::PrivateKey(keypair) => Ok(keypair),
            WalletMaterial::Mnemonic(wallet) => Ok(&wallet.keypair),
            WalletMaterial::PublicKey(_) => {
                bail!("SVPI wallet '{name}' contains only a public address and cannot sign")
            }
        }
    }

    pub fn address(&self, name: &str) -> Result<Pubkey> {
        Ok(self.wallet(name)?.address())
    }

    fn wallet(&self, name: &str) -> Result<&WalletMaterial> {
        self.wallets
            .get(name)
            .ok_or_else(|| anyhow!("SVPI wallet '{name}' has not been loaded"))
    }
}

fn parse_program_wallet(input: &str, name: &str) -> Result<WalletMaterial> {
    let input = input.trim();
    let unquoted;
    let input = if input.starts_with('"') {
        unquoted = Zeroizing::new(
            serde_json::from_str::<String>(input)
                .map_err(|_| anyhow!("Invalid wallet JSON string"))?,
        );
        unquoted.trim()
    } else {
        input
    };
    let path = get_mnemonic_profile("trustwallet");
    if is_mnemonic_record(input) {
        // Recovery metadata verifies the derived address. A passphrase-free
        // record needs no extra prompt; a mismatch gets one explicit retry.
        if let Ok(wallet) = parse_wallet(input, path, "") {
            return Ok(wallet);
        }
    } else if Mnemonic::from_phrase(input, Language::English).is_err() {
        return parse_wallet(input, path, "");
    }

    // A bare phrase has no recorded address: there is no way to infer whether
    // its owner used a BIP39 passphrase, so ask rather than choose another key.
    let passphrase = Zeroizing::new(prompt_hidden(&format!(
        "BIP39 passphrase for '{name}' (Enter for none):"
    ))?);
    parse_wallet(input, path, &passphrase)
}

fn is_mnemonic_record(input: &str) -> bool {
    if input.starts_with("Seed phrase:") {
        return true;
    }
    #[derive(Deserialize)]
    struct RecordKind<'a> {
        schema: &'a str,
        kind: Option<&'a str>,
    }
    match serde_json::from_str::<RecordKind<'_>>(input) {
        Ok(record) => {
            record.schema == "wmgr.mnemonic.v1"
                || (record.schema == "wmgr.wallet.v1" && record.kind == Some("mnemonic"))
        }
        Err(_) => false,
    }
}
