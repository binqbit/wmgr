use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvpiData {
    pub data_type: String,
    pub data: String,
}

fn parse_first_json_value(raw: &str) -> Result<Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(anyhow!("SVPI returned empty output"));
    }

    serde_json::from_str::<Value>(raw).context("Failed to parse JSON from SVPI output")
}

fn extract_typed_data(value: &Value) -> Result<SvpiData> {
    let data_type = value
        .get("data_type")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("SVPI response did not include data_type"))?;

    let data = value
        .get("data")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("SVPI response did not include data"))?;

    Ok(SvpiData { data_type, data })
}

fn parse_svpi_error(value: &Value) -> anyhow::Error {
    let code = value
        .get("error")
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or("svpi_error");
    let message = value
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("SVPI returned an error");
    let details = value
        .get("error")
        .and_then(|v| v.get("details"))
        .map(|v| format!(" Details: {v}"))
        .unwrap_or_default();

    anyhow!("SVPI error ({}): {}.{details}", code, message)
}

fn try_extract_svpi_data_from_json(raw: &str) -> Result<Option<SvpiData>> {
    let value = match parse_first_json_value(raw) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    if let Some(ok) = value.get("ok").and_then(|v| v.as_bool()) {
        if !ok {
            return Err(parse_svpi_error(&value));
        }
    }

    if let Some(result) = value.get("result") {
        return Ok(Some(extract_typed_data(result)?));
    }

    Ok(Some(extract_typed_data(&value)?))
}

pub fn get_data_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<SvpiData> {
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
    // SVPI may try to read stdin after printing the JSON response.
    // For programmatic usage we always provide stdin as null to prevent hangs.
    cmd.stdin(Stdio::null());
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

    let stdout_s = stdout.to_string();
    let stderr_s = stderr.to_string();
    if stdout_s.trim().is_empty() && stderr_s.trim().is_empty() {
        return Err(anyhow!(
            "SVPI returned no output (exit code {})",
            output.status
        ));
    }

    // 1) Prefer JSON output (SVPI --mode=json) but don't hard-depend on the exact schema/envelope.
    // Important: do not include raw stdout/stderr in error context; it may contain secrets.
    if let Some(v) = try_extract_svpi_data_from_json(&stdout_s)? {
        return Ok(v);
    }
    if let Some(v) = try_extract_svpi_data_from_json(&stderr_s)? {
        return Ok(v);
    }

    if !output.status.success() {
        return Err(anyhow!(
            "SVPI exited with status {} (no parseable JSON output)",
            output.status
        ));
    }

    Err(anyhow!(
        "SVPI returned unexpected output (expected JSON response)"
    ))
}

#[allow(unused)]
pub fn get_mnemonic_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<String> {
    Ok(get_data_from_svpi(name, password, file_path, cmd_path)?.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_json_parses_strict_json() {
        let raw = "{\"schema\":\"svpi.response.v1\",\"ok\":true,\"result\":{\"data\":\"alpha\",\"data_type\":\"plain\"}}";
        let typed = try_extract_svpi_data_from_json(raw).expect("parse").expect("typed");
        assert_eq!(typed.data_type, "plain");
        assert_eq!(typed.data, "alpha");
    }

    #[test]
    fn extract_from_json_rejects_trailing_text() {
        let raw = "{\"ok\":true,\"result\":{\"data\":\"alpha\",\"data_type\":\"plain\"}}\nPress Enter to continue...";
        let res = try_extract_svpi_data_from_json(raw);
        assert!(res.expect("no error").is_none());
    }

    #[test]
    fn extract_from_json_requires_data_type() {
        let raw = "{\"ok\":true,\"result\":{\"data\":\"alpha\"}}";
        let err = try_extract_svpi_data_from_json(raw)
            .expect_err("expected error")
            .to_string();
        assert!(err.contains("data_type"));
    }
}
