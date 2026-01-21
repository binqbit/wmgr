use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "wmgr",
    version,
    about = "Wallet Manager CLI for Solana and EVM networks"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Send tokens on Solana or EVM
    Send(SendCommand),
    /// Get SOL and USDC balances for an address or resolved wallet
    Balance(BalanceArgs),
}

#[derive(Args, Debug)]
pub struct SendCommand {
    #[command(subcommand)]
    pub kind: SendKind,
}

#[derive(Subcommand, Debug)]
pub enum SendKind {
    /// Send native SOL
    Sol(SendSolArgs),
    /// Send USDC (SPL token)
    Usdc(SendUsdcArgs),
    /// Send native ETH (or network native coin)
    Eth(SendEthArgs),
    /// Send ERC-20/BEP-20/PIP-20 token
    Erc20(SendErc20Args),
}

#[derive(Args, Debug, Clone)]
pub struct SendSolArgs {
    #[arg(value_name = "TO", help = "Recipient Solana address (base58)")]
    pub to: String,
    #[arg(value_name = "AMOUNT", help = "Amount of SOL to send")]
    pub amount: String,
    #[command(flatten)]
    pub key: SolanaKeyOptions,
    #[command(flatten)]
    pub rpc: SolanaRpcOptions,
}

#[derive(Args, Debug, Clone)]
pub struct SendUsdcArgs {
    #[arg(value_name = "TO", help = "Recipient Solana address (base58)")]
    pub to: String,
    #[arg(value_name = "AMOUNT", help = "Amount of USDC to send")]
    pub amount: String,
    #[command(flatten)]
    pub key: SolanaKeyOptions,
    #[command(flatten)]
    pub rpc: SolanaRpcOptions,
}

#[derive(Args, Debug, Clone)]
pub struct SendEthArgs {
    #[arg(value_name = "TO", help = "Recipient EVM address")]
    pub to: String,
    #[arg(value_name = "AMOUNT", help = "Amount of native token to send")]
    pub amount: String,
    #[command(flatten)]
    pub key: EvmKeyOptions,
    #[command(flatten)]
    pub tx: EvmTxOptions,
}

#[derive(Args, Debug, Clone)]
pub struct SendErc20Args {
    #[arg(value_name = "TOKEN", help = "ERC-20 token contract address")]
    pub token: String,
    #[arg(value_name = "TO", help = "Recipient EVM address")]
    pub to: String,
    #[arg(value_name = "AMOUNT", help = "Token amount to send")]
    pub amount: String,
    #[arg(long, value_name = "DECIMALS", help = "Override token decimals")]
    pub decimals: Option<u8>,
    #[command(flatten)]
    pub key: EvmKeyOptions,
    #[command(flatten)]
    pub tx: EvmTxOptions,
}

#[derive(Args, Debug, Clone)]
pub struct BalanceArgs {
    #[arg(value_name = "ADDRESS", help = "Solana address (base58)")]
    pub address: Option<String>,
    #[command(flatten)]
    pub key: SolanaKeyOptions,
    #[command(flatten)]
    pub rpc: SolanaRpcOptions,
}

#[derive(Args, Debug, Clone)]
pub struct SolanaKeyOptions {
    #[arg(long, value_name = "PATH", help = "Solana keypair file (JSON array)")]
    pub keyfile: Option<PathBuf>,
    #[arg(long, value_name = "MNEMONIC", help = "BIP39 seed phrase")]
    pub seed: Option<String>,
    #[arg(long, value_name = "PATH", help = "BIP44 derivation path")]
    pub path: Option<String>,
    #[arg(
        long,
        value_name = "PROFILE",
        default_value = "trustwallet",
        help = "Mnemonic profile (trustwallet|phantom|solflare|solana_cli)"
    )]
    pub mnemo: String,
    #[arg(long, value_name = "PASS", help = "BIP39 passphrase")]
    pub seed_passphrase: Option<String>,
    #[arg(long, help = "Use SVPI to fetch mnemonic")]
    pub svpi: bool,
    #[arg(long, value_name = "NAME", help = "SVPI wallet name")]
    pub svpi_name: Option<String>,
    #[arg(long, value_name = "PATH", help = "SVPI file mode path")]
    pub svpi_file: Option<PathBuf>,
    #[arg(
        long = "svpi_cmd",
        value_name = "PATH",
        help = "SVPI command path (defaults to svpi)",
        alias = "svpi-cmd"
    )]
    pub svpi_cmd: Option<PathBuf>,
    #[arg(long, value_name = "PASS", help = "SVPI password (optional, otherwise prompt)")]
    pub svpi_pass: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct EvmKeyOptions {
    #[arg(long, value_name = "HEX", help = "EVM private key (with or without 0x)")]
    pub privkey: Option<String>,
    #[arg(long, value_name = "PATH", help = "File containing EVM private key")]
    pub privkey_file: Option<PathBuf>,
    #[arg(long, value_name = "MNEMONIC", help = "BIP39 seed phrase")]
    pub seed: Option<String>,
    #[arg(long, value_name = "PATH", help = "BIP44 derivation path")]
    pub path: Option<String>,
    #[arg(long, value_name = "PASS", help = "BIP39 passphrase")]
    pub seed_passphrase: Option<String>,
    #[arg(long, help = "Use SVPI to fetch mnemonic")]
    pub svpi: bool,
    #[arg(long, value_name = "NAME", help = "SVPI wallet name")]
    pub svpi_name: Option<String>,
    #[arg(long, value_name = "PATH", help = "SVPI file mode path")]
    pub svpi_file: Option<PathBuf>,
    #[arg(
        long = "svpi_cmd",
        value_name = "PATH",
        help = "SVPI command path (defaults to svpi)",
        alias = "svpi-cmd"
    )]
    pub svpi_cmd: Option<PathBuf>,
    #[arg(long, value_name = "PASS", help = "SVPI password (optional, otherwise prompt)")]
    pub svpi_pass: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct SolanaRpcOptions {
    #[arg(long, value_name = "CLUSTER", default_value = "mainnet-beta")]
    pub cluster: String,
    #[arg(long, value_name = "URL", help = "Custom Solana RPC URL")]
    pub rpc: Option<String>,
    #[arg(long, value_enum, default_value_t = CommitmentArg::Confirmed)]
    pub commitment: CommitmentArg,
}

#[derive(Args, Debug, Clone)]
pub struct EvmTxOptions {
    #[arg(long, value_name = "NETWORK", default_value = "mainnet")]
    pub network: String,
    #[arg(long, value_name = "URL", help = "Custom EVM RPC URL")]
    pub rpc: Option<String>,
    #[arg(long, value_name = "GWEI", help = "Gas price in gwei")]
    pub gas_price: Option<String>,
    #[arg(long, value_name = "NUMBER", help = "Gas limit override")]
    pub gas_limit: Option<u64>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CommitmentArg {
    Processed,
    Confirmed,
    Finalized,
}
