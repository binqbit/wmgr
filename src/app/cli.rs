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
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Get SOL and USDC balances for an address or resolved wallet
    Balance(BalanceArgs),
    /// Send tokens on Solana or EVM
    Send(SendCommand),
    /// Show SOL/USDC price from Raydium pool
    Price(PriceArgs),
    /// Buy SOL/USDC on Raydium
    Buy(TradeArgs),
    /// Sell SOL/USDC on Raydium
    Sell(TradeArgs),
    /// Start interactive mode (REPL)
    Repl,
    /// Manage saved defaults (.wmgr)
    Config(ConfigArgs),
    /// Print SHA256 hash of this executable (and SVPI when enabled)
    SelfHash,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current config values
    Show,
    /// Update config values
    Set(ConfigSetArgs),
    /// Reset config to defaults
    Reset,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigSetArgs {
    #[arg(
        long,
        help = "Enable SVPI mode (default key source)",
        conflicts_with = "no_svpi"
    )]
    pub svpi: bool,
    #[arg(long = "no-svpi", help = "Disable SVPI mode", conflicts_with = "svpi")]
    pub no_svpi: bool,
    #[arg(
        long,
        value_name = "NAME",
        help = "Default SVPI wallet name",
        alias = "svpi_name"
    )]
    pub svpi_name: Option<String>,
    #[arg(
        long,
        value_name = "CLUSTER",
        help = "Default Solana cluster (default mainnet-beta)",
        alias = "solana_cluster",
        alias = "solana-cluster"
    )]
    pub cluster: Option<String>,
    #[arg(
        long,
        value_enum,
        value_name = "LEVEL",
        help = "Default Solana commitment (default confirmed)",
        alias = "solana_commitment",
        alias = "solana-commitment"
    )]
    pub commitment: Option<CommitmentArg>,
    #[arg(
        long,
        value_name = "PERCENT",
        help = "Default slippage percent for buy/sell (default 0.1)"
    )]
    pub slippage: Option<f64>,
    #[arg(
        long,
        value_enum,
        value_name = "NETWORK",
        help = "Default EVM network (default mainnet)",
        alias = "evm_network",
        alias = "evm-network"
    )]
    pub network: Option<EvmNetworkArg>,
    #[arg(
        long,
        value_name = "URL",
        help = "Default RPC URL (set both Solana and EVM unless --cluster or --network is present)",
        alias = "solana_rpc",
        alias = "solana-rpc",
        alias = "evm_rpc",
        alias = "evm-rpc"
    )]
    pub rpc: Option<String>,
    #[arg(
        long,
        value_name = "GWEI",
        help = "Default EVM gas price in gwei",
        alias = "evm_gas_price",
        alias = "evm-gas-price"
    )]
    pub gas_price: Option<String>,
    #[arg(
        long,
        value_name = "NUMBER",
        help = "Default EVM gas limit override",
        alias = "evm_gas_limit",
        alias = "evm-gas-limit"
    )]
    pub gas_limit: Option<u64>,
    #[arg(
        long = "svpi_cmd",
        value_name = "PATH",
        help = "SVPI command path (defaults to svpi)",
        alias = "svpi-cmd"
    )]
    pub svpi_cmd: Option<PathBuf>,
    #[arg(
        long,
        value_name = "PATH",
        help = "SVPI file mode path",
        alias = "svpi_file",
        alias = "svpi-file"
    )]
    pub svpi_file: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct BalanceArgs {
    #[arg(value_name = "ADDRESS", help = "Solana (base58) or EVM (hex) address")]
    pub address: Option<String>,
    #[arg(long, value_enum, help = "EVM network name (enables EVM balance mode)")]
    pub network: Option<EvmNetworkArg>,
    #[arg(
        long,
        value_name = "URL",
        help = "Custom RPC URL (Solana or EVM depending on mode)"
    )]
    pub rpc: Option<String>,
    #[arg(
        long,
        value_name = "CLUSTER",
        help = "Solana cluster (default mainnet-beta)"
    )]
    pub cluster: Option<String>,
    #[arg(
        long,
        value_enum,
        value_name = "LEVEL",
        help = "Solana commitment (default confirmed)"
    )]
    pub commitment: Option<CommitmentArg>,
    #[command(flatten)]
    pub key: BalanceKeyOptions,
}

#[derive(Args, Debug, Clone)]
pub struct BalanceKeyOptions {
    #[arg(long, value_name = "PATH", help = "Solana keypair file (JSON array)")]
    pub keyfile: Option<PathBuf>,
    #[arg(
        long,
        value_name = "HEX",
        help = "EVM private key (with or without 0x)"
    )]
    pub privkey: Option<String>,
    #[arg(long, value_name = "PATH", help = "File containing EVM private key")]
    pub privkey_file: Option<PathBuf>,
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
    #[arg(long, help = "Use SVPI to fetch mnemonic or hex private key")]
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
    #[arg(
        long,
        value_name = "PASS",
        help = "SVPI password (optional, otherwise prompt)"
    )]
    pub svpi_pass: Option<String>,
}

impl BalanceKeyOptions {
    pub fn into_solana(self) -> SolanaKeyOptions {
        SolanaKeyOptions {
            keyfile: self.keyfile,
            seed: self.seed,
            path: self.path,
            mnemo: self.mnemo,
            seed_passphrase: self.seed_passphrase,
            svpi: self.svpi,
            svpi_name: self.svpi_name,
            svpi_file: self.svpi_file,
            svpi_cmd: self.svpi_cmd,
            svpi_pass: self.svpi_pass,
        }
    }

    pub fn into_evm(self) -> EvmKeyOptions {
        EvmKeyOptions {
            privkey: self.privkey,
            privkey_file: self.privkey_file,
            seed: self.seed,
            path: self.path,
            seed_passphrase: self.seed_passphrase,
            svpi: self.svpi,
            svpi_name: self.svpi_name,
            svpi_file: self.svpi_file,
            svpi_cmd: self.svpi_cmd,
            svpi_pass: self.svpi_pass,
        }
    }
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
pub struct PriceArgs {
    #[arg(value_enum, value_name = "TOKEN", help = "Token symbol (sol|usdc)")]
    pub token: SwapToken,
    #[command(flatten)]
    pub key: SolanaKeyOptions,
    #[command(flatten)]
    pub rpc: SolanaRpcOptions,
}

#[derive(Args, Debug, Clone)]
pub struct TradeArgs {
    #[arg(value_name = "AMOUNT", help = "Amount of token to buy or sell")]
    pub amount: String,
    #[arg(value_enum, value_name = "TOKEN", help = "Token symbol (sol|usdc)")]
    pub token: SwapToken,
    #[arg(
        long,
        value_name = "PERCENT",
        help = "Slippage tolerance percent (default 0.1)"
    )]
    pub slippage: Option<f64>,
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
    #[arg(long, help = "Use SVPI to fetch mnemonic or hex private key")]
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
    #[arg(
        long,
        value_name = "PASS",
        help = "SVPI password (optional, otherwise prompt)"
    )]
    pub svpi_pass: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct EvmKeyOptions {
    #[arg(
        long,
        value_name = "HEX",
        help = "EVM private key (with or without 0x)"
    )]
    pub privkey: Option<String>,
    #[arg(long, value_name = "PATH", help = "File containing EVM private key")]
    pub privkey_file: Option<PathBuf>,
    #[arg(long, value_name = "MNEMONIC", help = "BIP39 seed phrase")]
    pub seed: Option<String>,
    #[arg(long, value_name = "PATH", help = "BIP44 derivation path")]
    pub path: Option<String>,
    #[arg(long, value_name = "PASS", help = "BIP39 passphrase")]
    pub seed_passphrase: Option<String>,
    #[arg(long, help = "Use SVPI to fetch mnemonic or hex private key")]
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
    #[arg(
        long,
        value_name = "PASS",
        help = "SVPI password (optional, otherwise prompt)"
    )]
    pub svpi_pass: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct SolanaRpcOptions {
    #[arg(
        long,
        value_name = "CLUSTER",
        help = "Solana cluster (default mainnet-beta)"
    )]
    pub cluster: Option<String>,
    #[arg(long, value_name = "URL", help = "Custom Solana RPC URL")]
    pub rpc: Option<String>,
    #[arg(
        long,
        value_enum,
        value_name = "LEVEL",
        help = "Solana commitment (default confirmed)"
    )]
    pub commitment: Option<CommitmentArg>,
}

#[derive(Args, Debug, Clone)]
pub struct EvmTxOptions {
    #[arg(
        long,
        value_enum,
        value_name = "NETWORK",
        help = "EVM network (default mainnet)"
    )]
    pub network: Option<EvmNetworkArg>,
    #[arg(long, value_name = "URL", help = "Custom EVM RPC URL")]
    pub rpc: Option<String>,
    #[arg(long, value_name = "GWEI", help = "Gas price in gwei")]
    pub gas_price: Option<String>,
    #[arg(long, value_name = "NUMBER", help = "Gas limit override")]
    pub gas_limit: Option<u64>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum SwapToken {
    #[value(name = "sol")]
    Sol,
    #[value(name = "usdc")]
    Usdc,
}

impl SwapToken {
    pub fn other(self) -> Self {
        match self {
            SwapToken::Sol => SwapToken::Usdc,
            SwapToken::Usdc => SwapToken::Sol,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            SwapToken::Sol => "SOL",
            SwapToken::Usdc => "USDC",
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CommitmentArg {
    Processed,
    Confirmed,
    Finalized,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum EvmNetworkArg {
    #[value(name = "mainnet")]
    Mainnet,
    #[value(name = "sepolia")]
    Sepolia,
    #[value(name = "holesky")]
    Holesky,
    #[value(name = "polygon")]
    Polygon,
    #[value(name = "polygon_amoy")]
    PolygonAmoy,
    #[value(name = "bsc")]
    Bsc,
    #[value(name = "bsc_testnet")]
    BscTestnet,
    #[value(name = "avalanche")]
    Avalanche,
    #[value(name = "avalanche_fuji")]
    AvalancheFuji,
    #[value(name = "optimism")]
    Optimism,
    #[value(name = "arbitrum")]
    Arbitrum,
}

impl EvmNetworkArg {
    pub fn as_str(self) -> &'static str {
        match self {
            EvmNetworkArg::Mainnet => "mainnet",
            EvmNetworkArg::Sepolia => "sepolia",
            EvmNetworkArg::Holesky => "holesky",
            EvmNetworkArg::Polygon => "polygon",
            EvmNetworkArg::PolygonAmoy => "polygon_amoy",
            EvmNetworkArg::Bsc => "bsc",
            EvmNetworkArg::BscTestnet => "bsc_testnet",
            EvmNetworkArg::Avalanche => "avalanche",
            EvmNetworkArg::AvalancheFuji => "avalanche_fuji",
            EvmNetworkArg::Optimism => "optimism",
            EvmNetworkArg::Arbitrum => "arbitrum",
        }
    }
}
