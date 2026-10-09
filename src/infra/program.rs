use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    sync::Arc,
};

use anyhow::{anyhow, bail, Context, Result};
use solana_cli::{
    cli::{CliConfig, ProcessResult},
    program::{process_program_subcommand, ProgramCliCommand},
};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_loader_v3_interface::{get_program_data_address, state::UpgradeableLoaderState};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signer::{null_signer::NullSigner, Signer},
};

use super::keys::generate::random_keypair;

const LOADER: Pubkey = solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
const MAX_PROGRAM_BYTES: usize =
    10 * 1024 * 1024 - UpgradeableLoaderState::size_of_programdata_metadata();

/// All signing keys are borrowed directly from the wallet resolver.
pub struct ProgramDeployment<'a> {
    pub file: &'a Path,
    pub payer: &'a Keypair,
    pub program: Pubkey,
    /// Some means an initial deployment; None means an existing program upgrade.
    pub program_signer: Option<&'a Keypair>,
    pub authority: &'a Keypair,
    pub buffer: Option<Pubkey>,
    pub compute_unit_price: Option<u64>,
}

pub struct ProgramInfo {
    pub program: Pubkey,
    pub program_data: Pubkey,
    pub authority: Option<Pubkey>,
    pub last_deployed_slot: u64,
    pub data_len: usize,
    pub lamports: u64,
}

pub async fn deploy(
    rpc_url: &str,
    commitment: CommitmentConfig,
    options: ProgramDeployment<'_>,
) -> Result<String> {
    let path = fs::canonicalize(options.file).context("Cannot open the program file")?;
    let location = path
        .to_str()
        .ok_or_else(|| anyhow!("Program file path must be valid UTF-8"))?;
    let metadata = fs::metadata(&path).context("Cannot inspect the program file")?;
    if !metadata.is_file() || !(64..=MAX_PROGRAM_BYTES as u64).contains(&metadata.len()) {
        bail!("Program must be a regular ELF file between 64 and {MAX_PROGRAM_BYTES} bytes");
    }
    let program_len = usize::try_from(metadata.len()).context("Program file is too large")?;
    let mut magic = [0u8; 4];
    File::open(&path)?.read_exact(&mut magic)?;
    if magic != *b"\x7fELF" {
        bail!("Program file is not an ELF binary");
    }
    if let Some(signer) = options.program_signer {
        if signer.pubkey() != options.program {
            bail!("Program signer does not match the requested program address");
        }
    }

    let new_buffer = if options.buffer.is_none() {
        Some(random_keypair()?)
    } else {
        None
    };
    let buffer_address = match (&new_buffer, options.buffer) {
        (Some(keypair), _) => keypair.pubkey(),
        (_, Some(address)) => address,
        _ => unreachable!(),
    };
    // The authority can resume or reclaim an initialized buffer
    // without the temporary account keypair.
    writeln!(io::stdout(), "Buffer: {buffer_address}")?;

    let rpc = Arc::new(RpcClient::new_with_commitment(
        rpc_url.to_owned(),
        commitment,
    ));
    let program_info = if options.program_signer.is_some() {
        if let Some(account) = rpc
            .get_account_with_commitment(&options.program, commitment)
            .await?
            .value
        {
            if account.owner == LOADER && account.executable {
                bail!(
                    "Program address {} already exists; use program upgrade",
                    options.program
                );
            }
            bail!(
                "Program address {} is already occupied by another account",
                options.program
            );
        }
        None
    } else {
        let info = read_program_info(&rpc, commitment, options.program).await?;
        require_authority(&info, options.authority)?;
        Some(info)
    };

    if options.buffer.is_some() {
        check_buffer(
            &rpc,
            commitment,
            buffer_address,
            options.authority.pubkey(),
            program_len,
        )
        .await?;
        // Agave includes extension in the initial transaction, whose signer
        // slot is the buffer keypair. A resumed public-only buffer cannot sign
        // that transaction. Fresh buffers support automatic extension normally.
        if program_info
            .as_ref()
            .is_some_and(|info| info.data_len < program_len)
        {
            bail!("The program needs more space; retry without --buffer to use a fresh buffer");
        }
    }

    let resumed_buffer = NullSigner::new(&buffer_address);
    let buffer_signer: &dyn Signer = match new_buffer.as_ref() {
        Some(keypair) => keypair,
        None => &resumed_buffer,
    };
    let mut signers: Vec<&dyn Signer> = vec![options.payer, options.authority, buffer_signer];
    let program_signer_index = options.program_signer.map(|signer| {
        signers.push(signer);
        signers.len() - 1
    });
    let config = cli_config(rpc_url, commitment, rpc.clone(), signers);
    let command = ProgramCliCommand::Deploy {
        program_location: Some(location.to_owned()),
        fee_payer_signer_index: 0,
        program_signer_index,
        program_pubkey: Some(options.program),
        buffer_signer_index: Some(2),
        buffer_pubkey: Some(buffer_address),
        upgrade_authority_signer_index: 1,
        is_final: false,
        max_len: None,
        skip_fee_check: false,
        compute_unit_price: options.compute_unit_price,
        max_sign_attempts: 5,
        auto_extend: true,
        use_rpc: true,
        skip_feature_verification: false,
    };
    writeln!(
        io::stderr(),
        "{} program ({} bytes)...",
        if options.program_signer.is_some() {
            "Deploying"
        } else {
            "Upgrading"
        },
        program_len
    )?;
    command_result(process_program_subcommand(rpc, &config, &command).await).map_err(|error| {
        anyhow!(
            "Deployment failed: {error}. Buffer {buffer_address} remains controlled by {} if initialized. \
             Resume with --buffer {buffer_address} or reclaim it with program close-buffer",
            options.authority.pubkey()
        )
    })
}

pub async fn show(
    rpc_url: &str,
    commitment: CommitmentConfig,
    program: Pubkey,
) -> Result<ProgramInfo> {
    let rpc = RpcClient::new_with_commitment(rpc_url.to_owned(), commitment);
    read_program_info(&rpc, commitment, program).await
}

pub async fn set_authority(
    rpc_url: &str,
    commitment: CommitmentConfig,
    program: Pubkey,
    payer: &Keypair,
    authority: &Keypair,
    new_authority: &Keypair,
) -> Result<String> {
    if authority.pubkey() == new_authority.pubkey() {
        bail!("New upgrade authority is already the current authority");
    }
    let rpc = Arc::new(RpcClient::new_with_commitment(
        rpc_url.to_owned(),
        commitment,
    ));
    let info = read_program_info(&rpc, commitment, program).await?;
    require_authority(&info, authority)?;
    let config = cli_config(
        rpc_url,
        commitment,
        rpc.clone(),
        vec![payer, authority, new_authority],
    );
    let command = ProgramCliCommand::SetUpgradeAuthorityChecked {
        program_pubkey: program,
        upgrade_authority_index: 1,
        new_upgrade_authority_index: 2,
        sign_only: false,
        dump_transaction_message: false,
        blockhash_query: Default::default(),
    };
    command_result(process_program_subcommand(rpc, &config, &command).await)
}

pub async fn close_buffer(
    rpc_url: &str,
    commitment: CommitmentConfig,
    buffer: Pubkey,
    payer: &Keypair,
    authority: &Keypair,
) -> Result<String> {
    let rpc = Arc::new(RpcClient::new_with_commitment(
        rpc_url.to_owned(),
        commitment,
    ));
    check_buffer(&rpc, commitment, buffer, authority.pubkey(), 0).await?;
    let config = cli_config(rpc_url, commitment, rpc.clone(), vec![payer, authority]);
    let command = ProgramCliCommand::Close {
        account_pubkey: Some(buffer),
        recipient_pubkey: payer.pubkey(),
        authority_index: 1,
        use_lamports_unit: true,
        // Agave's guard still refuses program closure if the account changes
        // after our explicit Buffer-state check.
        bypass_warning: false,
    };
    command_result(process_program_subcommand(rpc, &config, &command).await)
}

fn cli_config<'a>(
    rpc_url: &str,
    commitment: CommitmentConfig,
    rpc: Arc<RpcClient>,
    signers: Vec<&'a dyn Signer>,
) -> CliConfig<'a> {
    let mut config = CliConfig {
        json_rpc_url: rpc_url.to_owned(),
        commitment,
        signers,
        rpc_client: Some(rpc),
        ..CliConfig::default()
    };
    config.send_transaction_config.preflight_commitment = Some(commitment.commitment);
    config
}

fn command_result(result: ProcessResult) -> Result<String> {
    result.map_err(|error| anyhow!("{error}"))
}

fn require_authority(info: &ProgramInfo, signer: &Keypair) -> Result<()> {
    match info.authority {
        None => bail!("Program is immutable and has no upgrade authority"),
        Some(authority) if authority != signer.pubkey() => {
            bail!("Program upgrade authority does not match the supplied signer")
        }
        Some(_) => Ok(()),
    }
}

async fn read_program_info(
    rpc: &RpcClient,
    commitment: CommitmentConfig,
    program: Pubkey,
) -> Result<ProgramInfo> {
    let account = rpc
        .get_account_with_commitment(&program, commitment)
        .await?
        .value
        .ok_or_else(|| anyhow!("Program {program} was not found"))?;
    if account.owner != LOADER || !account.executable {
        bail!("Account {program} is not an executable upgradeable Solana program");
    }
    let state: UpgradeableLoaderState =
        bincode::deserialize(&account.data).context("Invalid program account state")?;
    let UpgradeableLoaderState::Program {
        programdata_address,
    } = state
    else {
        bail!("Account {program} does not contain an upgradeable program");
    };
    if programdata_address != get_program_data_address(&program) {
        bail!("ProgramData address does not match the program");
    }
    let data = rpc
        .get_account_with_commitment(&programdata_address, commitment)
        .await?
        .value
        .ok_or_else(|| anyhow!("ProgramData account was not found; the program may be closed"))?;
    if data.owner != LOADER || data.executable {
        bail!("Invalid ProgramData account owner or executable flag");
    }
    let state: UpgradeableLoaderState =
        bincode::deserialize(&data.data).context("Invalid ProgramData account state")?;
    let UpgradeableLoaderState::ProgramData {
        slot,
        upgrade_authority_address,
    } = state
    else {
        bail!("ProgramData account is closed or invalid");
    };
    let data_len = data
        .data
        .len()
        .checked_sub(UpgradeableLoaderState::size_of_programdata_metadata())
        .ok_or_else(|| anyhow!("ProgramData account is truncated"))?;
    Ok(ProgramInfo {
        program,
        program_data: programdata_address,
        authority: upgrade_authority_address,
        last_deployed_slot: slot,
        data_len,
        lamports: data.lamports,
    })
}

async fn check_buffer(
    rpc: &RpcClient,
    commitment: CommitmentConfig,
    buffer: Pubkey,
    authority: Pubkey,
    program_len: usize,
) -> Result<()> {
    let account = rpc
        .get_account_with_commitment(&buffer, commitment)
        .await?
        .value
        .ok_or_else(|| anyhow!("Buffer {buffer} was not found; retry without --buffer"))?;
    if account.owner != LOADER || account.executable {
        bail!("Account {buffer} is not an upgradeable loader buffer");
    }
    let state: UpgradeableLoaderState =
        bincode::deserialize(&account.data).context("Invalid buffer account state")?;
    let UpgradeableLoaderState::Buffer { authority_address } = state else {
        bail!("Account {buffer} is not a buffer; program accounts cannot be closed here");
    };
    if authority_address != Some(authority) {
        bail!("Buffer authority does not match the supplied signer");
    }
    if account.data.len() < UpgradeableLoaderState::size_of_buffer(program_len) {
        bail!("Buffer is too small for this program; retry without --buffer");
    }
    Ok(())
}
