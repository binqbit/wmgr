use std::io::{IsTerminal, Write};

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::{
    app::{
        cli::{Cli, Command},
        commands,
    },
    config::app_config::WmgrConfig,
    utils::terminal::ReplReader,
};

pub async fn run_repl(cfg: &mut WmgrConfig) -> Result<()> {
    let mut reader = ReplReader::new();

    loop {
        let line = match reader.read_line("wmgr> ") {
            Ok(Some(v)) => v,
            Ok(None) => {
                println!();
                break;
            }
            Err(err) => {
                eprintln!("device_error: Failed to read command: {err}");
                break;
            }
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let lowered = input.to_ascii_lowercase();
        if matches!(lowered.as_str(), "exit" | "quit" | "q") {
            break;
        }
        if matches!(lowered.as_str(), "clear" | "cls") {
            reader.clear_history();
            if std::io::stdout().is_terminal() {
                print!("\x1B[3J\x1B[2J");
                print!("\x1B[3J\x1B[H");
                let _ = std::io::stdout().flush();
            }
            continue;
        }
        if matches!(lowered.as_str(), "help" | "?") {
            let mut cmd = Cli::command();
            let _ = cmd.print_help();
            println!();
            continue;
        }

        reader.add_history_entry(input);

        let Some(tokens) = shlex::split(input) else {
            eprintln!("invalid_argument: Failed to parse command line");
            continue;
        };

        let mut argv = Vec::with_capacity(tokens.len() + 1);
        argv.push("wmgr".to_string());
        argv.extend(tokens);

        let parsed = match Cli::try_parse_from(argv) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        let Some(cmd) = parsed.command else {
            let mut root = Cli::command();
            let _ = root.print_help();
            println!();
            continue;
        };

        if matches!(cmd, Command::Repl) {
            eprintln!("invalid_argument: repl is already running");
            continue;
        }

        if let Err(err) = commands::run_command(cmd, cfg).await {
            eprintln!("Error: {err}");
        }
    }

    Ok(())
}
