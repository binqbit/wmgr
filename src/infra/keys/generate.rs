use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use bip39::{Language, Mnemonic, Seed};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize};
use solana_derivation_path::DerivationPath;
use solana_keypair::{seed_derivable::keypair_from_seed_and_derivation_path, Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use zeroize::Zeroizing;

const MNEMONIC_SCHEMA: &str = "wmgr.mnemonic.v1";
const WALLET_SCHEMA: &str = "wmgr.wallet.v1";

// Intentionally no Debug implementation: a mnemonic is a wallet secret.
pub struct MnemonicWallet {
    pub keypair: Keypair,
    pub phrase: Zeroizing<String>,
    pub derivation_path: String,
}

// These variants deliberately do not implement Debug: three contain secrets.
pub enum WalletMaterial {
    Keypair(Keypair),
    PrivateKey(Keypair),
    Mnemonic(MnemonicWallet),
    PublicKey(Pubkey),
}

impl WalletMaterial {
    pub fn address(&self) -> Pubkey {
        match self {
            Self::Keypair(keypair) | Self::PrivateKey(keypair) => keypair.pubkey(),
            Self::Mnemonic(wallet) => wallet.keypair.pubkey(),
            Self::PublicKey(address) => *address,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Keypair(_) => "keypair",
            Self::PrivateKey(_) => "private-key",
            Self::Mnemonic(_) => "mnemonic",
            Self::PublicKey(_) => "public-key",
        }
    }

    pub fn into_keypair(self) -> Result<Keypair> {
        match self {
            Self::Keypair(keypair) | Self::PrivateKey(keypair) => Ok(keypair),
            Self::Mnemonic(wallet) => Ok(wallet.keypair),
            Self::PublicKey(_) => bail!(
                "Public-key-only wallets cannot sign; provide a private key, keypair, or mnemonic"
            ),
        }
    }
}

pub fn keypair_from_mnemonic(
    phrase: &str,
    derivation_path: &str,
    passphrase: &str,
) -> Result<Keypair> {
    let mnemonic = Mnemonic::from_phrase(phrase, Language::English)
        .map_err(|_| anyhow!("Invalid BIP39 mnemonic"))?;
    derive_keypair(&mnemonic, derivation_path, passphrase)
}

fn derive_keypair(mnemonic: &Mnemonic, derivation_path: &str, passphrase: &str) -> Result<Keypair> {
    let seed = Seed::new(mnemonic, passphrase);
    let path = DerivationPath::from_absolute_path_str(derivation_path)
        .map_err(|_| anyhow!("Invalid derivation path"))?;
    keypair_from_seed_and_derivation_path(seed.as_bytes(), Some(path))
        .map_err(|_| anyhow!("Failed to derive Solana keypair"))
}

#[derive(Serialize)]
struct WalletRecord<'a> {
    schema: &'static str,
    kind: &'static str,
    address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    keypair: Option<&'a [u8]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    private_key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phrase: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    derivation_path: Option<&'a str>,
}

/// An unambiguous, versioned record for encrypted vault storage.
pub fn serialize_wallet(material: &WalletMaterial) -> Result<Zeroizing<String>> {
    let keypair_bytes = match material {
        WalletMaterial::Keypair(keypair) => Some(Zeroizing::new(keypair.to_bytes())),
        _ => None,
    };
    let private_key = match material {
        WalletMaterial::PrivateKey(keypair) => {
            Some(Zeroizing::new(hex::encode(keypair.secret_bytes())))
        }
        _ => None,
    };
    let (phrase, derivation_path) = match material {
        WalletMaterial::Mnemonic(wallet) => (
            Some(wallet.phrase.as_str()),
            Some(wallet.derivation_path.as_str()),
        ),
        _ => (None, None),
    };
    let record = WalletRecord {
        schema: WALLET_SCHEMA,
        kind: material.kind(),
        address: material.address().to_string(),
        keypair: keypair_bytes.as_ref().map(|bytes| bytes.as_slice()),
        private_key: private_key.as_ref().map(|key| key.as_str()),
        phrase,
        derivation_path,
    };
    Ok(Zeroizing::new(serde_json::to_string(&record)?))
}

pub fn save_wallet(path: &Path, material: &WalletMaterial) -> Result<()> {
    match material {
        WalletMaterial::Keypair(keypair) | WalletMaterial::PrivateKey(keypair) => {
            save_keypair(path, keypair)
        }
        WalletMaterial::Mnemonic(wallet) => save_mnemonic(path, wallet),
        WalletMaterial::PublicKey(address) => {
            write_secret_file(path, |file| writeln!(file, "{address}"))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedWalletBackup {
    #[serde(rename = "schema")]
    _schema: String,
    kind: String,
    address: String,
    #[serde(default, deserialize_with = "deserialize_optional_bytes")]
    keypair: Option<Zeroizing<Vec<u8>>>,
    #[serde(default, deserialize_with = "deserialize_optional_secret")]
    private_key: Option<Zeroizing<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_secret")]
    phrase: Option<Zeroizing<String>>,
    derivation_path: Option<String>,
}

fn deserialize_optional_bytes<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<Vec<u8>>>, D::Error> {
    Ok(Option::<Vec<u8>>::deserialize(deserializer)?.map(Zeroizing::new))
}

fn deserialize_optional_secret<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.map(Zeroizing::new))
}

/// Recognize standard wallet representations automatically. For raw 32-byte
/// values, hex/JSON bytes represent a private seed and Base58 represents a
/// public address. Versioned records carry their own authoritative kind.
pub fn parse_wallet(
    input: &str,
    default_derivation_path: &str,
    passphrase: &str,
) -> Result<WalletMaterial> {
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
    if input.is_empty() {
        bail!("Wallet input is empty");
    }
    let material = if input.starts_with('{') {
        parse_wallet_record(input, passphrase)?
    } else if input.starts_with("Seed phrase:") {
        WalletMaterial::Mnemonic(parse_mnemonic_text(input, passphrase)?)
    } else if let Ok(mnemonic) = Mnemonic::from_phrase(input, Language::English) {
        // Legacy bare phrases used the SDK's permissive path parser. Keep
        // that derivation, but save the exact hardened path for recovery.
        let path = DerivationPath::from_absolute_path_str(default_derivation_path)
            .map_err(|_| anyhow!("Invalid derivation path"))?;
        WalletMaterial::Mnemonic(wallet_from_mnemonic(
            &mnemonic,
            &format!("{path:?}"),
            passphrase,
        )?)
    } else {
        let bytes = decode_key_bytes(input)?;
        match bytes.len() {
            64 => WalletMaterial::Keypair(validated_keypair(&bytes)?),
            32 if input.starts_with('[') || is_hex_key(input) => {
                WalletMaterial::PrivateKey(private_keypair(&bytes)?)
            }
            32 => {
                let bytes: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| anyhow!("Public key must contain exactly 32 bytes"))?;
                WalletMaterial::PublicKey(Pubkey::new_from_array(bytes))
            }
            _ => bail!("Wallet key material must contain a 32-byte key or a 64-byte keypair"),
        }
    };
    Ok(material)
}

fn parse_wallet_record(input: &str, passphrase: &str) -> Result<WalletMaterial> {
    #[derive(Deserialize)]
    struct Schema<'a> {
        schema: &'a str,
    }
    let schema: Schema = serde_json::from_str(input)
        .map_err(|_| anyhow!("Invalid wallet record JSON or missing schema"))?;
    if schema.schema == MNEMONIC_SCHEMA {
        let backup: MnemonicBackup = serde_json::from_str(input)
            .map_err(|_| anyhow!("Invalid mnemonic backup JSON or fields"))?;
        let wallet = wallet_from_phrase(&backup.phrase, &backup.derivation_path, passphrase)?;
        verify_address(wallet.keypair.pubkey(), &backup.address)?;
        return Ok(WalletMaterial::Mnemonic(wallet));
    }
    if schema.schema != WALLET_SCHEMA {
        bail!("Unsupported wallet record schema");
    }
    let backup: TypedWalletBackup =
        serde_json::from_str(input).map_err(|_| anyhow!("Invalid typed wallet JSON or fields"))?;
    let material = match backup.kind.as_str() {
        "keypair"
            if backup.private_key.is_none()
                && backup.phrase.is_none()
                && backup.derivation_path.is_none() =>
        {
            WalletMaterial::Keypair(validated_keypair(
                backup
                    .keypair
                    .as_deref()
                    .ok_or_else(|| anyhow!("Keypair bytes are required"))?,
            )?)
        }
        "private-key"
            if backup.keypair.is_none()
                && backup.phrase.is_none()
                && backup.derivation_path.is_none() =>
        {
            let bytes = decode_key_bytes(
                backup
                    .private_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("Private key is required"))?,
            )?;
            WalletMaterial::PrivateKey(private_keypair(&bytes)?)
        }
        "mnemonic" if backup.keypair.is_none() && backup.private_key.is_none() => {
            WalletMaterial::Mnemonic(wallet_from_phrase(
                backup
                    .phrase
                    .as_deref()
                    .ok_or_else(|| anyhow!("Mnemonic phrase is required"))?,
                backup
                    .derivation_path
                    .as_deref()
                    .ok_or_else(|| anyhow!("Derivation path is required"))?,
                passphrase,
            )?)
        }
        "public-key"
            if backup.keypair.is_none()
                && backup.private_key.is_none()
                && backup.phrase.is_none()
                && backup.derivation_path.is_none() =>
        {
            WalletMaterial::PublicKey(
                backup
                    .address
                    .parse()
                    .map_err(|_| anyhow!("Invalid Solana public address"))?,
            )
        }
        _ => bail!("Unsupported wallet kind or fields inconsistent with its kind"),
    };
    verify_address(material.address(), &backup.address)?;
    Ok(material)
}

fn verify_address(actual: Pubkey, expected: &str) -> Result<()> {
    if actual.to_string() != expected {
        bail!("Wallet address does not match its contents; check the BIP39 passphrase and record");
    }
    Ok(())
}

fn decode_key_bytes(input: &str) -> Result<Zeroizing<Vec<u8>>> {
    if input.starts_with('[') {
        return serde_json::from_str::<Vec<u8>>(input)
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("Key material must be a JSON array of bytes"));
    }
    let hex_input = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"));
    if let Some(hex_input) = hex_input {
        return hex::decode(hex_input)
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("Invalid hexadecimal key material"));
    }
    if is_hex_key(input) {
        return hex::decode(input)
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("Invalid hexadecimal key material"));
    }
    bs58::decode(input)
        .into_vec()
        .map(Zeroizing::new)
        .map_err(|_| {
            anyhow!("Wallet input is not a supported mnemonic, JSON record, hex, or Base58 key")
        })
}

fn is_hex_key(input: &str) -> bool {
    input.starts_with("0x")
        || input.starts_with("0X")
        || (matches!(input.len(), 64 | 128) && input.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn validated_keypair(bytes: &[u8]) -> Result<Keypair> {
    if bytes.len() != 64 {
        bail!("A keypair must contain 64 bytes");
    }
    Keypair::try_from(bytes)
        .map_err(|_| anyhow!("Keypair public key does not match its private seed"))
}

fn private_keypair(bytes: &[u8]) -> Result<Keypair> {
    match bytes.len() {
        32 => {
            let mut seed = Zeroizing::new([0u8; 32]);
            seed.copy_from_slice(bytes);
            Ok(Keypair::new_from_array(*seed))
        }
        64 => validated_keypair(bytes),
        _ => {
            bail!("A Solana private key must contain a 32-byte seed or a validated 64-byte keypair")
        }
    }
}

pub fn random_keypair() -> Result<Keypair> {
    let mut secret = Zeroizing::new([0u8; 32]);
    OsRng
        .try_fill_bytes(secret.as_mut())
        .context("Operating system randomness is unavailable")?;
    Ok(Keypair::new_from_array(*secret))
}

pub fn mnemonic_wallet(
    word_count: usize,
    derivation_path: &str,
    passphrase: &str,
) -> Result<MnemonicWallet> {
    let entropy_bytes = match word_count {
        12 => 16,
        15 => 20,
        18 => 24,
        21 => 28,
        24 => 32,
        _ => bail!("Mnemonic word count must be 12, 15, 18, 21, or 24"),
    };
    validate_derivation_path(derivation_path)?;

    let mut entropy = Zeroizing::new(vec![0u8; entropy_bytes]);
    OsRng
        .try_fill_bytes(&mut entropy)
        .context("Operating system randomness is unavailable")?;
    let mnemonic = Mnemonic::from_entropy(&entropy, Language::English)
        .map_err(|_| anyhow!("Failed to encode BIP39 entropy"))?;
    wallet_from_mnemonic(&mnemonic, derivation_path, passphrase)
}

fn validate_derivation_path(value: &str) -> Result<()> {
    let parsed = DerivationPath::from_absolute_path_str(value)
        .map_err(|_| anyhow!("Invalid derivation path"))?;
    // The SDK silently hardens every component. Require the caller's path to
    // describe that exact derivation so recovery metadata cannot be misleading.
    if format!("{parsed:?}") != value {
        bail!("Derivation path must be an absolute canonical path with hardened components (for example m/44'/501'/0')");
    }
    Ok(())
}

fn wallet_from_phrase(
    phrase: &str,
    derivation_path: &str,
    passphrase: &str,
) -> Result<MnemonicWallet> {
    let mnemonic = Mnemonic::from_phrase(phrase, Language::English)
        .map_err(|_| anyhow!("Invalid BIP39 mnemonic"))?;
    wallet_from_mnemonic(&mnemonic, derivation_path, passphrase)
}

fn wallet_from_mnemonic(
    mnemonic: &Mnemonic,
    derivation_path: &str,
    passphrase: &str,
) -> Result<MnemonicWallet> {
    validate_derivation_path(derivation_path)?;
    let keypair = derive_keypair(mnemonic, derivation_path, passphrase)
        .map_err(|_| anyhow!("Failed to derive wallet from the BIP39 recovery phrase"))?;
    Ok(MnemonicWallet {
        keypair,
        phrase: Zeroizing::new(mnemonic.phrase().to_owned()),
        derivation_path: derivation_path.to_owned(),
    })
}

pub fn save_keypair(path: &Path, keypair: &Keypair) -> Result<()> {
    let keypair_bytes = Zeroizing::new(keypair.to_bytes());
    let mut serialized = Zeroizing::new(serde_json::to_vec(keypair_bytes.as_slice())?);
    serialized.push(b'\n');
    write_secret_file(path, |file| file.write_all(&serialized))
}

pub fn save_mnemonic(path: &Path, wallet: &MnemonicWallet) -> Result<()> {
    let recovery_path = path.with_extension("txt");
    // Refuse an existing recovery file before creating its companion keypair.
    // create_new below still protects both paths against a concurrent writer.
    match fs::symlink_metadata(&recovery_path) {
        Ok(_) => bail!("Recovery file already exists: {}", recovery_path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Cannot inspect recovery file destination"),
    }
    save_keypair(path, &wallet.keypair)?;
    let recovery = Zeroizing::new(format!(
        "Seed phrase: {}\nDerivation path: {}\nAddress: {}\n",
        wallet.phrase.as_str(),
        wallet.derivation_path,
        wallet.keypair.pubkey(),
    ));
    write_secret_file(&recovery_path, |file| file.write_all(recovery.as_bytes())).with_context(
        || {
            format!(
                "Keypair saved to {}; failed to save mnemonic recovery file {}",
                path.display(),
                recovery_path.display(),
            )
        },
    )
}

fn parse_mnemonic_text(input: &str, passphrase: &str) -> Result<MnemonicWallet> {
    let mut lines = input.lines().map(str::trim).filter(|line| !line.is_empty());
    let phrase = lines
        .next()
        .and_then(|line| line.strip_prefix("Seed phrase:"))
        .ok_or_else(|| anyhow!("Recovery file is missing its seed phrase"))?
        .trim();
    let path = lines
        .next()
        .and_then(|line| line.strip_prefix("Derivation path:"))
        .ok_or_else(|| anyhow!("Recovery file is missing its derivation path"))?
        .trim();
    let address = lines
        .next()
        .and_then(|line| line.strip_prefix("Address:"))
        .ok_or_else(|| anyhow!("Recovery file is missing its public address"))?
        .trim();
    if lines.next().is_some() {
        bail!("Recovery file has unexpected fields");
    }
    let wallet = wallet_from_phrase(phrase, path, passphrase)?;
    verify_address(wallet.keypair.pubkey(), address)?;
    Ok(wallet)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MnemonicBackup {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(deserialize_with = "deserialize_phrase")]
    phrase: Zeroizing<String>,
    derivation_path: String,
    address: String,
}

fn deserialize_phrase<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}

pub fn recover_mnemonic(path: &Path, passphrase: &str) -> Result<MnemonicWallet> {
    let raw = Zeroizing::new(
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read mnemonic backup {}", path.display()))?,
    );
    match parse_wallet(&raw, "m/44'/501'/0'", passphrase)? {
        WalletMaterial::Mnemonic(wallet) => Ok(wallet),
        _ => bail!("The backup does not contain a mnemonic"),
    }
}

fn write_secret_file(
    path: &Path,
    write_contents: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).with_context(|| {
        format!(
            "Cannot create secret file {}; destination must not exist",
            path.display()
        )
    })?;
    let result = write_contents(&mut file)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = result {
        if let Err(cleanup_error) = fs::remove_file(path) {
            return Err(error).with_context(|| {
                format!(
                    "Failed to save {}; could not remove the incomplete secret file: {cleanup_error}",
                    path.display()
                )
            });
        }
        return Err(error).with_context(|| format!("Failed to save {}", path.display()));
    }
    Ok(())
}
