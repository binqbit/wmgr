use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::{IsTerminal, Write};

pub enum ReplReader {
    Rustyline(DefaultEditor),
    Plain,
}

impl ReplReader {
    pub fn new() -> Self {
        if std::io::stdin().is_terminal() {
            match DefaultEditor::new() {
                Ok(v) => return Self::Rustyline(v),
                Err(err) => {
                    eprintln!("device_error: Failed to initialize interactive console: {err}");
                }
            }
        }

        Self::Plain
    }

    pub fn read_line(&mut self, prompt: &str) -> Result<Option<String>, String> {
        match self {
            ReplReader::Rustyline(rl) => match rl.readline(prompt) {
                Ok(v) => Ok(Some(v)),
                Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => Ok(None),
                Err(err) => Err(err.to_string()),
            },
            ReplReader::Plain => {
                print!("{prompt}");
                std::io::stdout().flush().map_err(|err| err.to_string())?;

                let stdin = std::io::stdin();
                let mut line = String::new();
                match stdin.read_line(&mut line) {
                    Ok(0) => Ok(None),
                    Ok(_) => Ok(Some(line)),
                    Err(err) => Err(err.to_string()),
                }
            }
        }
    }

    pub fn add_history_entry(&mut self, line: &str) {
        if let ReplReader::Rustyline(rl) = self {
            let _ = rl.add_history_entry(line);
        }
    }

    pub fn clear_history(&mut self) {
        if let ReplReader::Rustyline(rl) = self {
            let _ = rl.clear_history();
        }
    }
}
