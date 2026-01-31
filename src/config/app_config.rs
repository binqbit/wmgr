use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use borsh_derive::{BorshDeserialize, BorshSerialize};

pub const CONFIG_FILE_NAME: &str = ".wmgr";

const CONFIG_MAGIC: [u8; 4] = *b"WCFG";

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct WmgrConfig {
    magic: [u8; 4],
    /// 0=manual, 1=svpi
    pub mode: u8,
    pub svpi_cmd: Option<String>,
    pub svpi_file: Option<String>,
    pub svpi_name: Option<String>,
    pub solana_cluster: Option<String>,
    pub solana_rpc: Option<String>,
    /// 0=processed, 1=confirmed, 2=finalized
    pub solana_commitment: Option<u8>,
    pub slippage: Option<f64>,
    pub evm_network: Option<String>,
    pub evm_rpc: Option<String>,
    pub evm_gas_price: Option<String>,
    pub evm_gas_limit: Option<u64>,
}

impl Default for WmgrConfig {
    fn default() -> Self {
        Self {
            magic: CONFIG_MAGIC,
            mode: 0,
            svpi_cmd: None,
            svpi_file: None,
            svpi_name: None,
            solana_cluster: None,
            solana_rpc: None,
            solana_commitment: None,
            slippage: None,
            evm_network: None,
            evm_rpc: None,
            evm_gas_price: None,
            evm_gas_limit: None,
        }
    }
}

impl WmgrConfig {
    pub fn is_svpi_mode(&self) -> bool {
        self.mode == 1
    }

    pub fn path_in_cwd() -> io::Result<PathBuf> {
        Ok(std::env::current_dir()?.join(CONFIG_FILE_NAME))
    }

    pub fn load_from_cwd() -> io::Result<Option<Self>> {
        let path = Self::path_in_cwd()?;
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> io::Result<Option<Self>> {
        let bytes = match fs::read(path) {
            Ok(v) => v,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        if bytes.len() == 1 {
            let mut cfg = Self::default();
            cfg.mode = bytes[0];
            return Ok(Some(cfg));
        }

        let cfg = match <Self as borsh::BorshDeserialize>::try_from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        if cfg.magic != CONFIG_MAGIC {
            return Ok(None);
        }
        Ok(Some(cfg))
    }

    pub fn save_to_cwd(&self) -> io::Result<()> {
        let path = Self::path_in_cwd()?;
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        let mut cfg = self.clone();
        cfg.magic = CONFIG_MAGIC;

        let bytes = borsh::to_vec(&cfg)
            .map_err(|_| io::Error::new(ErrorKind::InvalidData, "Failed to serialize config"))?;

        let Some(parent) = path.parent() else {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "Invalid config path",
            ));
        };
        let tmp_path = parent.join(format!("{CONFIG_FILE_NAME}.tmp"));

        fs::write(&tmp_path, bytes)?;
        if let Err(err) = fs::rename(&tmp_path, path) {
            if err.kind() != ErrorKind::AlreadyExists {
                return Err(err);
            }
            let _ = fs::remove_file(path);
            fs::rename(&tmp_path, path)?;
        }

        Ok(())
    }
}
