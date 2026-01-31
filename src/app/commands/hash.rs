use std::{
    fs,
    io::ErrorKind,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

use crate::config::app_config::{WmgrConfig, CONFIG_FILE_NAME};

const SVPI_CONFIG_FILE_NAME: &str = ".svpi";

pub fn handle_self_hash(cfg: &WmgrConfig) -> Result<()> {
    let exe_path = std::env::current_exe()
        .map_err(|err| anyhow!("Failed to resolve current executable path: {err}"))?;
    let wmgr_hash = sha256_file_hex(&exe_path)
        .map_err(|err| anyhow!("Failed to hash wmgr executable: {err}"))?;

    println!("wmgr:");
    println!("{:15}{}", "app:", wmgr_hash);

    let cfg_hash = {
        let cfg_path = WmgrConfig::path_in_cwd()
            .map_err(|err| anyhow!("Failed to resolve {CONFIG_FILE_NAME}: {err}"))?;
        match sha256_file_hex(&cfg_path) {
            Ok(hash) => Some(hash),
            Err(err) if err.kind() == ErrorKind::NotFound => None,
            Err(err) => return Err(anyhow!("Failed to hash {CONFIG_FILE_NAME}: {err}")),
        }
    };
    println!(
        "{:15}{}",
        format!("config({CONFIG_FILE_NAME}):"),
        cfg_hash.as_deref().unwrap_or("(not found)")
    );

    if cfg.is_svpi_mode() {
        let svpi_cmd = cfg.svpi_cmd.as_deref().unwrap_or("svpi");
        println!();
        println!("svpi:");

        let app_hash = match resolve_executable_path(svpi_cmd) {
            Some(svpi_path) => Some(
                sha256_file_hex(&svpi_path)
                    .map_err(|err| anyhow!("Failed to hash svpi executable: {err}"))?,
            ),
            None => None,
        };
        println!(
            "{:15}{}",
            "app:",
            app_hash.as_deref().unwrap_or("(not found)")
        );

        let cfg_hash = {
            let cfg_path = std::env::current_dir()
                .map_err(|err| anyhow!("Failed to resolve {SVPI_CONFIG_FILE_NAME}: {err}"))?
                .join(SVPI_CONFIG_FILE_NAME);
            match sha256_file_hex(&cfg_path) {
                Ok(hash) => Some(hash),
                Err(err) if err.kind() == ErrorKind::NotFound => None,
                Err(err) => return Err(anyhow!("Failed to hash {SVPI_CONFIG_FILE_NAME}: {err}")),
            }
        };
        println!(
            "{:15}{}",
            format!("config({SVPI_CONFIG_FILE_NAME}):"),
            cfg_hash.as_deref().unwrap_or("(not found)")
        );
    }

    Ok(())
}

fn sha256_file_hex(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn resolve_executable_path(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let as_path = PathBuf::from(trimmed);
    if looks_like_path(trimmed) {
        return as_path.is_file().then_some(as_path);
    }

    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(trimmed);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{trimmed}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }

    None
}

fn looks_like_path(value: &str) -> bool {
    value.contains(std::path::MAIN_SEPARATOR)
        || value.contains('/')
        || value.contains('\\')
        || Path::new(value).is_absolute()
}
