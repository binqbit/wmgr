use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use solana_keypair::{Keypair, Signer};

struct Workspace(tempfile::TempDir);

impl Workspace {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wmgr"))
            .current_dir(self.0.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }

    fn keypair(&self, name: &str) -> Keypair {
        let bytes: Vec<u8> = serde_json::from_slice(&fs::read(self.file(name)).unwrap()).unwrap();
        Keypair::try_from(bytes.as_slice()).unwrap()
    }

    #[cfg(unix)]
    fn vault(&self, args: &[&str]) -> Output {
        let mut args = args.to_vec();
        args.extend_from_slice(&[
            "--svpi-name",
            "test-wallet",
            "--svpi-cmd",
            "./mock-svpi",
            "--svpi-pass",
            "test-only-password",
        ]);
        self.run(&args)
    }

    #[cfg(unix)]
    fn mock_vault(&self) {
        use std::os::unix::fs::PermissionsExt;

        // SVPI v1 JSON CLI contract. No real executable or vault is used.
        fs::write(self.file("mock-svpi"), r#"#!/bin/sh
set -eu
operation=
password=
while [ "$#" -gt 0 ]; do
  case "$1" in
    list|set|get) operation=$1 ;;
    --mode=json|--file=*) ;;
    --password=*) password=${1#*=} ;;
    --) shift; break ;;
    *) exit 2 ;;
  esac
  shift
done
if [ -f .mock-password-error ] && [ "$operation" != list ]; then
  printf '%s\n' '{"schema":"svpi.response.v1","ok":false,"error":{"code":"password_error","message":"secret-message-sentinel"}}'
  exit 7
fi
case "$operation" in
  list)
    if [ -f .mock-payload ]; then
      printf '{"schema":"svpi.response.v1","ok":true,"result":{"segments":[{"name":"%s"}]}}\n' "$(cat .mock-name)"
    else
      printf '%s\n' '{"schema":"svpi.response.v1","ok":true,"result":{"segments":[]}}'
    fi ;;
  set)
    [ -n "$password" ]
    printf '%s' "$1" > .mock-name
    printf '%s' "$2" > .mock-payload
    printf '{"schema":"svpi.response.v1","ok":true,"result":{"saved":true,"name":"%s","data_type":"plain","encrypted":true}}\n' "$1" ;;
  get)
    [ -n "$password" ]
    printf '%s' '{"schema":"svpi.response.v1","ok":true,"result":{"data_type":"plain","encrypted":true,"data":"'
    encoded=$(sed ':join
$!{
N
b join
}
s/\\/\\\\/g
s/"/\\"/g
s/\n/\\n/g' .mock-payload)
    printf '%s' "$encoded"
    printf '%s\n' '"}}' ;;
  *) exit 2 ;;
esac
"#).unwrap();
        fs::set_permissions(self.file("mock-svpi"), fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn address(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Address: "))
        .expect("public address in output")
        .to_owned()
}

#[test]
fn unavailable_svpi_preserves_the_generated_wallet_as_a_local_backup() {
    let dir = Workspace::new();
    let result = dir.run(&[
        "wallet",
        "new",
        "--svpi-name",
        "test-wallet",
        "--svpi-cmd",
        "./missing-svpi",
        "--svpi-pass",
        "test-only-password",
    ]);
    let saved = address(&result);
    assert_eq!(
        dir.keypair(&format!("{saved}.json")).pubkey().to_string(),
        saved
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("saving a local backup"));

    #[cfg(unix)]
    {
        dir.mock_vault();
        fs::write(dir.file(".mock-password-error"), b"").unwrap();
        let failed_save = dir.vault(&["wallet", "new"]);
        let saved = address(&failed_save);
        assert_eq!(
            dir.keypair(&format!("{saved}.json")).pubkey().to_string(),
            saved
        );
        assert!(String::from_utf8_lossy(&failed_save.stderr).contains("saving a local backup"));
        assert!(!String::from_utf8_lossy(&failed_save.stderr).contains("secret-message-sentinel"));
        assert!(!String::from_utf8_lossy(&failed_save.stdout).contains("secret-message-sentinel"));
        assert!(!dir.file(".mock-payload").exists());
    }
}

#[cfg(unix)]
#[test]
fn unavailable_prompt_output_still_preserves_a_local_wallet() {
    use std::{
        os::{fd::OwnedFd, unix::net::UnixStream},
        process::Stdio,
    };

    let dir = Workspace::new();
    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader);
    let stderr: OwnedFd = writer.into();
    let result = Command::new(env!("CARGO_BIN_EXE_wmgr"))
        .current_dir(dir.0.path())
        .args(["wallet", "new"])
        .stdin(Stdio::null())
        .stderr(Stdio::from(stderr))
        .output()
        .unwrap();
    let saved = address(&result);
    assert_eq!(
        dir.keypair(&format!("{saved}.json")).pubkey().to_string(),
        saved
    );
}

#[cfg(unix)]
#[test]
fn mnemonic_import_export_and_recovery_preserve_the_wallet_and_path() {
    let dir = Workspace::new();
    dir.mock_vault();
    for folder in ["source", "exported", "recovered", "normalized"] {
        fs::create_dir(dir.file(folder)).unwrap();
    }
    let generated = dir.run(&[
        "wallet",
        "new",
        "--format",
        "mnemonic",
        "--path",
        "m/44'/501'/0'/0'",
        "--no-svpi",
        "--out-dir",
        "source",
    ]);
    let public_key = address(&generated);
    let source = format!("source/{public_key}.txt");
    let backup = fs::read_to_string(dir.file(&source)).unwrap();
    let phrase = backup
        .lines()
        .next()
        .unwrap()
        .strip_prefix("Seed phrase: ")
        .unwrap();
    assert_eq!(phrase.split_whitespace().count(), 24);
    assert!(backup.contains("Derivation path: m/44'/501'/0'/0'"));
    assert_eq!(
        dir.keypair(&format!("source/{public_key}.json"))
            .pubkey()
            .to_string(),
        public_key
    );
    assert!(!String::from_utf8_lossy(&generated.stdout).contains(phrase));
    fs::write(
        dir.file("phrase.txt"),
        phrase.split_whitespace().collect::<Vec<_>>().join("\n \t"),
    )
    .unwrap();
    assert_eq!(
        address(&dir.run(&[
            "wallet",
            "import",
            "--from",
            "phrase.txt",
            "--path",
            "m/44/501/0/0",
            "--no-svpi",
            "--out-dir",
            "normalized",
        ])),
        public_key
    );
    let normalized = fs::read_to_string(dir.file(&format!("normalized/{public_key}.txt"))).unwrap();
    assert_eq!(normalized, backup);
    assert_eq!(
        address(&dir.vault(&["wallet", "import", "--from", &source])),
        public_key
    );
    assert_eq!(
        address(&dir.vault(&["wallet", "export", "--out-dir", "exported"])),
        public_key
    );
    let exported = format!("exported/{public_key}.txt");
    let restored = fs::read_to_string(dir.file(&exported)).unwrap();
    assert_eq!(restored, backup);
    assert_eq!(
        dir.keypair(&format!("exported/{public_key}.json"))
            .pubkey()
            .to_string(),
        public_key
    );
    assert_eq!(
        address(&dir.run(&[
            "wallet",
            "recover",
            "--from",
            &exported,
            "--no-svpi",
            "--out-dir",
            "recovered",
        ])),
        public_key
    );
    assert_eq!(
        dir.keypair(&format!("recovered/{public_key}.json"))
            .pubkey()
            .to_string(),
        public_key
    );
}

#[cfg(unix)]
#[test]
fn new_wallet_defaults_to_svpi_and_export_writes_a_solana_keypair() {
    let dir = Workspace::new();
    dir.mock_vault();
    let result = dir.vault(&["wallet", "new"]);
    let public_key = address(&result);
    assert!(String::from_utf8_lossy(&result.stdout).contains("SVPI entry: test-wallet"));
    assert!(!dir.file(&format!("{public_key}.json")).exists());

    let original = fs::read(dir.file(".mock-payload")).unwrap();
    let duplicate_name = dir.vault(&["wallet", "new"]);
    let second_key = address(&duplicate_name);
    assert_ne!(second_key, public_key);
    assert!(String::from_utf8_lossy(&duplicate_name.stderr).contains("saving a local backup"));
    assert_eq!(
        dir.keypair(&format!("{second_key}.json"))
            .pubkey()
            .to_string(),
        second_key
    );
    assert!(original == fs::read(dir.file(".mock-payload")).unwrap());

    assert_eq!(address(&dir.vault(&["wallet", "export"])), public_key);
    let keypair = dir.keypair(&format!("{public_key}.json"));
    assert_eq!(keypair.pubkey().to_string(), public_key);

    fs::write(dir.file(".mock-password-error"), b"").unwrap();
    fs::create_dir(dir.file("failed-export")).unwrap();
    let failed_export = dir.vault(&["wallet", "export", "--out-dir", "failed-export"]);
    assert!(!failed_export.status.success());
    assert!(!String::from_utf8_lossy(&failed_export.stderr).contains("secret-message-sentinel"));
    assert!(fs::read_dir(dir.file("failed-export"))
        .unwrap()
        .next()
        .is_none());

    let prompted = Workspace::new();
    prompted.mock_vault();
    let mut child = Command::new(env!("CARGO_BIN_EXE_wmgr"))
        .current_dir(prompted.0.path())
        .args([
            "wallet",
            "new",
            "--svpi-cmd",
            "./mock-svpi",
            "--svpi-pass",
            "test-only-password",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"chosen-by-user\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    let public_key = address(&result);
    assert!(String::from_utf8_lossy(&result.stdout).contains("SVPI entry: chosen-by-user"));
    assert!(!prompted.file(&format!("{public_key}.json")).exists());
}

#[test]
fn imports_detect_public_and_private_keys_and_reject_invalid_input() {
    let dir = Workspace::new();
    fs::write(dir.file("invalid.txt"), "not a wallet").unwrap();
    let public_key = Keypair::new_from_array([42; 32]).pubkey().to_string();
    fs::write(dir.file("address.txt"), &public_key).unwrap();
    assert!(!dir
        .run(&["wallet", "import", "--from", "invalid.txt", "--no-svpi"])
        .status
        .success());
    assert!(!dir.file(&format!("{public_key}.json")).exists());
    assert_eq!(
        address(&dir.run(&["wallet", "import", "--from", "address.txt", "--no-svpi",])),
        public_key
    );
    assert_eq!(
        fs::read_to_string(dir.file(&format!("{public_key}.txt")))
            .unwrap()
            .trim(),
        public_key
    );
    assert!(!dir.file(&format!("{public_key}.json")).exists());

    fs::write(dir.file("private.txt"), hex::encode([42; 32])).unwrap();
    fs::create_dir(dir.file("private")).unwrap();
    assert_eq!(
        address(&dir.run(&[
            "wallet",
            "import",
            "--from",
            "private.txt",
            "--no-svpi",
            "--out-dir",
            "private",
        ])),
        public_key
    );
    let private = dir.keypair(&format!("private/{public_key}.json"));
    assert_eq!(private.secret_bytes(), &[42; 32]);
    assert_eq!(private.pubkey().to_string(), public_key);
}

#[test]
fn imports_reject_inconsistent_keys_and_preserve_existing_backups() {
    let dir = Workspace::new();
    let keypair = Keypair::new_from_array([42; 32]);
    let public_key = keypair.pubkey().to_string();
    let output = format!("{public_key}.json");
    let mut damaged = keypair.to_bytes();
    damaged[63] ^= 1;
    let wrong_address = serde_json::json!({
        "schema": "wmgr.wallet.v1", "kind": "keypair",
        "address": Keypair::new_from_array([43; 32]).pubkey().to_string(),
        "keypair": keypair.to_bytes().as_slice(),
    });
    for data in [
        serde_json::to_vec(damaged.as_slice()).unwrap(),
        serde_json::to_vec(&wrong_address).unwrap(),
    ] {
        fs::write(dir.file("import.json"), data).unwrap();
        assert!(!dir
            .run(&["wallet", "import", "--from", "import.json", "--no-svpi"])
            .status
            .success());
        assert!(!dir.file(&output).exists());
    }

    fs::write(
        dir.file("import.json"),
        serde_json::to_vec(keypair.to_bytes().as_slice()).unwrap(),
    )
    .unwrap();
    fs::write(dir.file(&output), b"existing backup").unwrap();
    assert!(!dir
        .run(&["wallet", "import", "--from", "import.json", "--no-svpi"])
        .status
        .success());
    assert_eq!(fs::read(dir.file(&output)).unwrap(), b"existing backup");

    #[cfg(unix)]
    {
        fs::remove_file(dir.file(&output)).unwrap();
        fs::write(dir.file("untouched"), b"existing target").unwrap();
        std::os::unix::fs::symlink(dir.file("untouched"), dir.file(&output)).unwrap();
        assert!(!dir
            .run(&["wallet", "import", "--from", "import.json", "--no-svpi"])
            .status
            .success());
        assert_eq!(fs::read(dir.file("untouched")).unwrap(), b"existing target");
    }
}
