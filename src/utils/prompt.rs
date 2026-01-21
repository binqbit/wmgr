use std::io::{self, Write};

use anyhow::{anyhow, Result};

pub fn prompt(message: &str) -> Result<String> {
    eprint!("{message} ");
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn prompt_hidden(message: &str) -> Result<String> {
    let prompt = format!("{message} ");
    let value = rpassword::prompt_password(prompt)
        .map_err(|err| anyhow!("Failed to read password: {err}"))?;
    Ok(value.trim().to_string())
}
