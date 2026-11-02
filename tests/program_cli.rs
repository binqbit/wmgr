#![cfg(unix)]

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use solana_keypair::{Keypair, Signer};
use solana_loader_v3_interface::{get_program_data_address, state::UpgradeableLoaderState};

const LOADER: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

struct Workspace(tempfile::TempDir);

impl Workspace {
    fn new() -> Self {
        let workspace = Self(tempfile::tempdir().unwrap());
        let script = workspace.0.path().join("svpi");
        fs::write(
            &script,
            "#!/bin/sh\nset -eu\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    --mode=json|--file=*|--password=*|get) shift ;;\n    --) shift; break ;;\n    *) exit 2 ;;\n  esac\ndone\nprintf '%s\\n' \"$1\" >> gets\ncat \"$1.json\"\n",
        )
        .unwrap();
        fs::set_permissions(script, fs::Permissions::from_mode(0o700)).unwrap();
        let mut elf = [0; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        fs::write(workspace.0.path().join("program.so"), elf).unwrap();
        workspace
    }

    fn record(&self, name: &str, data: &str) {
        fs::write(
            self.0.path().join(format!("{name}.json")),
            serde_json::to_vec(&json!({
                "schema": "svpi.response.v2", "ok": true,
                "result": {
                    "record": {
                        "name": name, "kind": "data", "data_type": "plain",
                        "encrypted": true, "encryption": {"mode": "password"}
                    },
                    "data": data
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn signer(&self, name: &str, seed: u8) -> Keypair {
        let signer = Keypair::new_from_array([seed; 32]);
        self.record(
            name,
            &serde_json::to_string(signer.to_bytes().as_slice()).unwrap(),
        );
        signer
    }

    fn run(&self, args: &[&str], input: &str, rpc: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_wmgr"))
            .current_dir(self.0.path())
            .arg("program")
            .args(args)
            .args([
                "--svpi-cmd",
                "./svpi",
                "--svpi-pass",
                "test-only-password",
                "--cluster",
                "localnet",
                "--rpc",
                rpc,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    fn gets(&self) -> String {
        fs::read_to_string(self.0.path().join("gets")).unwrap()
    }
}

// Only account reads are allowed: these tests must stop before broadcasting.
fn with_rpc(
    accounts: HashMap<String, Value>,
    run: impl FnOnce(&str) -> Output,
) -> (Output, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let finished = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&finished);
    let server = thread::spawn(move || {
        let mut methods = Vec::new();
        while !stop.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("local RPC accept: {error}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let request: Value = serde_json::from_slice(&body).unwrap();
            let method = request["method"].as_str().unwrap();
            methods.push(method.to_owned());
            let response = match method {
                "getVersion" => json!({"result": {"solana-core": "3.1.0"}}),
                "getAccountInfo" => json!({"result": {
                    "context": {"slot": 42},
                    "value": accounts.get(request["params"][0].as_str().unwrap())
                }}),
                _ => json!({"error": {"code": -32601, "message": "Unexpected RPC request"}}),
            };
            let mut response = response;
            response["jsonrpc"] = json!("2.0");
            response["id"] = request["id"].clone();
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                response.len()
            )
            .unwrap();
        }
        methods
    });
    let output = run(&url);
    finished.store(true, Ordering::Relaxed);
    let methods = server.join().unwrap();
    assert!(
        methods
            .iter()
            .all(|method| matches!(method.as_str(), "getVersion" | "getAccountInfo")),
        "unexpected RPC calls: {methods:?}"
    );
    (output, methods)
}

fn deployed_accounts(program: &Keypair, authority: &Keypair) -> HashMap<String, Value> {
    let data_address = get_program_data_address(&program.pubkey());
    let program_data = bincode::serialize(&UpgradeableLoaderState::Program {
        programdata_address: data_address,
    })
    .unwrap();
    let metadata = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        slot: 42,
        upgrade_authority_address: Some(authority.pubkey()),
    })
    .unwrap();
    let account = |data: Vec<u8>, executable: bool| {
        json!({
            "lamports": 1_000_000, "owner": LOADER, "executable": executable,
            "rentEpoch": 0, "data": [bs58::encode(data).into_string(), "base58"]
        })
    };
    HashMap::from([
        (program.pubkey().to_string(), account(program_data, true)),
        (data_address.to_string(), account(metadata, false)),
    ])
}

fn fails_with(output: &Output, message: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(message), "{stderr}");
}

#[test]
fn interactive_roles_cache_svpi_records_and_reject_public_only_signers() {
    let workspace = Workspace::new();
    let program = Keypair::new_from_array([7; 32]);
    workspace.record("program", &program.pubkey().to_string());
    workspace.signer("shared", 8);
    let (output, methods) = with_rpc(HashMap::new(), |rpc| {
        workspace.run(&["upgrade"], "program.so\nprogram\nshared\nshared\n", rpc)
    });
    fails_with(&output, "was not found");
    assert!(methods.iter().any(|method| method == "getAccountInfo"));
    assert_eq!(workspace.gets(), "program\nshared\n");
    let prompts = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    for label in ["program", "payer", "authority"] {
        assert!(prompts.contains(label), "missing {label} prompt: {prompts}");
    }

    let (output, methods) = with_rpc(HashMap::new(), |rpc| {
        workspace.run(
            &[
                "deploy",
                "program.so",
                "--program",
                "program",
                "--payer",
                "shared",
                "--authority",
                "shared",
            ],
            "",
            rpc,
        )
    });
    fails_with(&output, "contains only a public address and cannot sign");
    assert!(methods.is_empty());
}

#[test]
fn show_resolves_a_public_program_record_without_requesting_signers() {
    let workspace = Workspace::new();
    let program = Keypair::new_from_array([7; 32]);
    let authority = Keypair::new_from_array([8; 32]);
    workspace.record("program", &program.pubkey().to_string());
    let (output, _) = with_rpc(deployed_accounts(&program, &authority), |rpc| {
        workspace.run(&["show", "--program", "program"], "", rpc)
    });
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&program.pubkey().to_string()));
    assert!(stdout.contains(&authority.pubkey().to_string()));
    assert_eq!(workspace.gets(), "program\n");
}

#[test]
fn deployment_timeout_stops_waiting_for_an_unresponsive_rpc() {
    let workspace = Workspace::new();
    workspace.signer("program", 7);
    workspace.signer("payer", 8);
    // The socket accepts connections at the OS level but never replies.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let started = Instant::now();
    let output = workspace.run(
        &[
            "deploy",
            "program.so",
            "--program",
            "program",
            "--payer",
            "payer",
            "--authority",
            "payer",
            "--timeout",
            "1",
        ],
        "",
        &url,
    );
    fails_with(&output, "Deployment timed out (limit: 1 seconds)");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(String::from_utf8_lossy(&output.stderr).contains("program show"));
}

#[test]
fn deploy_and_upgrade_reject_invalid_account_state_before_broadcast() {
    let workspace = Workspace::new();
    let program = workspace.signer("program", 7);
    workspace.signer("shared", 8);
    let other_authority = Keypair::new_from_array([9; 32]);
    for (command, accounts, error) in [
        (
            "deploy",
            deployed_accounts(&program, &other_authority),
            "already exists",
        ),
        ("upgrade", HashMap::new(), "was not found"),
        (
            "upgrade",
            deployed_accounts(&program, &other_authority),
            "authority does not match",
        ),
    ] {
        let (output, _) = with_rpc(accounts, |rpc| {
            workspace.run(
                &[
                    command,
                    "program.so",
                    "--program",
                    "program",
                    "--payer",
                    "shared",
                    "--authority",
                    "shared",
                ],
                "",
                rpc,
            )
        });
        fails_with(&output, error);
    }
}
