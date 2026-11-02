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

        // SVPI 8.1 JSON CLI contract. No real executable or vault is used.
        fs::write(self.file("mock-svpi"), r#"#!/bin/sh
set -eu
operation=
password=
data_type=
encryption=
while [ "$#" -gt 0 ]; do
  case "$1" in
    list|set|get) operation=$1 ;;
    --mode=json|--file=*) ;;
    --password=*) password=${1#*=} ;;
    --data-type=*) data_type=${1#*=} ;;
    --encryption=*) encryption=${1#*=} ;;
    --) shift; break ;;
    *) exit 2 ;;
  esac
  shift
done
if [ -f .mock-password-error ] && [ "$operation" != list ]; then
  printf '%s\n' '{"schema":"svpi.response.v2","ok":false,"error":{"code":"password_error","message":"secret-message-sentinel"}}'
  exit 7
fi
stored_encryption='{"mode":"password"}'
if [ -f .mock-use-named-key ]; then
  stored_encryption='{"mode":"named_key","name":"wallet-key"}'
fi
case "$operation" in
  list)
    if [ -f .mock-v1 ]; then
      printf '%s\n' '{"schema":"svpi.response.v1","ok":true,"result":{"segments":[]}}'
    elif [ -f .mock-named-key ]; then
      printf '%s\n' '{"schema":"svpi.response.v2","ok":true,"result":{"records":[{"name":"test-wallet","kind":"encryption_key","data_type":"binary","encrypted":true,"encryption":{"mode":"password"}}],"capacity":null}}'
    elif [ -f .mock-payload ]; then
      printf '{"schema":"svpi.response.v2","ok":true,"result":{"records":[{"name":"%s","kind":"data","data_type":"plain","encrypted":true,"encryption":{"mode":"password"}}],"capacity":null}}\n' "$(cat .mock-name)"
    else
      printf '%s\n' '{"schema":"svpi.response.v2","ok":true,"result":{"records":[],"capacity":null}}'
    fi ;;
  set)
    [ -n "$password" ]
    [ "$data_type" = plain ]
    [ "$encryption" = auto ]
    printf '%s\n' set >> .mock-writes
    if [ -f .mock-set-cancelled ]; then
      printf '%s\n' '{"schema":"svpi.response.v2","ok":true,"result":{"cancelled":true,"action":"set","context":{}}}'
      exit 0
    fi
    if [ -f .mock-set-error ]; then
      code=$(cat .mock-set-error)
      if [ "$code" = storage_outcome_unknown ]; then
        printf '%s' "$1" > .mock-name
        printf '%s' "$2" > .mock-payload
      fi
      printf '{"schema":"svpi.response.v2","ok":false,"error":{"code":"%s","message":"secret-message-sentinel","details":{"password":"secret-details-sentinel"}}}\n' "$code"
      exit 9
    fi
    printf '%s' "$1" > .mock-name
    printf '%s' "$2" > .mock-payload
    if [ -f .mock-set-malformed-ack ]; then
      printf '{"schema":"svpi.response.v2","ok":true,"result":{"saved":true,"name":"%s","data_type":"plain"}}\n' "$1"
    elif [ -f .mock-set-plaintext ]; then
      printf '{"schema":"svpi.response.v2","ok":true,"result":{"saved":true,"name":"%s","data_type":"plain","encryption":{"mode":"none"},"version":{"kind":"unversioned"}}}\n' "$1"
    else
      printf '{"schema":"svpi.response.v2","ok":true,"result":{"saved":true,"name":"%s","data_type":"plain","encryption":%s,"version":{"kind":"unversioned"}}}\n' "$1" "$stored_encryption"
    fi ;;
  get)
    [ -n "$password" ]
    printf '%s\n' get >> .mock-reads
    record_name=$1
    record_kind=data
    record_encrypted=true
    if [ -f .mock-get-wrong-name ]; then record_name=other-wallet; fi
    if [ -f .mock-get-wrong-kind ]; then record_kind=encryption_key; fi
    if [ -f .mock-get-unencrypted ]; then record_encrypted=false; fi
    if [ -f .mock-get-plaintext ]; then stored_encryption='{"mode":"none"}'; fi
    if [ -f .mock-get-local-fingerprint ]; then
      stored_encryption='{"mode":"legacy_fingerprint","fingerprint":"1234abcd"}'
    fi
    printf '{"schema":"svpi.response.v2","ok":true,"result":{"record":{"name":"%s","kind":"%s","data_type":"plain","encrypted":%s' "$record_name" "$record_kind" "$record_encrypted"
    if [ ! -f .mock-get-missing-encryption ]; then
      printf ',"encryption":%s' "$stored_encryption"
    fi
    printf '%s' '},"data":"'
    if [ -f .mock-get-different-data ]; then
      printf '%s' secret-readback-sentinel
    else
    encoded=$(sed ':join
$!{
N
b join
}
s/\\/\\\\/g
s/"/\\"/g
s/\n/\\n/g' .mock-payload)
    printf '%s' "$encoded"
    fi
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
fn local_generation_and_bounded_vanity_produce_usable_keypairs() {
    let dir = Workspace::new();
    let generated = address(&dir.run(&["wallet", "new", "--no-svpi"]));
    let keypair = dir.keypair(&format!("{generated}.json"));
    assert_eq!(keypair.pubkey().to_string(), generated);
    let message = b"functional wallet test";
    assert!(keypair
        .sign_message(message)
        .verify(keypair.pubkey().as_ref(), message));

    let found = address(&dir.run(&[
        "wallet",
        "vanity",
        "--suffix",
        "a",
        "--ignore-case",
        "--backend",
        "cpu",
        "--threads",
        "2",
        "--max-attempts",
        "20000",
        "--no-svpi",
    ]));
    assert!(found.to_ascii_lowercase().ends_with('a'));
    assert_eq!(
        dir.keypair(&format!("{found}.json")).pubkey().to_string(),
        found
    );
}

#[test]
fn automatic_backend_falls_back_when_opencl_is_unavailable() {
    let dir = Workspace::new();
    let impossible = dir.run(&[
        "wallet",
        "vanity",
        "--prefix",
        "11111111111111111111111111111111",
        "--max-attempts",
        "1",
        "--no-svpi",
    ]);
    assert!(!impossible.status.success());
    assert!(String::from_utf8_lossy(&impossible.stderr).contains("Pattern cannot match"));
    let run = |backend: &str| {
        Command::new(env!("CARGO_BIN_EXE_wmgr"))
            .current_dir(dir.0.path())
            .env("OPENCL_DYLIB_PATH", dir.file("missing-opencl-library"))
            .args([
                "wallet",
                "vanity",
                "--suffix",
                "a",
                "--ignore-case",
                "--backend",
                backend,
                "--threads",
                "2",
                "--max-attempts",
                "20000",
                "--no-svpi",
            ])
            .output()
            .unwrap()
    };
    let required_gpu = run("gpu");
    assert!(!required_gpu.status.success());
    assert!(fs::read_dir(dir.0.path()).unwrap().next().is_none());

    let automatic = run("auto");
    let found = address(&automatic);
    assert!(String::from_utf8_lossy(&automatic.stdout).contains("Backend: CPU"));
    assert!(found.to_ascii_lowercase().ends_with('a'));
    assert_eq!(
        dir.keypair(&format!("{found}.json")).pubkey().to_string(),
        found
    );
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

#[cfg(unix)]
#[test]
fn incompatible_svpi_and_existing_named_keys_do_not_trigger_a_write() {
    for marker in [".mock-v1", ".mock-named-key"] {
        let dir = Workspace::new();
        dir.mock_vault();
        fs::write(dir.file(marker), b"").unwrap();

        let result = dir.vault(&["wallet", "new"]);
        let public_key = address(&result);
        assert!(String::from_utf8_lossy(&result.stderr).contains("saving a local backup"));
        assert!(!String::from_utf8_lossy(&result.stdout).contains("SVPI entry:"));
        assert_eq!(
            dir.keypair(&format!("{public_key}.json"))
                .pubkey()
                .to_string(),
            public_key
        );
        assert!(
            !dir.file(".mock-writes").exists(),
            "{marker} allowed a write"
        );
        assert!(!dir.file(".mock-payload").exists());
    }
}

#[cfg(unix)]
#[test]
fn encrypted_svpi_readbacks_support_named_keys_and_local_fingerprints() {
    for (named_key, local_fingerprint) in [(false, true), (true, false), (true, true)] {
        let dir = Workspace::new();
        dir.mock_vault();
        if named_key {
            fs::write(dir.file(".mock-use-named-key"), b"").unwrap();
        }
        if local_fingerprint {
            fs::write(dir.file(".mock-get-local-fingerprint"), b"").unwrap();
        }

        let result = dir.vault(&["wallet", "new"]);
        let public_key = address(&result);
        assert!(String::from_utf8_lossy(&result.stdout).contains("SVPI entry: test-wallet"));
        assert!(!dir.file(&format!("{public_key}.json")).exists());
        assert_eq!(address(&dir.vault(&["wallet", "export"])), public_key);
        assert_eq!(
            dir.keypair(&format!("{public_key}.json"))
                .pubkey()
                .to_string(),
            public_key
        );
    }
}

#[cfg(unix)]
#[test]
fn unconfirmed_or_unencrypted_svpi_saves_preserve_a_local_backup() {
    for marker in [
        ".mock-set-cancelled",
        ".mock-set-malformed-ack",
        ".mock-set-plaintext",
        ".mock-get-missing-encryption",
        ".mock-get-plaintext",
        ".mock-get-unencrypted",
        ".mock-get-wrong-name",
        ".mock-get-wrong-kind",
        ".mock-get-different-data",
    ] {
        let dir = Workspace::new();
        dir.mock_vault();
        fs::write(dir.file(marker), b"").unwrap();

        let result = dir.vault(&["wallet", "new"]);
        let public_key = address(&result);
        assert!(String::from_utf8_lossy(&result.stderr).contains("saving a local backup"));
        assert!(!String::from_utf8_lossy(&result.stdout).contains("SVPI entry:"));
        assert!(!String::from_utf8_lossy(&result.stdout).contains("sentinel"));
        assert!(!String::from_utf8_lossy(&result.stderr).contains("sentinel"));
        assert_eq!(
            dir.keypair(&format!("{public_key}.json"))
                .pubkey()
                .to_string(),
            public_key
        );
        assert_eq!(
            fs::read_to_string(dir.file(".mock-writes")).unwrap(),
            "set\n"
        );
        if marker == ".mock-set-cancelled" {
            assert!(!dir.file(".mock-payload").exists());
        }
        if marker.starts_with(".mock-get-") {
            assert_eq!(
                fs::read_to_string(dir.file(".mock-reads")).unwrap(),
                "get\n"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn conflicting_or_uncertain_svpi_writes_are_not_retried_or_disclosed() {
    for (code, expected_error) in [
        ("conflict", "inspect it before retrying"),
        ("storage_outcome_unknown", "outcome is unknown"),
    ] {
        let dir = Workspace::new();
        dir.mock_vault();
        fs::write(dir.file(".mock-set-error"), code).unwrap();

        let result = dir.vault(&["wallet", "new"]);
        let public_key = address(&result);
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains(expected_error), "{stderr}");
        assert!(stderr.contains("saving a local backup"));
        assert!(!stdout.contains("SVPI entry:"));
        for output in [&stdout, &stderr] {
            assert!(!output.contains("sentinel"));
            assert!(!output.contains("test-only-password"));
        }
        assert_eq!(
            dir.keypair(&format!("{public_key}.json"))
                .pubkey()
                .to_string(),
            public_key
        );
        assert_eq!(
            fs::read_to_string(dir.file(".mock-writes")).unwrap(),
            "set\n"
        );
        assert!(!dir.file(".mock-reads").exists());
        assert_eq!(
            dir.file(".mock-payload").exists(),
            code == "storage_outcome_unknown"
        );
    }
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
