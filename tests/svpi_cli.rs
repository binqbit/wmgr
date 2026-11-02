//! Opt-in checks against the real SVPI executable. All records and configuration
//! are synthetic and confined to a temporary directory; no RPC is contacted.
//! Run with `cargo test --test svpi_cli -- --ignored` and optionally `SVPI_BIN`.

use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use solana_keypair::{Keypair, Signer};

struct Workspace {
    directory: tempfile::TempDir,
    executable: OsString,
}

impl Workspace {
    fn new() -> Self {
        let workspace = Self {
            directory: tempfile::tempdir().unwrap(),
            executable: std::env::var_os("SVPI_BIN").unwrap_or_else(|| "svpi".into()),
        };
        let version = Command::new(&workspace.executable)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .expect("SVPI_BIN must point to an installed SVPI 8.1 executable");
        assert!(version.status.success(), "SVPI version check failed");
        assert!(
            String::from_utf8_lossy(&version.stdout).starts_with("svpi 8.1."),
            "These checks require SVPI 8.1"
        );
        workspace
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn svpi(&self, vault: &str, args: &[&str]) -> Value {
        let mut file_argument = OsString::from("--file=");
        file_argument.push(self.file(vault));
        let output = Command::new(&self.executable)
            .current_dir(self.directory.path())
            .args(["--no-config", "--mode=json"])
            .arg(file_argument)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("Failed to start SVPI");
        svpi_result(output)
    }

    fn initialize(&self, vault: &str) {
        let result = self.svpi(vault, &["init", "1000000", "low"]);
        assert_eq!(result["initialized"], true);
    }

    fn config(&self, args: &[&str]) {
        let mut config_argument = OsString::from("--config=");
        config_argument.push(self.file(".svpi"));
        let output = Command::new(&self.executable)
            .current_dir(self.directory.path())
            .arg("--mode=json")
            .arg(config_argument)
            .arg("config")
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("Failed to start SVPI config command");
        svpi_result(output);
    }

    fn wmgr(&self, args: &[&str], name: &str, password: &str, vault: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_wmgr"));
        command
            .current_dir(self.directory.path())
            .args(args)
            .arg(format!("--svpi-name={name}"))
            .arg("--svpi-cmd")
            .arg(&self.executable)
            .arg(format!("--svpi-pass={password}"))
            .stdin(Stdio::null());
        if let Some(vault) = vault {
            command.arg("--svpi-file").arg(self.file(vault));
        }
        command.output().expect("Failed to start wmgr")
    }

    fn get(&self, vault: &str, name: &str, password: &str) -> Value {
        self.svpi(
            vault,
            &["get", &format!("--password={password}"), "--", name],
        )
    }

    fn exported_keypair(&self, address: &str) -> Keypair {
        let bytes: Vec<u8> =
            serde_json::from_slice(&fs::read(self.file(&format!("{address}.json"))).unwrap())
                .unwrap();
        Keypair::try_from(bytes.as_slice()).unwrap()
    }
}

fn svpi_result(output: Output) -> Value {
    // Captured output may contain decrypted wallet material. Never print it in
    // assertion diagnostics, including on unexpected executable failures.
    assert!(output.status.success(), "SVPI command failed");
    let mut reply: Value =
        serde_json::from_slice(&output.stdout).expect("Invalid SVPI JSON response");
    assert_eq!(reply["schema"], "svpi.response.v2");
    assert_eq!(reply["ok"], true);
    reply["result"].take()
}

fn saved_address(output: &Output, name: &str) -> String {
    let address = address(output);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("SVPI entry: {name}")),
        "wmgr did not confirm an SVPI save"
    );
    address
}

fn address(output: &Output) -> String {
    assert!(output.status.success(), "wmgr command failed");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Address: "))
        .expect("wmgr did not report a wallet address")
        .to_owned()
}

#[test]
#[ignore = "requires installed SVPI 8.1"]
fn real_svpi_file_roundtrip_preserves_credentials_and_refuses_duplicates() {
    let workspace = Workspace::new();
    workspace.initialize("vault.bin");
    let name = "--wallet-україна";
    let password = " --test-only пароль\t'\"\\\n ";

    let generated = workspace.wmgr(&["wallet", "new"], name, password, Some("vault.bin"));
    let public_key = saved_address(&generated, name);
    assert!(!workspace.file(&format!("{public_key}.json")).exists());
    let original = workspace.get("vault.bin", name, password);
    assert_eq!(original["record"]["data_type"], "plain");
    assert_eq!(original["record"]["encrypted"], true);
    // The local vault stores the selector rather than the explicit encryption
    // mode returned by `set`; reads describe it as a legacy fingerprint.
    assert_eq!(
        original["record"]["encryption"]["mode"],
        "legacy_fingerprint"
    );

    let exported = workspace.wmgr(&["wallet", "export"], name, password, Some("vault.bin"));
    assert_eq!(address(&exported), public_key);
    let signer = workspace.exported_keypair(&public_key);
    assert_eq!(signer.pubkey().to_string(), public_key);
    let message = b"real SVPI integration verification";
    assert!(signer
        .sign_message(message)
        .verify(signer.pubkey().as_ref(), message));

    let duplicate = workspace.wmgr(&["wallet", "new"], name, password, Some("vault.bin"));
    let fallback = address(&duplicate);
    assert_ne!(fallback, public_key);
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("record already exists"));
    assert!(!String::from_utf8_lossy(&duplicate.stdout).contains("SVPI entry:"));
    assert_eq!(
        workspace.exported_keypair(&fallback).pubkey().to_string(),
        fallback
    );
    let after = workspace.get("vault.bin", name, password);
    assert!(
        after["data"] == original["data"],
        "Existing wallet was replaced"
    );
}

#[test]
#[ignore = "requires installed SVPI 8.1"]
fn real_svpi_named_key_roundtrip_preserves_mnemonic_and_key_record() {
    let workspace = Workspace::new();
    workspace.initialize("vault.bin");
    let password = "test-only-key-password";
    workspace.svpi(
        "vault.bin",
        &["set-master-password", "--master-password=test-only-master"],
    );
    workspace.svpi(
        "vault.bin",
        &[
            "add-encryption-key",
            "wallet-key",
            "low",
            "--master-password=test-only-master",
            "--key-password=test-only-key-password",
        ],
    );
    let keys_before = workspace.svpi("vault.bin", &["list-keys"]);
    let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    fs::write(workspace.file("phrase.txt"), phrase).unwrap();
    let path = "m/44'/501'/3'/0'";
    let imported = workspace.wmgr(
        &["wallet", "import", "--from", "phrase.txt", "--path", path],
        "mnemonic-wallet",
        password,
        Some("vault.bin"),
    );
    let public_key = saved_address(&imported, "mnemonic-wallet");
    let saved = workspace.get("vault.bin", "mnemonic-wallet", password);
    assert_eq!(saved["record"]["encryption"]["mode"], "legacy_fingerprint");
    assert_eq!(
        saved["record"]["encryption"]["fingerprint"],
        keys_before["keys"][0]["fingerprint"]
    );
    let payload: Value = serde_json::from_str(saved["data"].as_str().unwrap()).unwrap();
    assert!(payload["phrase"] == phrase, "Mnemonic changed");
    assert_eq!(payload["derivation_path"], path);

    let exported = workspace.wmgr(
        &["wallet", "export"],
        "mnemonic-wallet",
        password,
        Some("vault.bin"),
    );
    assert_eq!(address(&exported), public_key);
    assert_eq!(
        workspace.exported_keypair(&public_key).pubkey().to_string(),
        public_key
    );
    let recovery = fs::read_to_string(workspace.file(&format!("{public_key}.txt"))).unwrap();
    assert!(recovery.contains(phrase), "Recovery phrase missing");
    assert!(recovery.contains(path), "Recovery derivation path missing");

    let collision = workspace.wmgr(
        &["wallet", "new"],
        "wallet-key",
        password,
        Some("vault.bin"),
    );
    assert!(collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("record already exists"));
    assert!(!String::from_utf8_lossy(&collision.stdout).contains("SVPI entry:"));
    let keys_after = workspace.svpi("vault.bin", &["list-keys"]);
    assert!(keys_before == keys_after, "Existing named key was changed");
}

#[test]
#[ignore = "requires installed SVPI 8.1"]
fn real_svpi_default_profile_and_explicit_file_select_the_intended_vault() {
    let workspace = Workspace::new();
    workspace.initialize("profile.bin");
    workspace.initialize("override.bin");
    workspace.config(&["profile", "add", "test-profile", "--file=./profile.bin"]);
    workspace.config(&["use", "test-profile"]);
    let password = "test-only-profile-password";

    let selected = workspace.wmgr(&["wallet", "new"], "profile-wallet", password, None);
    saved_address(&selected, "profile-wallet");
    let overridden = workspace.wmgr(
        &["wallet", "new"],
        "override-wallet",
        password,
        Some("override.bin"),
    );
    saved_address(&overridden, "override-wallet");

    let profile_records = workspace.svpi("profile.bin", &["list"]);
    let override_records = workspace.svpi("override.bin", &["list"]);
    assert_eq!(profile_records["records"].as_array().unwrap().len(), 1);
    assert_eq!(profile_records["records"][0]["name"], "profile-wallet");
    assert_eq!(override_records["records"].as_array().unwrap().len(), 1);
    assert_eq!(override_records["records"][0]["name"], "override-wallet");
}
