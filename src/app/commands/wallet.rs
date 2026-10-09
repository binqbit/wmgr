use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use zeroize::Zeroizing;

use crate::{
    app::cli::{
        WalletArgs, WalletBackend, WalletCommand, WalletFormat, WalletSaveOptions,
        WalletSvpiOptions,
    },
    config::{app_config::WmgrConfig, mnemonics::get_mnemonic_profile},
    core::vanity::AddressPattern,
    infra::keys::{
        generate::{self, WalletMaterial},
        svpi,
        vanity::{self, Backend, SearchOptions, SearchProgress},
    },
    utils::prompt::{prompt, prompt_hidden},
};

pub fn handle_wallet(args: WalletArgs, cfg: &WmgrConfig) -> Result<()> {
    match args.command {
        WalletCommand::New(args) => {
            check_output_directory(&args.save.out_dir)?;
            let material = match args.format {
                WalletFormat::Keypair => {
                    if args.words.is_some() || args.path.is_some() || args.passphrase {
                        bail!("--words, --path, and --passphrase require --format mnemonic");
                    }
                    WalletMaterial::Keypair(generate::random_keypair()?)
                }
                WalletFormat::Mnemonic => {
                    let words = args.words.as_deref().unwrap_or("24").parse()?;
                    let passphrase = read_passphrase(args.passphrase, true)?;
                    WalletMaterial::Mnemonic(generate::mnemonic_wallet(
                        words,
                        derivation_path(args.path.as_deref()),
                        &passphrase,
                    )?)
                }
            };
            save_wallet(&material, &args.save, cfg)?;
        }
        WalletCommand::Recover(args) => {
            check_output_directory(&args.save.out_dir)?;
            let passphrase = read_passphrase(args.passphrase, false)?;
            let wallet = generate::recover_mnemonic(&args.from, &passphrase)?;
            save_wallet(&WalletMaterial::Keypair(wallet.keypair), &args.save, cfg)?;
        }
        WalletCommand::Import(args) => {
            check_output_directory(&args.save.out_dir)?;
            let raw = Zeroizing::new(
                fs::read_to_string(&args.from).context("Failed to read wallet import file")?,
            );
            let passphrase = read_passphrase(args.passphrase, false)?;
            let material =
                generate::parse_wallet(&raw, derivation_path(args.path.as_deref()), &passphrase)?;
            save_wallet(&material, &args.save, cfg)?;
        }
        WalletCommand::Export(args) => {
            check_output_directory(&args.out_dir)?;
            let name = match args.svpi.svpi_name.as_deref().or(cfg.svpi_name.as_deref()) {
                Some(name) => name.to_owned(),
                None => prompt("SVPI wallet name:")?,
            };
            let password = svpi_password(&args.svpi, false)?;
            let data = svpi::get_data_from_svpi(
                &name,
                &password,
                args.svpi
                    .svpi_file
                    .as_deref()
                    .or_else(|| cfg.svpi_file.as_deref().map(Path::new)),
                args.svpi
                    .svpi_cmd
                    .as_deref()
                    .or_else(|| cfg.svpi_cmd.as_deref().map(Path::new)),
            )?;
            let raw = Zeroizing::new(data.data);
            let passphrase = read_passphrase(args.passphrase, false)?;
            let material =
                generate::parse_wallet(&raw, derivation_path(args.path.as_deref()), &passphrase)?;
            save_file(&material, &args.out_dir)?;
        }
        WalletCommand::Vanity(args) => {
            let pattern = AddressPattern::new(
                args.prefix.as_deref().unwrap_or_default(),
                args.suffix.as_deref().unwrap_or_default(),
                args.ignore_case,
            )?;
            check_output_directory(&args.save.out_dir)?;
            let options = SearchOptions {
                backend: match args.backend {
                    WalletBackend::Auto => Backend::Auto,
                    WalletBackend::Cpu => Backend::Cpu,
                    WalletBackend::Gpu => Backend::Gpu,
                },
                threads: args.threads.map(|v| v.get()).unwrap_or_else(|| {
                    std::thread::available_parallelism()
                        .map(|v| v.get())
                        .unwrap_or(1)
                }),
                max_attempts: args.max_attempts.map(|v| v.get()),
            };
            let result = vanity::search(&pattern, &options, |event| match event {
                SearchProgress::Backend(name) => {
                    let _ = writeln!(io::stderr(), "Searching with {name}");
                }
                SearchProgress::Fallback(reason) => {
                    let _ = writeln!(io::stderr(), "GPU unavailable: {reason}; using CPU");
                }
                SearchProgress::Attempts(count) => {
                    let _ = writeln!(io::stderr(), "Searched {count} candidates");
                }
            })?;
            save_wallet(&WalletMaterial::Keypair(result.keypair), &args.save, cfg)?;
            writeln!(
                io::stdout(),
                "Backend: {}; candidates checked: {}",
                result.backend,
                result.attempts
            )?;
        }
    }
    Ok(())
}

fn derivation_path(path: Option<&str>) -> &str {
    path.unwrap_or_else(|| get_mnemonic_profile("trustwallet"))
}

fn print_wallet(material: &WalletMaterial) -> Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "Type: {}", material.kind())?;
    writeln!(output, "Address: {}", material.address())?;
    if let WalletMaterial::Mnemonic(wallet) = material {
        writeln!(output, "Derivation path: {}", wallet.derivation_path)?;
    }
    Ok(())
}

fn save_wallet(
    material: &WalletMaterial,
    opts: &WalletSaveOptions,
    cfg: &WmgrConfig,
) -> Result<()> {
    let notice = if !opts.no_svpi {
        match save_to_svpi(material, &opts.svpi, cfg) {
            Ok(Some(name)) => {
                print_wallet(material)?;
                writeln!(io::stdout(), "SVPI entry: {name}")?;
                return Ok(());
            }
            Ok(None) => Some("SVPI save skipped; saving a local backup".to_owned()),
            Err(error) => Some(format!("SVPI save failed: {error}; saving a local backup")),
        }
    } else {
        None
    };
    save_file(material, &opts.out_dir)?;
    if let Some(notice) = notice {
        let _ = writeln!(io::stderr(), "{notice}");
    }
    Ok(())
}

fn save_to_svpi(
    material: &WalletMaterial,
    opts: &WalletSvpiOptions,
    cfg: &WmgrConfig,
) -> Result<Option<String>> {
    // A configured default names an existing wallet. New records use the name
    // explicitly supplied by the user, never an existing default implicitly.
    let name = match &opts.svpi_name {
        Some(name) => name.trim().to_owned(),
        None => prompt("SVPI wallet name (Enter to save to file):")?,
    };
    if name.is_empty() {
        return Ok(None);
    }
    let file = opts
        .svpi_file
        .as_deref()
        .or_else(|| cfg.svpi_file.as_deref().map(Path::new));
    let command = opts
        .svpi_cmd
        .as_deref()
        .or_else(|| cfg.svpi_cmd.as_deref().map(Path::new));
    svpi::check_new_wallet(&name, file, command)?;
    let password = svpi_password(opts, true)?;
    if password.is_empty() {
        return Ok(None);
    }
    let data = generate::serialize_wallet(material)?;
    svpi::set_data_in_svpi(&name, &data, &password, file, command)?;
    Ok(Some(name))
}

fn svpi_password(opts: &WalletSvpiOptions, confirm: bool) -> Result<Zeroizing<String>> {
    if let Some(password) = &opts.svpi_pass {
        return Ok(Zeroizing::new(password.clone()));
    }
    let password = Zeroizing::new(prompt_hidden(if confirm {
        "SVPI password (Enter to save to file):"
    } else {
        "SVPI password:"
    })?);
    if confirm && !password.is_empty() {
        let repeated = Zeroizing::new(prompt_hidden("Repeat SVPI password:")?);
        if *password != *repeated {
            bail!("SVPI passwords do not match");
        }
    }
    Ok(password)
}

fn save_file(material: &WalletMaterial, directory: &Path) -> Result<()> {
    let extension = if matches!(material, WalletMaterial::PublicKey(_)) {
        "txt"
    } else {
        "json"
    };
    let path = directory.join(format!("{}.{extension}", material.address()));
    generate::save_wallet(&path, material)?;
    print_wallet(material)?;
    writeln!(io::stdout(), "File: {}", path.display())?;
    if matches!(material, WalletMaterial::Mnemonic(_)) {
        writeln!(
            io::stdout(),
            "Recovery file: {}",
            path.with_extension("txt").display()
        )?;
    }
    Ok(())
}

fn check_output_directory(directory: &Path) -> Result<()> {
    if !directory.is_dir() {
        bail!(
            "Output directory must already exist: {}",
            directory.display()
        );
    }
    Ok(())
}

fn read_passphrase(enabled: bool, confirm: bool) -> Result<Zeroizing<String>> {
    if !enabled {
        return Ok(Zeroizing::new(String::new()));
    }
    let passphrase = Zeroizing::new(
        rpassword::prompt_password("BIP39 passphrase (not saved in backup): ")
            .context("Failed to read BIP39 passphrase")?,
    );
    if passphrase.is_empty() {
        bail!("Passphrase cannot be empty when --passphrase is set");
    }
    if confirm {
        let repeated = Zeroizing::new(
            rpassword::prompt_password("Repeat BIP39 passphrase: ")
                .context("Failed to confirm BIP39 passphrase")?,
        );
        if *passphrase != *repeated {
            bail!("Passphrases do not match");
        }
    }
    Ok(passphrase)
}
