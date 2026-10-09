use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::{
    app::{
        cli::{
            ProgramArgs, ProgramCommand, ProgramDeployArgs, ProgramSignerOptions,
            ProgramSvpiOptions, SolanaRpcOptions,
        },
        commands::commitment_from_arg,
        defaults::resolve_solana_rpc_defaults,
    },
    config::{app_config::WmgrConfig, clusters::get_cluster_config},
    infra::{keys::program::ProgramWallets, program},
    utils::prompt::prompt,
};

pub async fn handle_program(args: ProgramArgs, cfg: &WmgrConfig) -> Result<()> {
    match args.command {
        ProgramCommand::Deploy(args) => deploy(args, cfg, false).await,
        ProgramCommand::Upgrade(args) => deploy(args, cfg, true).await,
        ProgramCommand::Show(args) => {
            let (url, commitment) = connection(args.rpc, cfg)?;
            let name = select_name(args.program, "Program wallet in SVPI", None)?;
            let mut wallets = wallets(&args.vault, cfg);
            wallets.load(&name)?;
            let info = program::show(&url, commitment, wallets.address(&name)?).await?;
            let mut output = io::stdout().lock();
            writeln!(output, "Program: {}", info.program)?;
            writeln!(output, "Program data: {}", info.program_data)?;
            match info.authority {
                Some(authority) => writeln!(output, "Authority: {authority}")?,
                None => writeln!(output, "Authority: none (immutable)")?,
            }
            writeln!(output, "Last deployed slot: {}", info.last_deployed_slot)?;
            writeln!(output, "Data length: {} bytes", info.data_len)?;
            writeln!(output, "Program data balance: {} lamports", info.lamports)?;
            Ok(())
        }
    }
}

async fn deploy(args: ProgramDeployArgs, cfg: &WmgrConfig, upgrade: bool) -> Result<()> {
    let (url, commitment) = connection(args.target.rpc, cfg)?;
    let file = match args.file {
        Some(path) => path,
        None => PathBuf::from(select_name(None, "Program file (.so)", None)?),
    };
    check_program_file(&file)?;
    let buffer = args.buffer.as_deref().map(parse_address).transpose()?;
    let name = select_name(args.target.program, "Program wallet in SVPI", None)?;
    let (payer, authority) = select_signers(args.signers, cfg)?;
    let mut wallets = wallets(&args.target.vault, cfg);
    wallets.load(&name)?;
    if !upgrade {
        wallets.signer(&name)?;
    }
    wallets.load(&payer)?;
    wallets.signer(&payer)?;
    wallets.load(&authority)?;
    let program_signer = if upgrade {
        None
    } else {
        Some(wallets.signer(&name)?)
    };
    print_roles(&wallets, &name, &payer, &authority)?;
    let result = program::deploy(
        &url,
        commitment,
        program::ProgramDeployment {
            file: &file,
            payer: wallets.signer(&payer)?,
            program: wallets.address(&name)?,
            program_signer,
            authority: wallets.signer(&authority)?,
            buffer,
            compute_unit_price: args.compute_unit_price,
        },
    )
    .await?;
    writeln!(io::stdout(), "{result}")?;
    Ok(())
}

fn connection(
    opts: SolanaRpcOptions,
    cfg: &WmgrConfig,
) -> Result<(String, solana_commitment_config::CommitmentConfig)> {
    let opts = resolve_solana_rpc_defaults(opts, cfg);
    let cluster = get_cluster_config(&opts.cluster, opts.rpc.as_deref())?;
    writeln!(
        io::stdout(),
        "Using cluster: {}, RPC: {}",
        cluster.name,
        cluster.rpc_url
    )?;
    Ok((cluster.rpc_url, commitment_from_arg(opts.commitment)))
}

fn wallets<'a>(opts: &'a ProgramSvpiOptions, cfg: &'a WmgrConfig) -> ProgramWallets<'a> {
    ProgramWallets::new(
        opts.svpi_file
            .as_deref()
            .or_else(|| cfg.svpi_file.as_deref().map(Path::new)),
        opts.svpi_cmd
            .as_deref()
            .or_else(|| cfg.svpi_cmd.as_deref().map(Path::new)),
        opts.svpi_pass.as_deref(),
    )
}

fn select_signers(opts: ProgramSignerOptions, cfg: &WmgrConfig) -> Result<(String, String)> {
    let payer = select_name(opts.payer, "Payer wallet in SVPI", cfg.svpi_name.as_deref())?;
    let authority = select_name(opts.authority, "Authority wallet in SVPI", Some(&payer))?;
    Ok((payer, authority))
}

fn select_name(value: Option<String>, label: &str, default: Option<&str>) -> Result<String> {
    let value = match value {
        Some(value) => value.trim().to_owned(),
        None => {
            let message = match default {
                Some(default) => format!("{label} [{default}]:"),
                None => format!("{label}:"),
            };
            let value = prompt(&message)?;
            if value.is_empty() {
                default.unwrap_or_default().to_owned()
            } else {
                value
            }
        }
    };
    if value.is_empty() {
        bail!("{label} is required");
    }
    Ok(value)
}

fn parse_address(value: &str) -> Result<Pubkey> {
    Pubkey::from_str(value.trim()).context("Invalid deployment buffer address")
}

fn check_program_file(path: &Path) -> Result<()> {
    if !fs::metadata(path)
        .context("Failed to inspect program file")?
        .is_file()
    {
        bail!("Program file must be a regular file");
    }
    let mut file = File::open(path).context("Failed to open program file")?;
    if !file.metadata()?.is_file() {
        bail!("Program file must be a regular file");
    }
    let mut magic = [0; 4];
    file.read_exact(&mut magic)
        .context("Program file is empty or truncated")?;
    if magic != *b"\x7fELF" {
        bail!("Program file must be a compiled Solana ELF (.so)");
    }
    Ok(())
}

fn print_roles(
    wallets: &ProgramWallets<'_>,
    program: &str,
    payer: &str,
    authority: &str,
) -> Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "Program: {program} ({})", wallets.address(program)?)?;
    writeln!(
        output,
        "Payer: {payer} ({})",
        wallets.signer(payer)?.pubkey()
    )?;
    writeln!(
        output,
        "Authority: {authority} ({})",
        wallets.signer(authority)?.pubkey()
    )?;
    Ok(())
}
