use super::*;
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    sysvar::clock::Clock,
};
use anchor_spl::{token, associated_token};

// Anchor discriminator calculation
fn get_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(
        &solana_sdk::hash::hash(preimage.as_bytes()).to_bytes()[..8],
    );
    sighash
}

// Simple SPL token mint/transfer utilities
fn create_mint_ix(payer: &Pubkey, mint: &Pubkey, authority: &Pubkey, decimals: u8) -> Vec<Instruction> {
    // solana_sdk system program create account
    let create_acc_ix = solana_sdk::system_instruction::create_account(
        payer,
        mint,
        10_000_000,
        82, // Mint size
        &to_sdk_pubkey(&token::ID),
    );
    
    // anchor_spl token initialize_mint
    let mut init_mint_data = vec![0]; // InitializeMint2 instruction index is 0 for InitializeMint, 20 for InitializeMint2. Let's use InitializeMint which is 0
    init_mint_data.push(decimals);
    init_mint_data.extend_from_slice(authority.as_ref());
    init_mint_data.push(0); // no freeze authority

    let init_mint_ix = Instruction {
        program_id: to_sdk_pubkey(&token::ID),
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
        ],
        data: init_mint_data,
    };

    vec![create_acc_ix, init_mint_ix]
}

fn mint_to_ix(mint: &Pubkey, ata: &Pubkey, authority: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![7]; // MintTo instruction index is 7
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: to_sdk_pubkey(&token::ID),
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*ata, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

// Pubkey converters because of solana_sdk and solana_program version mismatch
fn to_sdk_pubkey(p: &anchor_lang::prelude::Pubkey) -> Pubkey {
    Pubkey::new_from_array(p.to_bytes())
}

fn get_vesting_pda(company_name: &str) -> (Pubkey, u8) {
    let pda = Pubkey::find_program_address(&[company_name.as_bytes()], &to_sdk_pubkey(&crate::ID));
    (pda.0, pda.1)
}

fn get_treasury_pda(company_name: &str) -> (Pubkey, u8) {
    let pda = Pubkey::find_program_address(&[b"vesting_treasury", company_name.as_bytes()], &to_sdk_pubkey(&crate::ID));
    (pda.0, pda.1)
}

fn get_employee_pda(beneficiary: &Pubkey, vesting_pda: &Pubkey) -> (Pubkey, u8) {
    let pda = Pubkey::find_program_address(
        &[
            b"employee_vesting",
            beneficiary.as_ref(),
            vesting_pda.as_ref(),
        ],
        &to_sdk_pubkey(&crate::ID),
    );
    (pda.0, pda.1)
}

fn get_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let pda = Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            to_sdk_pubkey(&token::ID).as_ref(),
            mint.as_ref(),
        ],
        &to_sdk_pubkey(&associated_token::ID),
    );
    pda.0
}

#[test]
fn test_vesting_flow() {
    let mut svm = LiteSVM::new();

    let owner = Keypair::new();
    let employee = Keypair::new();
    let mint_keypair = Keypair::new();
    
    // Give owner and employee some SOL
    svm.airdrop(&owner.pubkey(), 10 * 1_000_000_000).unwrap();
    svm.airdrop(&employee.pubkey(), 10 * 1_000_000_000).unwrap();

    // Add program to LiteSVM
    let program_bytes = include_bytes!("../../../target/deploy/vesting.so");
    svm.add_program(to_sdk_pubkey(&crate::ID), program_bytes).unwrap();

    let company_name = "TestCompany".to_string();
    let decimals = 9;

    // 1. Create Mint
    let mint_ixs = create_mint_ix(&owner.pubkey(), &mint_keypair.pubkey(), &owner.pubkey(), decimals);
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &mint_ixs,
        Some(&owner.pubkey()),
        &[&owner, &mint_keypair],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // 2. Create Vesting Account
    let (vesting_pda, _) = get_vesting_pda(&company_name);
    let (treasury_pda, _) = get_treasury_pda(&company_name);

    let mut data = get_discriminator("create_vesting_account").to_vec();
    data.extend_from_slice(&(company_name.len() as u32).to_le_bytes());
    data.extend_from_slice(company_name.as_bytes());

    let create_vesting_ix = Instruction {
        program_id: to_sdk_pubkey(&crate::ID),
        accounts: vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new(vesting_pda, false),
            AccountMeta::new_readonly(mint_keypair.pubkey(), false),
            AccountMeta::new(treasury_pda, false),
            AccountMeta::new_readonly(to_sdk_pubkey(&token::ID), false),
            AccountMeta::new_readonly(to_sdk_pubkey(&anchor_lang::system_program::ID), false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[create_vesting_ix],
        Some(&owner.pubkey()),
        &[&owner],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // 3. Fund Treasury
    let total_amount: u64 = 1_000_000;
    let fund_treasury_ix = mint_to_ix(
        &mint_keypair.pubkey(),
        &treasury_pda,
        &owner.pubkey(),
        total_amount,
    );
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[fund_treasury_ix],
        Some(&owner.pubkey()),
        &[&owner],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // 4. Create Employee Vesting
    let (employee_pda, _) = get_employee_pda(&employee.pubkey(), &vesting_pda);
    
    // Use current LiteSVM clock time
    let clock = svm.get_sysvar::<Clock>();
    let start_time = clock.unix_timestamp;
    let end_time = start_time + 100; // 100 seconds vesting
    let cliff_time = start_time + 10; // 10 seconds cliff

    let mut data = get_discriminator("create_employee_vesting").to_vec();
    data.extend_from_slice(&start_time.to_le_bytes());
    data.extend_from_slice(&end_time.to_le_bytes());
    data.extend_from_slice(&total_amount.to_le_bytes());
    data.extend_from_slice(&cliff_time.to_le_bytes());

    let create_employee_ix = Instruction {
        program_id: to_sdk_pubkey(&crate::ID),
        accounts: vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(employee.pubkey(), false),
            AccountMeta::new_readonly(vesting_pda, false),
            AccountMeta::new(employee_pda, false),
            AccountMeta::new_readonly(to_sdk_pubkey(&anchor_lang::system_program::ID), false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[create_employee_ix],
        Some(&owner.pubkey()),
        &[&owner],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // 5. Claim Tokens (Before Cliff - Should Fail)
    let employee_ata = get_ata(&employee.pubkey(), &mint_keypair.pubkey());
    
    let mut data = get_discriminator("claim_tokens").to_vec();
    data.extend_from_slice(&(company_name.len() as u32).to_le_bytes());
    data.extend_from_slice(company_name.as_bytes());

    let claim_ix = Instruction {
        program_id: to_sdk_pubkey(&crate::ID),
        accounts: vec![
            AccountMeta::new(employee.pubkey(), true),
            AccountMeta::new(employee_pda, false),
            AccountMeta::new(vesting_pda, false),
            AccountMeta::new_readonly(mint_keypair.pubkey(), false),
            AccountMeta::new(treasury_pda, false),
            AccountMeta::new(employee_ata, false),
            AccountMeta::new_readonly(to_sdk_pubkey(&token::ID), false),
            AccountMeta::new_readonly(to_sdk_pubkey(&associated_token::ID), false),
            AccountMeta::new_readonly(to_sdk_pubkey(&anchor_lang::system_program::ID), false),
        ],
        data: data.clone(),
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[claim_ix.clone()],
        Some(&employee.pubkey()),
        &[&employee],
        blockhash,
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Should fail before cliff");

    // 6. Warp time to after cliff, halfway through vesting
    let mut new_clock = clock.clone();
    new_clock.unix_timestamp = start_time + 50; // 50% vested
    svm.set_sysvar::<Clock>(&new_clock);

    // Add a dummy transfer to ensure unique transaction signature
    let dummy_ix1 = solana_sdk::system_instruction::transfer(&employee.pubkey(), &Keypair::new().pubkey(), 1);

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[claim_ix.clone(), dummy_ix1],
        Some(&employee.pubkey()),
        &[&employee],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Check balance (should be 50% = 500_000)
    let balance = svm.get_account(&employee_ata).unwrap().data;
    // Basic decode of spl token balance (offset 64 is amount as u64)
    let amount = u64::from_le_bytes(balance[64..72].try_into().unwrap());
    assert_eq!(amount, 500_000);

    // 7. Warp time to end of vesting
    let mut new_clock = clock;
    new_clock.unix_timestamp = end_time + 1; // 100% vested
    svm.set_sysvar::<Clock>(&new_clock);

    // Add another dummy transfer
    let dummy_ix2 = solana_sdk::system_instruction::transfer(&employee.pubkey(), &Keypair::new().pubkey(), 2);

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[claim_ix, dummy_ix2],
        Some(&employee.pubkey()),
        &[&employee],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Check balance (should be 100% = 1_000_000)
    let balance = svm.get_account(&employee_ata).unwrap().data;
    let amount = u64::from_le_bytes(balance[64..72].try_into().unwrap());
    assert_eq!(amount, 1_000_000);
}
