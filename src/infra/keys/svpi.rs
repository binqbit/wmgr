use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq)]
pub struct SvpiData {
    pub data_type: String,
    pub data: String,
}

impl std::fmt::Debug for SvpiData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SvpiData { [REDACTED] }")
    }
}

fn parse_first_json_value(raw: &str) -> Result<Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(anyhow!("SVPI returned empty output"));
    }

    serde_json::from_str::<Value>(raw).map_err(|_| anyhow!("Failed to parse JSON from SVPI output"))
}

fn extract_typed_data(value: &Value) -> Result<SvpiData> {
    let data_type = value
        .get("data_type")
        .and_then(|v| v.as_str())
        .map(|value| value.trim().to_owned())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("SVPI response did not include data_type"))?;

    let data = value
        .get("data")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("SVPI response did not include data"))?;

    Ok(SvpiData { data_type, data })
}

fn error_code(value: &Value) -> Option<&str> {
    value
        .get("error")
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str())
}

fn parse_svpi_error(value: &Value) -> anyhow::Error {
    // Child messages and details can contain wallet data or credentials. Only
    // known protocol codes are mapped to locally controlled error messages.
    let message = match error_code(value) {
        Some("data_not_found") => "SVPI record was not found",
        Some("password_required" | "password_error" | "master_password_invalid") => {
            "SVPI password is missing or incorrect"
        }
        Some("device_not_initialized") => "SVPI storage is not initialized",
        Some("not_enough_memory") => "SVPI storage has insufficient capacity",
        Some("device_not_found" | "device_error" | "io_error") => "SVPI storage is unavailable",
        Some("architecture_mismatch") => "SVPI storage version is incompatible",
        Some("invalid_argument" | "missing_argument" | "invalid_command") => {
            "SVPI rejected the operation arguments"
        }
        Some("confirmation_required") => "SVPI requires confirmation for this operation",
        _ => "SVPI operation failed",
    };
    anyhow!(message)
}

#[cfg(test)]
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

fn validate_credentials(name: &str, password: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("SVPI name is required");
    }
    if password.is_empty() {
        bail!("SVPI password is required");
    }
    Ok(())
}

fn svpi_command(file_path: Option<&Path>, cmd_path: Option<&Path>) -> Command {
    let cmd_exe = cmd_path
        .map(|path| path.as_os_str())
        .unwrap_or_else(|| OsStr::new("svpi"));
    let mut cmd = Command::new(cmd_exe);
    // Keep the inherited working directory so SVPI resolves its own configured
    // defaults. JSON mode must never fall back to an interactive child prompt.
    cmd.stdin(Stdio::null());
    cmd.arg("--mode=json");
    if let Some(path) = file_path {
        let mut argument = OsString::from("--file=");
        argument.push(path);
        cmd.arg(argument);
    }
    cmd
}

struct SvpiResponse {
    value: Value,
    exit_success: bool,
    ok: bool,
}

fn run_svpi(mut command: Command) -> Result<SvpiResponse> {
    let output = command
        .output()
        .map_err(|_| anyhow!("Failed to execute SVPI command"))?;
    let stdout = Zeroizing::new(output.stdout);
    let stderr = Zeroizing::new(output.stderr);
    if output.status.code().is_none() {
        bail!("SVPI command terminated unexpectedly");
    }
    let parse = |bytes: &[u8]| {
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|raw| parse_first_json_value(raw).ok())
    };
    let value = parse(&stdout)
        .or_else(|| parse(&stderr))
        .ok_or_else(|| anyhow!("SVPI returned empty or malformed JSON output"))?;
    if value.get("schema").and_then(Value::as_str) != Some("svpi.response.v1") {
        bail!("SVPI returned an unsupported response schema");
    }
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow!("SVPI response is missing its operation status"))?;
    if ok != output.status.success() {
        bail!("SVPI exit status disagrees with its response");
    }
    Ok(SvpiResponse {
        value,
        exit_success: output.status.success(),
        ok,
    })
}

fn successful_result(response: &SvpiResponse) -> Result<&Value> {
    if !response.exit_success || !response.ok {
        return Err(parse_svpi_error(&response.value));
    }
    response
        .value
        .get("result")
        .filter(|value| value.is_object())
        .ok_or_else(|| anyhow!("SVPI response did not include an operation result"))
}

pub fn get_data_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<SvpiData> {
    validate_credentials(name, password)?;
    let mut command = svpi_command(file_path, cmd_path);
    command
        .arg("get")
        .arg(format!("--password={password}"))
        .arg("--")
        .arg(name);
    let response = run_svpi(command)?;
    extract_typed_data(successful_result(&response)?)
}

pub fn check_new_wallet(
    name: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<()> {
    if name.trim().is_empty() {
        bail!("SVPI name is required");
    }
    if name.len() > 32 {
        bail!("SVPI wallet names must not exceed 32 UTF-8 bytes");
    }
    let mut inspect = svpi_command(file_path, cmd_path);
    inspect.arg("list");
    let previous = run_svpi(inspect)?;
    let segments = successful_result(&previous)?
        .get("segments")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("SVPI did not return its entry list"))?;
    for segment in segments {
        let existing_name = segment
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("SVPI returned an entry without a name"))?;
        if existing_name == name {
            bail!("SVPI record already exists; refusing to replace it");
        }
    }
    Ok(())
}

pub fn set_data_in_svpi(
    name: &str,
    data: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<()> {
    validate_credentials(name, password)?;
    if data.is_empty() {
        bail!("SVPI wallet data is required");
    }
    // Recheck immediately before writing, even if the caller already checked
    // availability before prompting for the password.
    check_new_wallet(name, file_path, cmd_path)?;

    // SVPI v1 has no atomic create-only command: this
    // preflight prevents an ordinary replacement but cannot exclude another
    // writer racing between list and set.
    // This existing CLI interface accepts secrets only as arguments; callers
    // must not describe it as protection from local process-argument inspection.
    let mut command = svpi_command(file_path, cmd_path);
    command
        .arg("set")
        .arg(format!("--password={password}"))
        .arg("--")
        .arg(name)
        .arg(data);
    let response = run_svpi(command)?;
    let result = successful_result(&response)?;
    if result.get("saved").and_then(Value::as_bool) != Some(true)
        || result.get("name").and_then(Value::as_str) != Some(name)
        || result.get("data_type").and_then(Value::as_str) != Some("plain")
        || result.get("encrypted").and_then(Value::as_bool) != Some(true)
    {
        bail!("SVPI did not confirm the encrypted wallet save");
    }
    let restored = get_data_from_svpi(name, password, file_path, cmd_path)?;
    if restored.data_type != "plain" || restored.data != data {
        bail!("SVPI wallet readback did not match the saved data");
    }
    Ok(())
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
        let typed = try_extract_svpi_data_from_json(raw)
            .expect("parse")
            .expect("typed");
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
