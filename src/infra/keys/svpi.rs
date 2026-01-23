use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::Value;

const SVPI_SCHEMA: &str = "svpi.response.v1";

#[derive(Debug, Deserialize)]
struct SvpiResponse {
    schema: String,
    ok: bool,
    result: Option<Value>,
    error: Option<SvpiError>,
}

#[derive(Debug, Deserialize)]
struct SvpiError {
    code: String,
    message: String,
    details: Option<Value>,
}

pub fn get_data_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<String> {
    if name.trim().is_empty() {
        return Err(anyhow!("SVPI name is required"));
    }
    if password.trim().is_empty() {
        return Err(anyhow!("SVPI password is required"));
    }

    let cmd_label = cmd_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "svpi".to_string());
    let cmd_exe = cmd_path
        .map(|path| path.as_os_str())
        .unwrap_or_else(|| OsStr::new("svpi"));

    let mut cmd = Command::new(cmd_exe);
    cmd.arg("--mode=json");
    if let Some(path) = file_path {
        cmd.arg(format!("--file={}", path.display()));
    }
    cmd.arg("get");
    cmd.arg(name);
    cmd.arg(format!("--password={password}"));

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute SVPI command: {cmd_label}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = if !stdout.trim().is_empty() {
        stdout.to_string()
    } else {
        stderr.to_string()
    };

    if raw.trim().is_empty() {
        return Err(anyhow!(
            "SVPI returned no JSON output (exit code {})",
            output.status
        ));
    }

    let resp: SvpiResponse = serde_json::from_str(raw.trim())
        .with_context(|| format!("Failed to parse SVPI JSON output: {raw}"))?;

    if resp.schema != SVPI_SCHEMA {
        return Err(anyhow!("Unexpected SVPI schema: {}", resp.schema));
    }

    if !resp.ok {
        let err = resp.error.unwrap_or(SvpiError {
            code: "svpi_error".to_string(),
            message: "SVPI returned an error".to_string(),
            details: None,
        });
        let details = err
            .details
            .map(|v| format!(" Details: {v}"))
            .unwrap_or_default();
        return Err(anyhow!(
            "SVPI error ({}): {}.{details}",
            err.code,
            err.message
        ));
    }

    let Some(result) = resp.result else {
        return Err(anyhow!("SVPI response missing result"));
    };
    let data = result
        .get("data")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("SVPI response did not include data"))?;

    Ok(data)
}

pub fn get_mnemonic_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<String> {
    get_data_from_svpi(name, password, file_path, cmd_path)
}
