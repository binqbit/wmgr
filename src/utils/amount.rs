use anyhow::{anyhow, Result};

pub fn parse_amount_to_u128(amount: &str, decimals: u8) -> Result<u128> {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Amount is empty"));
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(anyhow!("Invalid amount format: {amount}"));
    }

    let mut parts = trimmed.split('.');
    let left_raw = parts.next().unwrap_or("");
    let right_raw = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return Err(anyhow!("Invalid amount format: {amount}"));
    }
    if !left_raw.chars().all(|c| c.is_ascii_digit())
        || !right_raw.chars().all(|c| c.is_ascii_digit())
    {
        return Err(anyhow!("Invalid amount format: {amount}"));
    }

    if decimals == 0 {
        let left = left_raw.trim_start_matches('0');
        let digits = if left.is_empty() { "0" } else { left };
        return Ok(digits.parse()?);
    }

    let left = left_raw.trim_start_matches('0');
    let left_norm = if left.is_empty() { "0" } else { left };

    let mut right = right_raw.to_string();
    let target = decimals as usize;
    if right.len() > target {
        right.truncate(target);
    } else if right.len() < target {
        right.push_str(&"0".repeat(target - right.len()));
    }

    let digits = format!("{left_norm}{right}");
    let digits_trimmed = digits.trim_start_matches('0');
    let final_digits = if digits_trimmed.is_empty() {
        "0"
    } else {
        digits_trimmed
    };
    Ok(final_digits.parse()?)
}

pub fn parse_amount_to_u64(amount: &str, decimals: u8) -> Result<u64> {
    let value = parse_amount_to_u128(amount, decimals)?;
    if value > u64::MAX as u128 {
        return Err(anyhow!("Amount exceeds u64 limit"));
    }
    Ok(value as u64)
}

pub fn format_integer_amount(amount: u128, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32);
    let whole = amount / factor;
    let frac = amount % factor;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_str = format!("{:0width$}", frac, width = decimals as usize);
    while frac_str.ends_with('0') {
        frac_str.pop();
    }
    format!("{whole}.{frac_str}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts() {
        assert_eq!(parse_amount_to_u128("1", 6).unwrap(), 1_000_000);
        assert_eq!(parse_amount_to_u128("0.5", 6).unwrap(), 500_000);
        assert_eq!(parse_amount_to_u128("12.3456", 2).unwrap(), 1234);
        assert_eq!(parse_amount_to_u128("0.000000001", 9).unwrap(), 1);
    }

    #[test]
    fn formats_amounts() {
        assert_eq!(format_integer_amount(1_000_000, 6), "1");
        assert_eq!(format_integer_amount(500_000, 6), "0.5");
        assert_eq!(format_integer_amount(1234, 2), "12.34");
    }
}
