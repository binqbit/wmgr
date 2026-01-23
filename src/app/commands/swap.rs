use std::str::FromStr;

use anyhow::{anyhow, Result};
use raydium_amm_swap::amm::client::AmmSwapClient;
use raydium_amm_swap::consts::SOL_MINT;
use raydium_amm_swap::interface::{AmmPool, PoolKeys};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::app::cli::{SwapToken, TradeArgs};
use crate::app::commands::commitment_from_arg;
use crate::config::clusters::get_cluster_config;
use crate::config::raydium::{SOL_USDC_POOL_ID, USDC_MINT};
use crate::core::amount::{format_integer_amount, parse_amount_to_u64};
use crate::infra::keys::solana::resolve_solana_keypair;
use crate::infra::raydium::{
    build_swap_instructions, compute_swap_quote, compute_swap_quote_out, SwapKind,
};
use crate::utils::prompt::prompt;

#[derive(Clone, Copy)]
enum TradeSide {
    Buy,
    Sell,
}

struct TradeSummary {
    side: TradeSide,
    input_symbol: &'static str,
    output_symbol: &'static str,
    input_amount_ui: String,
    input_max_ui: String,
    output_expected_ui: String,
    output_min_ui: String,
    fee_ui: String,
    slippage_percent: f64,
    price: f64,
    price_impact: f64,
}

impl TradeSummary {
    fn sell(
        input_amount: u64,
        output_expected: u64,
        output_min: u64,
        fee: u64,
        input_decimals: u8,
        output_decimals: u8,
        input_token: SwapToken,
        output_token: SwapToken,
        slippage_percent: f64,
        price: f64,
        price_impact: f64,
    ) -> Self {
        Self {
            side: TradeSide::Sell,
            input_symbol: input_token.symbol(),
            output_symbol: output_token.symbol(),
            input_amount_ui: format_integer_amount(input_amount as u128, input_decimals),
            input_max_ui: format_integer_amount(input_amount as u128, input_decimals),
            output_expected_ui: format_integer_amount(output_expected as u128, output_decimals),
            output_min_ui: format_integer_amount(output_min as u128, output_decimals),
            fee_ui: format_integer_amount(fee as u128, input_decimals),
            slippage_percent,
            price,
            price_impact,
        }
    }

    fn buy(
        output_amount: u64,
        input_expected: u64,
        input_max: u64,
        fee: u64,
        input_decimals: u8,
        output_decimals: u8,
        input_token: SwapToken,
        output_token: SwapToken,
        slippage_percent: f64,
        price: f64,
        price_impact: f64,
    ) -> Self {
        Self {
            side: TradeSide::Buy,
            input_symbol: input_token.symbol(),
            output_symbol: output_token.symbol(),
            input_amount_ui: format_integer_amount(input_expected as u128, input_decimals),
            input_max_ui: format_integer_amount(input_max as u128, input_decimals),
            output_expected_ui: format_integer_amount(output_amount as u128, output_decimals),
            output_min_ui: format_integer_amount(output_amount as u128, output_decimals),
            fee_ui: format_integer_amount(fee as u128, input_decimals),
            slippage_percent,
            price,
            price_impact,
        }
    }

    fn print(&self) {
        const LABEL_WIDTH: usize = 16;
        const VALUE_WIDTH: usize = 20;
        enum RowValue {
            Amount { amount: String, symbol: String },
            Text(String),
        }

        struct Row {
            label: &'static str,
            value: RowValue,
        }

        let mut rows: Vec<Row> = Vec::new();

        match self.side {
            TradeSide::Sell => {
                rows.push(Row {
                    label: "Sell",
                    value: RowValue::Amount {
                        amount: self.input_amount_ui.trim().to_string(),
                        symbol: self.input_symbol.to_string(),
                    },
                });
                rows.push(Row {
                    label: "Min receive",
                    value: RowValue::Amount {
                        amount: self.output_min_ui.trim().to_string(),
                        symbol: self.output_symbol.to_string(),
                    },
                });
                rows.push(Row {
                    label: "Fee",
                    value: RowValue::Amount {
                        amount: self.fee_ui.trim().to_string(),
                        symbol: self.input_symbol.to_string(),
                    },
                });
            }
            TradeSide::Buy => {
                rows.push(Row {
                    label: "Buy",
                    value: RowValue::Amount {
                        amount: self.output_expected_ui.trim().to_string(),
                        symbol: self.output_symbol.to_string(),
                    },
                });
                rows.push(Row {
                    label: "Max spend",
                    value: RowValue::Amount {
                        amount: self.input_max_ui.trim().to_string(),
                        symbol: self.input_symbol.to_string(),
                    },
                });
                rows.push(Row {
                    label: "Fee",
                    value: RowValue::Amount {
                        amount: self.fee_ui.trim().to_string(),
                        symbol: self.input_symbol.to_string(),
                    },
                });
            }
        }

        rows.push(Row {
            label: "Price",
            value: RowValue::Amount {
                amount: format!("{:.8}", self.price),
                symbol: format!("{} per {}", self.output_symbol, self.input_symbol),
            },
        });
        rows.push(Row {
            label: "Impact",
            value: RowValue::Text(format!("{:.4}%", self.price_impact)),
        });
        rows.push(Row {
            label: "Slippage",
            value: RowValue::Text(format!("{}%", self.slippage_percent)),
        });

        for row in rows {
            match row.value {
                RowValue::Amount { amount, symbol } => {
                    println!(
                        "{:<label_width$} {:<value_width$} {}",
                        format!("{}:", row.label),
                        amount,
                        symbol,
                        label_width = LABEL_WIDTH,
                        value_width = VALUE_WIDTH
                    );
                }
                RowValue::Text(value) => {
                    println!(
                        "{:<label_width$} {}",
                        format!("{}:", row.label),
                        value,
                        label_width = LABEL_WIDTH
                    );
                }
            }
        }
    }
}

pub async fn handle_buy(args: TradeArgs) -> Result<()> {
    handle_trade(TradeSide::Buy, args).await
}

pub async fn handle_sell(args: TradeArgs) -> Result<()> {
    handle_trade(TradeSide::Sell, args).await
}

async fn handle_trade(side: TradeSide, args: TradeArgs) -> Result<()> {
    let keypair = resolve_solana_keypair(&args.key)?;
    let keypair_copy = solana_keypair::Keypair::try_from(keypair.to_bytes().as_slice())
        .map_err(|err| anyhow!("Failed to clone keypair: {err}"))?;

    let commitment = commitment_from_arg(args.rpc.commitment);
    let cluster = get_cluster_config(&args.rpc.cluster, args.rpc.rpc.as_deref())?;
    println!("Using cluster: {}, RPC: {}", cluster.name, cluster.rpc_url);

    let rpc_for_client = RpcClient::new_with_commitment(cluster.rpc_url.clone(), commitment);
    let rpc = RpcClient::new_with_commitment(cluster.rpc_url.clone(), commitment);

    let client = AmmSwapClient::new(rpc_for_client, keypair_copy);

    let pool_id = Pubkey::from_str(SOL_USDC_POOL_ID)
        .map_err(|err| anyhow!("Invalid Raydium pool id: {err}"))?;
    let pool_keys: PoolKeys<AmmPool> = client.fetch_pools_keys_by_id(&pool_id).await?;
    let pool = pool_keys
        .data
        .get(0)
        .ok_or_else(|| anyhow!("Raydium pool keys not found"))?;

    if !pool_mints_match(pool) {
        return Err(anyhow!("Raydium pool mints do not match SOL/USDC"));
    }

    let rpc_pool = client.get_rpc_pool_info(&pool_id).await?;
    let base_mint = pool.mint_a.address.as_str();
    let quote_mint = pool.mint_b.address.as_str();
    let base_decimals = u8::try_from(pool.mint_a.decimals)
        .map_err(|_| anyhow!("Base mint decimals out of range"))?;
    let quote_decimals = u8::try_from(pool.mint_b.decimals)
        .map_err(|_| anyhow!("Quote mint decimals out of range"))?;

    let slippage_percent = args.slippage;
    if !(0.0..=100.0).contains(&slippage_percent) {
        return Err(anyhow!("Slippage percent must be between 0 and 100"));
    }

    let (input_token, output_token) = match side {
        TradeSide::Buy => (args.token.other(), args.token),
        TradeSide::Sell => (args.token, args.token.other()),
    };

    let input_mint = mint_for_token(input_token);
    let output_mint = mint_for_token(output_token);

    let input_is_base = input_mint == base_mint;
    let output_is_base = output_mint == base_mint;
    if !input_is_base && input_mint != quote_mint {
        return Err(anyhow!("Input mint not found in Raydium pool"));
    }
    if !output_is_base && output_mint != quote_mint {
        return Err(anyhow!("Output mint not found in Raydium pool"));
    }

    let (swap_kind, summary) = match side {
        TradeSide::Sell => {
            let input_decimals = if input_is_base {
                base_decimals
            } else {
                quote_decimals
            };
            let output_decimals = if input_is_base {
                quote_decimals
            } else {
                base_decimals
            };
            let reserve_in = if input_is_base {
                rpc_pool.base_reserve
            } else {
                rpc_pool.quote_reserve
            };
            let reserve_out = if input_is_base {
                rpc_pool.quote_reserve
            } else {
                rpc_pool.base_reserve
            };

            let amount_in = parse_amount_to_u64(&args.amount, input_decimals)?;
            let quote = compute_swap_quote(
                amount_in,
                reserve_in,
                reserve_out,
                input_decimals,
                output_decimals,
            )?;
            let min_amount_out = apply_slippage_min(quote.amount_out, slippage_percent);
            let swap_kind = SwapKind::BaseIn {
                amount_in,
                min_amount_out,
            };

            let output_expected = quote.amount_out;
            let summary = TradeSummary::sell(
                amount_in,
                output_expected,
                min_amount_out,
                quote.fee,
                input_decimals,
                output_decimals,
                input_token,
                output_token,
                slippage_percent,
                quote.price,
                quote.price_impact,
            );
            (swap_kind, summary)
        }
        TradeSide::Buy => {
            let output_decimals = if output_is_base {
                base_decimals
            } else {
                quote_decimals
            };
            let input_decimals = if output_is_base {
                quote_decimals
            } else {
                base_decimals
            };
            let reserve_in = if output_is_base {
                rpc_pool.quote_reserve
            } else {
                rpc_pool.base_reserve
            };
            let reserve_out = if output_is_base {
                rpc_pool.base_reserve
            } else {
                rpc_pool.quote_reserve
            };

            let amount_out = parse_amount_to_u64(&args.amount, output_decimals)?;
            let quote = compute_swap_quote_out(
                amount_out,
                reserve_in,
                reserve_out,
                input_decimals,
                output_decimals,
            )?;
            let max_amount_in = apply_slippage_max(quote.amount_in, slippage_percent);

            let swap_kind = SwapKind::BaseOut {
                max_amount_in,
                amount_out,
            };

            let summary = TradeSummary::buy(
                amount_out,
                quote.amount_in,
                max_amount_in,
                quote.fee,
                input_decimals,
                output_decimals,
                input_token,
                output_token,
                slippage_percent,
                quote.price,
                quote.price_impact,
            );
            (swap_kind, summary)
        }
    };

    let input_mint_key =
        Pubkey::from_str(input_mint).map_err(|err| anyhow!("Invalid input mint: {err}"))?;
    let output_mint_key =
        Pubkey::from_str(output_mint).map_err(|err| anyhow!("Invalid output mint: {err}"))?;
    ensure_spl_token_mint(&rpc, &input_mint_key, "input").await?;
    ensure_spl_token_mint(&rpc, &output_mint_key, "output").await?;

    let instructions = build_swap_instructions(
        pool,
        &keypair.pubkey(),
        &input_mint_key,
        &output_mint_key,
        swap_kind,
    )?;

    println!("Simulating swap...");
    simulate_transaction(&rpc, &keypair, &instructions).await?;
    println!("Simulation: ok");

    summary.print();
    let answer = prompt("Continue? (y/N):")?;
    let answer = answer.trim().to_ascii_lowercase();
    if answer != "y" && answer != "yes" {
        println!("Aborted.");
        return Ok(());
    }

    let sig = send_transaction(&rpc, &keypair, &instructions).await?;
    println!("SUCCESS: Swap signature: {sig}");
    Ok(())
}

fn mint_for_token(token: SwapToken) -> &'static str {
    match token {
        SwapToken::Sol => SOL_MINT,
        SwapToken::Usdc => USDC_MINT,
    }
}

fn apply_slippage_min(amount: u64, slippage_percent: f64) -> u64 {
    let slippage = slippage_percent / 100.0;
    ((amount as f64) * (1.0 - slippage)).floor() as u64
}

fn apply_slippage_max(amount: u64, slippage_percent: f64) -> u64 {
    let slippage = slippage_percent / 100.0;
    ((amount as f64) * (1.0 + slippage)).ceil() as u64
}

fn pool_mints_match(pool: &AmmPool) -> bool {
    let a = pool.mint_a.address.as_str();
    let b = pool.mint_b.address.as_str();
    (a == SOL_MINT && b == USDC_MINT) || (a == USDC_MINT && b == SOL_MINT)
}

async fn simulate_transaction(
    rpc: &RpcClient,
    keypair: &solana_keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<()> {
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&keypair.pubkey()),
        &[keypair],
        recent_blockhash,
    );

    let result = rpc.simulate_transaction(&tx).await?;
    if let Some(err) = result.value.err {
        if let Some(logs) = result.value.logs {
            for log in logs {
                println!("{log}");
            }
        }
        return Err(anyhow!("Simulation failed: {err:?}"));
    }
    Ok(())
}

async fn send_transaction(
    rpc: &RpcClient,
    keypair: &solana_keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<String> {
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&keypair.pubkey()),
        &[keypair],
        recent_blockhash,
    );
    let sig = rpc.send_and_confirm_transaction(&tx).await?;
    Ok(sig.to_string())
}

async fn ensure_spl_token_mint(rpc: &RpcClient, mint: &Pubkey, label: &str) -> Result<()> {
    let account = rpc.get_account(mint).await?;
    if account.owner != spl_token::id() {
        return Err(anyhow!(
            "{label} mint {} is not owned by the SPL Token program (owner: {}). Raydium AMM v4 only supports SPL Token mints.",
            mint,
            account.owner
        ));
    }
    Ok(())
}
