#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use serde_json::{json, Value};
use solana_keypair::{Keypair, Signer};

const PHRASE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn vault_response(directory: &Path, data: &str) {
    fs::write(
        directory.join("response.json"),
        serde_json::to_vec(&json!({
            "schema": "svpi.response.v2",
            "ok": true,
            "result": {
                "record": {
                    "name": "legacy-wallet", "kind": "data", "data_type": "plain",
                    "encrypted": true,
                    "encryption": {"mode": "legacy_fingerprint", "fingerprint": "1234abcd"}
                },
                "data": data
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let script = directory.join("svpi");
    fs::write(&script, "#!/bin/sh\ncat response.json\n").unwrap();
    fs::set_permissions(script, fs::Permissions::from_mode(0o700)).unwrap();
}

// Serve only balance reads on loopback. Closing each response keeps this tiny
// fixture independent of HTTP connection reuse or the client's RPC sequence.
fn balance(directory: &Path, args: &[&str]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let finished = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&finished);
    let server = thread::spawn(move || {
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
            let result = match request["method"].as_str().unwrap() {
                "eth_getBalance" => json!("0x0"),
                "getBalance" => json!({"context": {"slot": 1}, "value": 0}),
                "getAccountInfo" => json!({"context": {"slot": 1}, "value": null}),
                "getVersion" => json!({"solana-core": "2.1.0"}),
                method => panic!("unexpected RPC method: {method}"),
            };
            let response =
                json!({"jsonrpc": "2.0", "id": request["id"], "result": result}).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                response.len()
            )
            .unwrap();
        }
    });
    let binary =
        std::env::var_os("WMGR_COMPAT_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_wmgr").into());
    let output = Command::new(binary)
        .current_dir(directory)
        .arg("balance")
        .args(args)
        .args(["--rpc", &format!("http://{address}")])
        .output()
        .unwrap();
    finished.store(true, Ordering::Relaxed);
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Address: "))
        .expect("resolved wallet address")
        .to_owned()
}

const VAULT_ARGS: &[&str] = &[
    "--svpi-name",
    "legacy-wallet",
    "--svpi-pass",
    "test-only-password",
    "--svpi-cmd",
    "./svpi",
];

#[test]
fn evm_svpi_mnemonic_accepts_legacy_surrounding_whitespace() {
    let workspace = tempfile::tempdir().unwrap();
    let expected = balance(
        workspace.path(),
        &["--network", "mainnet", "--seed", PHRASE],
    );
    vault_response(workspace.path(), &format!("\n  {PHRASE}\t\n"));
    let mut args = VAULT_ARGS.to_vec();
    args.extend_from_slice(&["--network", "mainnet"]);
    assert_eq!(balance(workspace.path(), &args), expected);
}

#[test]
fn solana_legacy_keyfiles_and_vault_sources_preserve_addresses() {
    let workspace = tempfile::tempdir().unwrap();
    let keypair = Keypair::new_from_array([7; 32]);
    let expected = keypair.pubkey().to_string();
    fs::write(
        workspace.path().join("keypair.json"),
        serde_json::to_vec(keypair.to_bytes().as_slice()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        balance(workspace.path(), &["--keyfile", "keypair.json"]),
        expected
    );
    for value in [
        hex::encode(keypair.secret_bytes()),
        format!("0x{}", hex::encode(keypair.to_bytes())),
    ] {
        vault_response(workspace.path(), &format!(" \n{value}\n"));
        assert_eq!(balance(workspace.path(), VAULT_ARGS), expected);
    }
    let options = [
        "--path",
        "m/44/501/2",
        "--seed-passphrase",
        "test-only-passphrase",
    ];
    let mut seed_args = vec!["--seed", PHRASE];
    seed_args.extend_from_slice(&options);
    let expected = balance(workspace.path(), &seed_args);
    vault_response(
        workspace.path(),
        &format!(" \n{}\n", PHRASE.replace(' ', "\t  ")),
    );
    let mut vault_args = VAULT_ARGS.to_vec();
    vault_args.extend_from_slice(&options);
    assert_eq!(balance(workspace.path(), &vault_args), expected);
}
