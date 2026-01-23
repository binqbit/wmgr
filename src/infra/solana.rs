use std::str::FromStr;

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use solana_system_interface::instruction as system_instruction;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;
use spl_token::instruction::transfer_checked;
use spl_token::state::{Account as TokenAccount, Mint};

use crate::core::amount::{format_integer_amount, parse_amount_to_u64};

pub struct BalanceResult {
    pub address: String,
    pub sol_lamports: u64,
    pub sol: String,
    pub usdc_raw: u64,
    pub usdc: String,
    pub usdc_decimals: u8,
}

pub fn create_rpc_client(rpc_url: &str, commitment: CommitmentConfig) -> RpcClient {
    RpcClient::new_with_commitment(rpc_url.to_string(), commitment)
}

pub fn transfer_sol(client: &RpcClient, from: &Keypair, to: &str, amount: &str) -> Result<String> {
    let to_pubkey =
        Pubkey::from_str(to).map_err(|err| anyhow!("Invalid recipient address: {err}"))?;
    let lamports = parse_amount_to_u64(amount, 9)?;

    let instruction = system_instruction::transfer(&from.pubkey(), &to_pubkey, lamports);
    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&from.pubkey()),
        &[from],
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx)?;
    Ok(sig.to_string())
}

pub fn transfer_spl_token(
    client: &RpcClient,
    from: &Keypair,
    to: &str,
    amount: &str,
    mint: &str,
) -> Result<String> {
    let to_owner =
        Pubkey::from_str(to).map_err(|err| anyhow!("Invalid recipient address: {err}"))?;
    let mint_pubkey =
        Pubkey::from_str(mint).map_err(|err| anyhow!("Invalid mint address: {err}"))?;

    let decimals = get_mint_decimals(client, &mint_pubkey).unwrap_or(6);
    let amount_u64 = parse_amount_to_u64(amount, decimals)?;

    let from_ata = get_associated_token_address(&from.pubkey(), &mint_pubkey);
    let to_ata = get_associated_token_address(&to_owner, &mint_pubkey);

    let mut instructions = Vec::new();
    instructions.push(create_associated_token_account_idempotent(
        &from.pubkey(),
        &from.pubkey(),
        &mint_pubkey,
        &spl_token::id(),
    ));
    instructions.push(create_associated_token_account_idempotent(
        &from.pubkey(),
        &to_owner,
        &mint_pubkey,
        &spl_token::id(),
    ));
    instructions.push(transfer_checked(
        &spl_token::id(),
        &from_ata,
        &mint_pubkey,
        &to_ata,
        &from.pubkey(),
        &[],
        amount_u64,
        decimals,
    )?);

    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&from.pubkey()),
        &[from],
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx)?;
    Ok(sig.to_string())
}

pub fn get_balances(client: &RpcClient, owner: &Pubkey, mint: &Pubkey) -> Result<BalanceResult> {
    let lamports = client.get_balance(owner)?;
    let sol = format_integer_amount(lamports as u128, 9);

    let decimals = get_mint_decimals(client, mint).unwrap_or(6);

    let ata = get_associated_token_address(owner, mint);
    let usdc_raw = match client.get_account(&ata) {
        Ok(account) => TokenAccount::unpack(&account.data)?.amount,
        Err(_) => 0,
    };
    let usdc = format_integer_amount(usdc_raw as u128, decimals);

    Ok(BalanceResult {
        address: owner.to_string(),
        sol_lamports: lamports,
        sol,
        usdc_raw,
        usdc,
        usdc_decimals: decimals,
    })
}

fn get_mint_decimals(client: &RpcClient, mint: &Pubkey) -> Result<u8> {
    let account = client.get_account(mint)?;
    let mint_state = Mint::unpack(&account.data)?;
    Ok(mint_state.decimals)
}
