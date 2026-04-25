#[cfg(test)]
mod tests {
    use crate::ID as PROGRAM_ID;
    use litesvm::LiteSVM;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_program,
        transaction::Transaction,
    };

    const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    fn get_vesting_pda(signer: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vesting", signer.as_ref()], &PROGRAM_ID)
    }

    fn create_deposit_ix(signer: &Pubkey, vesting: &Pubkey, amount: u64) -> Instruction {
        // Anchor discriminator for "deposit" = hash("global:deposit")[0..8]
        let discriminator: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
        let mut data = discriminator.to_vec();
        data.extend_from_slice(&amount.to_le_bytes());

        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(*signer, true),
                AccountMeta::new(*vesting, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        }
    }

    fn create_withdraw_ix(signer: &Pubkey, vesting: &Pubkey) -> Instruction {
        // Anchor discriminator for "withdraw" = hash("global:withdraw")[0..8]
        let discriminator: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];

        Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(*signer, true),
                AccountMeta::new(*vesting, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: discriminator.to_vec(),
        }
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let mut svm = LiteSVM::new();

        // Load the program
        let program_bytes = include_bytes!("../../../target/deploy/vesting.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        // Create a user with some SOL
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        // Get vesting PDA
        let (vesting_pda, _bump) = get_vesting_pda(&user.pubkey());

        // Deposit 1 SOL
        let deposit_amount = LAMPORTS_PER_SOL;
        let deposit_ix = create_deposit_ix(&user.pubkey(), &vesting_pda, deposit_amount);

        let blockhash = svm.latest_blockhash();
        let deposit_tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(deposit_tx);
        assert!(result.is_ok(), "Deposit should succeed");

        // Check vesting balance
        let vesting_account = svm.get_account(&vesting_pda).unwrap();
        assert_eq!(vesting_account.lamports, deposit_amount);

        // Withdraw
        let withdraw_ix = create_withdraw_ix(&user.pubkey(), &vesting_pda);

        let blockhash = svm.latest_blockhash();
        let withdraw_tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(withdraw_tx);
        assert!(result.is_ok(), "Withdraw should succeed");

        // Check vesting is empty (account may not exist or have 0 lamports)
        let vesting_account = svm.get_account(&vesting_pda);
        assert!(
            vesting_account.is_none() || vesting_account.unwrap().lamports == 0,
            "Vesting should be empty after withdraw"
        );
    }

    #[test]
    fn test_deposit_fails_if_vesting_has_funds() {
        let mut svm = LiteSVM::new();

        let program_bytes = include_bytes!("../../../target/deploy/vesting.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vesting_pda, _bump) = get_vesting_pda(&user.pubkey());

        // First deposit
        let deposit_ix = create_deposit_ix(&user.pubkey(), &vesting_pda, LAMPORTS_PER_SOL);
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );
        svm.send_transaction(tx).unwrap();

        // Second deposit should fail
        let deposit_ix2 = create_deposit_ix(&user.pubkey(), &vesting_pda, LAMPORTS_PER_SOL);
        let blockhash = svm.latest_blockhash();
        let tx2 = Transaction::new_signed_with_payer(
            &[deposit_ix2],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(tx2);
        assert!(result.is_err(), "Second deposit should fail");
    }

    #[test]
    fn test_withdraw_fails_if_vesting_empty() {
        let mut svm = LiteSVM::new();

        let program_bytes = include_bytes!("../../../target/deploy/vesting.so");
        svm.add_program(PROGRAM_ID, program_bytes);

        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vesting_pda, _bump) = get_vesting_pda(&user.pubkey());

        // Try to withdraw from empty vesting
        let withdraw_ix = create_withdraw_ix(&user.pubkey(), &vesting_pda);
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&user.pubkey()),
            &[&user],
            blockhash,
        );

        let result = svm.send_transaction(tx);
        assert!(result.is_err(), "Withdraw from empty vesting should fail");
    }
}
