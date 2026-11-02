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

fn parse_json_value(raw: &str) -> Result<Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(anyhow!("SVPI returned empty output"));
    }

    serde_json::from_str::<Value>(raw).map_err(|_| anyhow!("Failed to parse JSON from SVPI output"))
}

fn extract_typed_data(value: &Value, name: &str) -> Result<SvpiData> {
    let record = value
        .get("record")
        .filter(|record| record.is_object())
        .ok_or_else(|| anyhow!("SVPI response did not include a record description"))?;
    if record.get("name").and_then(Value::as_str) != Some(name) {
        bail!("SVPI returned a different record than requested");
    }
    if record.get("kind").and_then(Value::as_str) != Some("data") {
        bail!("SVPI entry is not a wallet data record");
    }
    let data_type = record
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
        Some("device_not_initialized" | "context_not_initialized") => {
            "SVPI storage is not initialized"
        }
        Some("not_enough_memory") => "SVPI storage has insufficient capacity",
        Some("device_not_found" | "device_error" | "io_error" | "storage_unavailable") => {
            "SVPI storage is unavailable"
        }
        Some("architecture_mismatch" | "unsupported_version") => {
            "SVPI storage version is incompatible"
        }
        Some("unauthorized" | "forbidden") => "SVPI storage access was denied",
        Some("conflict") => "SVPI record changed or already exists; inspect it before retrying",
        Some("storage_timeout" | "storage_outcome_unknown") => {
            "SVPI storage outcome is unknown; inspect the record before retrying a save"
        }
        Some("storage_protocol_error") => "SVPI storage returned an invalid response",
        Some("unsupported_context") => {
            "SVPI storage does not support its configured access context"
        }
        Some("unsupported_operation") => "SVPI storage does not support this operation",
        Some("invalid_config") => "SVPI configuration is invalid",
        Some("invalid_argument" | "missing_argument" | "invalid_command") => {
            "SVPI rejected the operation arguments"
        }
        Some("confirmation_required") => "SVPI requires confirmation for this operation",
        _ => "SVPI operation failed",
    };
    anyhow!(message)
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
    ok: bool,
}

fn parse_response(stdout: &[u8], exit_success: bool) -> Result<SvpiResponse> {
    // JSON mode's public response is on stdout. Stderr is diagnostic output and
    // must never substitute for a missing or malformed operation response.
    let value = std::str::from_utf8(stdout)
        .ok()
        .and_then(|raw| parse_json_value(raw).ok())
        .ok_or_else(|| anyhow!("SVPI returned empty or malformed JSON output"))?;
    if value.get("schema").and_then(Value::as_str) != Some("svpi.response.v2") {
        bail!("SVPI returned an unsupported response schema; SVPI 8.1 / v2 is required");
    }
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow!("SVPI response is missing its operation status"))?;
    if ok != exit_success {
        bail!("SVPI exit status disagrees with its response");
    }
    Ok(SvpiResponse { value, ok })
}

fn run_svpi(mut command: Command) -> Result<SvpiResponse> {
    let output = command
        .output()
        .map_err(|_| anyhow!("Failed to execute SVPI command"))?;
    let stdout = Zeroizing::new(output.stdout);
    let _stderr = Zeroizing::new(output.stderr);
    if output.status.code().is_none() {
        bail!("SVPI command terminated unexpectedly");
    }
    parse_response(&stdout, output.status.success())
}

fn successful_result(response: &SvpiResponse) -> Result<&Value> {
    if !response.ok {
        return Err(parse_svpi_error(&response.value));
    }
    let result = response
        .value
        .get("result")
        .filter(|value| value.is_object())
        .ok_or_else(|| anyhow!("SVPI response did not include an operation result"))?;
    if result.get("cancelled").and_then(Value::as_bool) == Some(true) {
        bail!("SVPI operation was cancelled");
    }
    Ok(result)
}

fn get_response(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<SvpiResponse> {
    validate_credentials(name, password)?;
    let mut command = svpi_command(file_path, cmd_path);
    command
        .arg("get")
        .arg(format!("--password={password}"))
        .arg("--")
        .arg(name);
    run_svpi(command)
}

pub fn get_data_from_svpi(
    name: &str,
    password: &str,
    file_path: Option<&Path>,
    cmd_path: Option<&Path>,
) -> Result<SvpiData> {
    let response = get_response(name, password, file_path, cmd_path)?;
    extract_typed_data(successful_result(&response)?, name)
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
    // CLI list includes both data and named encryption keys, unlike API list.
    let records = successful_result(&previous)?
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("SVPI did not return its entry list"))?;
    for record in records {
        let existing_name = record
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

    // Local SVPI writes have no atomic create-only command: this preflight
    // cannot exclude a concurrent writer between list and set. HTTP storage
    // makes this write create-only because no expected revision is supplied.
    // This existing CLI interface accepts secrets only as arguments; callers
    // must not describe it as protection from local process-argument inspection.
    let mut command = svpi_command(file_path, cmd_path);
    command
        .arg("set")
        .arg(format!("--password={password}"))
        .arg("--data-type=plain")
        // Preserve selection of a matching named key before password fallback.
        .arg("--encryption=auto")
        .arg("--")
        .arg(name)
        .arg(data);
    let response = run_svpi(command)?;
    let result = successful_result(&response)?;
    if result.get("saved").and_then(Value::as_bool) != Some(true)
        || result.get("name").and_then(Value::as_str) != Some(name)
        || result.get("data_type").and_then(Value::as_str) != Some("plain")
        || !encrypted_reference(&result["encryption"])
    {
        bail!("SVPI did not confirm the encrypted wallet save");
    }
    let restored_response = get_response(name, password, file_path, cmd_path)?;
    let restored_result = successful_result(&restored_response)?;
    let restored = extract_typed_data(restored_result, name)?;
    if restored.data_type != "plain"
        || restored.data != data
        || restored_result["record"]["encrypted"].as_bool() != Some(true)
        // Local vaults describe encrypted records using legacy fingerprints,
        // even when the write selected a password or a named key.
        || !encrypted_reference(&restored_result["record"]["encryption"])
    {
        bail!("SVPI wallet readback did not match the saved data");
    }
    Ok(())
}

fn encrypted_reference(encryption: &Value) -> bool {
    match encryption.get("mode").and_then(Value::as_str) {
        Some("password") => true,
        Some("named_key") => encryption
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.is_empty()),
        Some("legacy_fingerprint") => encryption
            .get("fingerprint")
            .and_then(Value::as_str)
            .is_some_and(|value| value.len() == 8 && value.bytes().all(|b| b.is_ascii_hexdigit())),
        _ => false,
    }
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
    use serde_json::json;

    fn read_result() -> Value {
        json!({
            "record": {
                "name": "wallet", "kind": "data", "data_type": "plain",
                "encrypted": true,
                "encryption": {"mode": "legacy_fingerprint", "fingerprint": "1234abcd"}
            },
            "data": " alpha\n"
        })
    }

    fn reply(result: Value) -> Value {
        json!({"schema": "svpi.response.v2", "ok": true, "result": result})
    }

    #[test]
    fn reads_nested_v2_record_without_changing_data() {
        let raw = serde_json::to_vec(&reply(read_result())).unwrap();
        let response = parse_response(&raw, true).unwrap();
        let typed = extract_typed_data(successful_result(&response).unwrap(), "wallet").unwrap();
        assert_eq!(typed.data_type, "plain");
        assert_eq!(typed.data, " alpha\n");
        assert!(!format!("{typed:?}").contains("alpha"));
    }

    #[test]
    fn refuses_malformed_or_legacy_envelopes_and_exit_disagreement() {
        let valid = reply(read_result());
        for raw in [
            Vec::new(),
            vec![0xff],
            format!("{}\nPress Enter to continue...", valid).into_bytes(),
            format!("{}\n{}", valid, valid).into_bytes(),
            b"{\"schema\":\"svpi.response.v1\",\"ok\":true,\"result\":{}}".to_vec(),
            b"{\"ok\":true,\"result\":{}}".to_vec(),
            b"{\"schema\":\"svpi.response.v2\",\"ok\":\"true\",\"result\":{}}".to_vec(),
        ] {
            assert!(parse_response(&raw, true).is_err());
        }
        assert!(parse_response(&serde_json::to_vec(&valid).unwrap(), false).is_err());
        let failure = json!({"schema": "svpi.response.v2", "ok": false});
        assert!(parse_response(&serde_json::to_vec(&failure).unwrap(), true).is_err());
    }

    #[test]
    fn rejects_wrong_record_kind_identity_or_missing_data() {
        let original = read_result();
        for result in [
            json!({"data_type": "plain", "data": "alpha"}),
            json!({"record": {"name": "wallet", "kind": "data"}, "data": "alpha"}),
            json!({"record": original["record"]}),
            json!({"record": original["record"], "data": ""}),
            json!({"record": original["record"], "data": 123}),
        ] {
            assert!(extract_typed_data(&result, "wallet").is_err());
        }
        for (field, value) in [
            ("name", json!("other")),
            ("kind", json!("encryption_key")),
            ("kind", Value::Null),
            ("data_type", json!("")),
        ] {
            let mut result = original.clone();
            result["record"][field] = value;
            assert!(extract_typed_data(&result, "wallet").is_err());
        }
    }

    #[test]
    fn does_not_treat_cancelled_or_missing_results_as_success() {
        for value in [
            reply(json!({"cancelled": true})),
            reply(Value::Null),
            reply(json!([])),
            json!({"schema": "svpi.response.v2", "ok": true}),
        ] {
            let response = parse_response(&serde_json::to_vec(&value).unwrap(), true).unwrap();
            assert!(successful_result(&response).is_err());
        }
    }

    #[test]
    fn maps_storage_errors_without_disclosing_child_messages() {
        for (code, expected) in [
            ("password_error", "password is missing or incorrect"),
            ("context_not_initialized", "not initialized"),
            ("unauthorized", "access was denied"),
            ("forbidden", "access was denied"),
            ("conflict", "inspect it before retrying"),
            ("storage_timeout", "outcome is unknown"),
            ("storage_outcome_unknown", "outcome is unknown"),
            ("storage_unavailable", "unavailable"),
            ("storage_protocol_error", "invalid response"),
            ("unsupported_context", "access context"),
            ("unsupported_operation", "does not support"),
            ("unsupported_version", "incompatible"),
            ("invalid_config", "configuration is invalid"),
            ("secret-code-sentinel", "operation failed"),
        ] {
            let value = json!({
                "schema": "svpi.response.v2", "ok": false,
                "error": {"code": code, "message": "secret-message-sentinel", "details": "secret-details-sentinel"}
            });
            let response = parse_response(&serde_json::to_vec(&value).unwrap(), false).unwrap();
            let error = successful_result(&response).unwrap_err().to_string();
            assert!(error.contains(expected), "{code}: {error}");
            assert!(!error.contains("sentinel"));
        }
    }

    #[test]
    fn recognizes_encrypted_references_from_remote_and_local_storage() {
        for reference in [
            json!({"mode": "password"}),
            json!({"mode": "named_key", "name": "personal"}),
            json!({"mode": "legacy_fingerprint", "fingerprint": "1234abcd"}),
        ] {
            assert!(encrypted_reference(&reference));
        }
        for reference in [
            Value::Null,
            json!({"mode": "none"}),
            json!({"mode": "unknown"}),
            json!({"mode": "named_key"}),
            json!({"mode": "named_key", "name": ""}),
            json!({"mode": "legacy_fingerprint", "fingerprint": "invalid"}),
        ] {
            assert!(!encrypted_reference(&reference));
        }
    }
}
