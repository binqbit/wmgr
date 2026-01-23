use std::str::FromStr;

use anyhow::{anyhow, Result};
use raydium_amm_swap::amm::{AmmInstruction, SwapInstructionBaseIn, SwapInstructionBaseOut};
use raydium_amm_swap::interface::AmmPool;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;

pub use crate::core::amm_math::{compute_swap_quote, compute_swap_quote_out};

pub enum SwapKind {
    BaseIn { amount_in: u64, min_amount_out: u64 },
    BaseOut { max_amount_in: u64, amount_out: u64 },
}

impl SwapKind {
    pub fn input_amount(&self) -> u64 {
        match self {
            SwapKind::BaseIn { amount_in, .. } => *amount_in,
            SwapKind::BaseOut { max_amount_in, .. } => *max_amount_in,
        }
    }
}

pub fn build_swap_instructions(
    pool: &AmmPool,
    owner: &Pubkey,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
    swap: SwapKind,
) -> Result<Vec<Instruction>> {
    let source_ata = get_associated_token_address(owner, input_mint);
    let destination_ata = get_associated_token_address(owner, output_mint);

    let mut instructions = Vec::new();
    instructions.push(create_associated_token_account_idempotent(
        owner,
        owner,
        input_mint,
        &spl_token::id(),
    ));
    if destination_ata != source_ata {
        instructions.push(create_associated_token_account_idempotent(
            owner,
            owner,
            output_mint,
            &spl_token::id(),
        ));
    }

    let input_amount = swap.input_amount();
    if *input_mint == spl_token::native_mint::id() {
        instructions.push(system_instruction::transfer(
            owner,
            &source_ata,
            input_amount,
        ));
        instructions.push(spl_token::instruction::sync_native(
            &spl_token::id(),
            &source_ata,
        )?);
    }

    let amm_program = parse_pubkey("amm program", &pool.program_id)?;
    let amm_id = parse_pubkey("amm id", &pool.id)?;
    let authority = parse_pubkey("amm authority", &pool.authority)?;
    let open_orders = parse_pubkey("amm open orders", &pool.open_orders)?;
    let coin_vault = parse_pubkey("amm coin vault", &pool.vault.a)?;
    let pc_vault = parse_pubkey("amm pc vault", &pool.vault.b)?;
    let market_program = parse_pubkey("market program", &pool.market_program_id)?;
    let market_id = parse_pubkey("market id", &pool.market_id)?;
    let market_bids = parse_pubkey("market bids", &pool.market_bids)?;
    let market_asks = parse_pubkey("market asks", &pool.market_asks)?;
    let market_event_queue = parse_pubkey("market event queue", &pool.market_event_queue)?;
    let market_coin_vault = parse_pubkey("market base vault", &pool.market_base_vault)?;
    let market_pc_vault = parse_pubkey("market quote vault", &pool.market_quote_vault)?;
    let market_authority = parse_pubkey("market authority", &pool.market_authority)?;

    let data = match swap {
        SwapKind::BaseIn {
            amount_in,
            min_amount_out,
        } => AmmInstruction::SwapBaseIn(SwapInstructionBaseIn {
            amount_in,
            minimum_amount_out: min_amount_out,
        }),
        SwapKind::BaseOut {
            max_amount_in,
            amount_out,
        } => AmmInstruction::SwapBaseOut(SwapInstructionBaseOut {
            max_amount_in,
            amount_out,
        }),
    }
    .pack()
    .map_err(|err| anyhow!("Failed to pack swap instruction: {err}"))?;

    let accounts = vec![
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(amm_id, false),
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new(open_orders, false),
        AccountMeta::new(coin_vault, false),
        AccountMeta::new(pc_vault, false),
        AccountMeta::new_readonly(market_program, false),
        AccountMeta::new(market_id, false),
        AccountMeta::new(market_bids, false),
        AccountMeta::new(market_asks, false),
        AccountMeta::new(market_event_queue, false),
        AccountMeta::new(market_coin_vault, false),
        AccountMeta::new(market_pc_vault, false),
        AccountMeta::new_readonly(market_authority, false),
        AccountMeta::new(source_ata, false),
        AccountMeta::new(destination_ata, false),
        AccountMeta::new_readonly(*owner, true),
    ];

    let swap_ix = Instruction {
        program_id: amm_program,
        accounts,
        data,
    };

    instructions.push(swap_ix);

    Ok(instructions)
}

fn parse_pubkey(label: &str, value: &str) -> Result<Pubkey> {
    Pubkey::from_str(value).map_err(|err| anyhow!("Invalid {label}: {err}"))
}
