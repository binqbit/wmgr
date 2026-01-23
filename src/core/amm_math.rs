use anyhow::{anyhow, Result};
use raydium_amm_swap::consts::{LIQUIDITY_FEES_DENOMINATOR, LIQUIDITY_FEES_NUMERATOR};

pub struct SwapQuote {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price: f64,
    pub price_impact: f64,
}

pub fn compute_swap_quote(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    decimals_in: u8,
    decimals_out: u8,
) -> Result<SwapQuote> {
    validate_exact_in(amount_in, reserve_in, reserve_out)?;

    let fee = fee_for_amount(amount_in);
    let amount_in_with_fee = amount_in.saturating_sub(fee);
    let amount_out = constant_product_out(amount_in_with_fee, reserve_in, reserve_out)?;

    let price = current_price(reserve_in, reserve_out, decimals_in, decimals_out);
    let execution_price =
        execution_price(amount_in_with_fee, amount_out, decimals_in, decimals_out);
    let price_impact = price_impact(price, execution_price);

    Ok(SwapQuote {
        amount_in,
        amount_out,
        fee,
        price,
        price_impact,
    })
}

pub fn compute_swap_quote_out(
    amount_out: u64,
    reserve_in: u64,
    reserve_out: u64,
    decimals_in: u8,
    decimals_out: u8,
) -> Result<SwapQuote> {
    validate_exact_out(amount_out, reserve_in, reserve_out)?;

    let amount_in_with_fee = constant_product_in(amount_out, reserve_in, reserve_out)?;
    let (amount_in, fee) = gross_input_from_net(amount_in_with_fee);
    let net_input = amount_in.saturating_sub(fee);

    let price = current_price(reserve_in, reserve_out, decimals_in, decimals_out);
    let execution_price = execution_price(net_input, amount_out, decimals_in, decimals_out);
    let price_impact = price_impact(price, execution_price);

    Ok(SwapQuote {
        amount_in,
        amount_out,
        fee,
        price,
        price_impact,
    })
}

fn validate_exact_in(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Result<()> {
    if amount_in == 0 {
        return Err(anyhow!("Amount must be greater than zero"));
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err(anyhow!("Pool reserves are empty"));
    }
    Ok(())
}

fn validate_exact_out(amount_out: u64, reserve_in: u64, reserve_out: u64) -> Result<()> {
    if amount_out == 0 {
        return Err(anyhow!("Amount must be greater than zero"));
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err(anyhow!("Pool reserves are empty"));
    }
    if amount_out >= reserve_out {
        return Err(anyhow!("Requested amount exceeds pool reserve"));
    }
    Ok(())
}

fn constant_product_out(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Result<u64> {
    let amount_in_u128 = amount_in as u128;
    let reserve_in_u128 = reserve_in as u128;
    let reserve_out_u128 = reserve_out as u128;
    let denominator = reserve_in_u128 + amount_in_u128;
    if denominator == 0 {
        return Err(anyhow!("Pool reserves are empty"));
    }
    let amount_out_u128 = (amount_in_u128 * reserve_out_u128) / denominator;
    if amount_out_u128 > u64::MAX as u128 {
        return Err(anyhow!("Amount exceeds u64 limit"));
    }
    Ok(amount_out_u128 as u64)
}

fn fee_for_amount(amount_in: u64) -> u64 {
    if amount_in == 0 {
        return 0;
    }
    let numerator = amount_in as u128 * LIQUIDITY_FEES_NUMERATOR as u128;
    let denominator = LIQUIDITY_FEES_DENOMINATOR as u128;
    ((numerator + denominator - 1) / denominator) as u64
}

fn constant_product_in(amount_out: u64, reserve_in: u64, reserve_out: u64) -> Result<u64> {
    let reserve_in_u128 = reserve_in as u128;
    let reserve_out_u128 = reserve_out as u128;
    let amount_out_u128 = amount_out as u128;
    let denominator = reserve_out_u128.saturating_sub(amount_out_u128);
    if denominator == 0 {
        return Err(anyhow!("Requested amount exceeds pool reserve"));
    }
    let numerator = reserve_in_u128 * amount_out_u128;
    let amount_in_u128 = ceil_div(numerator, denominator);
    if amount_in_u128 == 0 {
        return Err(anyhow!("Amount too small for pool reserves"));
    }
    if amount_in_u128 > u64::MAX as u128 {
        return Err(anyhow!("Amount exceeds u64 limit"));
    }
    Ok(amount_in_u128 as u64)
}

fn gross_input_from_net(amount_in_with_fee: u64) -> (u64, u64) {
    let numerator = amount_in_with_fee as u128 * LIQUIDITY_FEES_DENOMINATOR as u128;
    let denominator = (LIQUIDITY_FEES_DENOMINATOR - LIQUIDITY_FEES_NUMERATOR) as u128;
    let mut amount_in = ceil_div(numerator, denominator) as u64;
    let mut fee = fee_for_amount(amount_in);
    if amount_in.saturating_sub(fee) < amount_in_with_fee {
        amount_in = amount_in.saturating_add(1);
        fee = fee_for_amount(amount_in);
    }
    (amount_in, fee)
}

fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    if denominator == 0 {
        return 0;
    }
    (numerator + denominator - 1) / denominator
}

fn current_price(reserve_in: u64, reserve_out: u64, decimals_in: u8, decimals_out: u8) -> f64 {
    let div_in = 10u128.pow(decimals_in as u32) as f64;
    let div_out = 10u128.pow(decimals_out as u32) as f64;
    let reserve_in_f = reserve_in as f64 / div_in;
    let reserve_out_f = reserve_out as f64 / div_out;
    if reserve_in_f == 0.0 {
        0.0
    } else {
        reserve_out_f / reserve_in_f
    }
}

fn execution_price(amount_in: u64, amount_out: u64, decimals_in: u8, decimals_out: u8) -> f64 {
    let div_in = 10u128.pow(decimals_in as u32) as f64;
    let div_out = 10u128.pow(decimals_out as u32) as f64;
    let input_f = amount_in as f64 / div_in;
    let output_f = amount_out as f64 / div_out;
    if input_f == 0.0 {
        0.0
    } else {
        output_f / input_f
    }
}

fn price_impact(current_price: f64, execution_price: f64) -> f64 {
    if current_price == 0.0 {
        0.0
    } else {
        (current_price - execution_price) / current_price * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_in_constant_product() {
        let quote = compute_swap_quote(100, 1_000, 1_000, 0, 0).unwrap();
        assert_eq!(quote.amount_out, 90);
    }

    #[test]
    fn exact_out_constant_product() {
        let quote = compute_swap_quote_out(100, 1_000, 1_000, 0, 0).unwrap();
        assert_eq!(quote.amount_in, 113);
    }
}
