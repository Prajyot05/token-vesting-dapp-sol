#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked};

#[cfg(test)]
mod tests;

declare_id!("FnGSzFa8EmF8CwrVTxXSUw3BmKN3LXVgRHy4PyJLbJuS");

#[program]
pub mod vesting {
    use anchor_spl::token_interface;

    use super::*;

    /// Creates a new vesting account to manage a company's token vesting schedules.
    /// This initializes the main VestingAccount and the associated Treasury Token Account.
    pub fn create_vesting_account(
        ctx: Context<CreateVestingAccount>,
        company_name: String,
    ) -> Result<()> {
        // Ensure the company name doesn't exceed the max length configured in the struct
        require!(
            company_name.len() <= 50,
            ErrorCode::CompanyNameTooLong
        );

        *ctx.accounts.vesting_account = VestingAccount {
            owner: ctx.accounts.signer.key(),
            mint: ctx.accounts.mint.key(),
            treasury_token_account: ctx.accounts.treasury_token_account.key(),
            company_name,
            treasury_bump: ctx.bumps.treasury_token_account,
            bump: ctx.bumps.vesting_account,
        };

        Ok(())
    }

    /// Creates a vesting schedule for a specific employee (beneficiary).
    /// This sets up an EmployeeAccount detailing the vesting timeline and token amount.
    pub fn create_employee_vesting(
        ctx: Context<CreateEmployeeAccount>,
        start_time: i64,
        end_time: i64,
        total_amount: u64,
        cliff_time: i64,
    ) -> Result<()> {
        // Validate vesting parameters to ensure logical timelines and valid amounts
        require!(start_time < end_time, ErrorCode::InvalidVestingPeriod);
        require!(cliff_time <= end_time, ErrorCode::InvalidVestingPeriod);
        require!(total_amount > 0, ErrorCode::InvalidAmount);

        *ctx.accounts.employee_account = EmployeeAccount {
            beneficiary: ctx.accounts.beneficiary.key(),
            start_time,
            end_time,
            total_amount,
            total_withdrawn: 0,
            cliff_time,
            vesting_account: ctx.accounts.vesting_account.key(),
            bump: ctx.bumps.employee_account,
        };

        Ok(())
    }

    /// Allows an employee to claim their vested tokens.
    /// Calculates the vested amount based on the current time and transfers tokens from the treasury.
    pub fn claim_tokens(ctx: Context<ClaimTokens>, _company_name: String) -> Result<()> {
        let employee_account = &mut ctx.accounts.employee_account;
        let now = Clock::get()?.unix_timestamp;

        // Check if the current time is before the cliff time
        if now < employee_account.cliff_time {
            return Err(ErrorCode::ClaimNotAvailableYet.into());
        }

        // Check if vesting hasn't even started yet (just in case cliff < start_time)
        if now < employee_account.start_time {
            return Err(ErrorCode::ClaimNotAvailableYet.into());
        }

        // Calculate the total vesting time duration
        let total_vesting_time = employee_account
            .end_time
            .saturating_sub(employee_account.start_time);
            
        // Calculate the elapsed time since start
        let time_since_start = now.saturating_sub(employee_account.start_time);
        
        // Calculate the currently vested amount
        let vested_amount = if now >= employee_account.end_time {
            // Full amount is vested if we are past the end time
            employee_account.total_amount
        } else {
            // We cast to u128 to prevent overflow during multiplication
            let vested = (employee_account.total_amount as u128)
                .checked_mul(time_since_start as u128)
                .ok_or(ErrorCode::CalculationOverflow)?
                .checked_div(total_vesting_time as u128)
                .ok_or(ErrorCode::CalculationOverflow)?;
                
            vested as u64
        };

        // Calculate the actual claimable amount subtracting what has already been withdrawn
        let claimable_amount = vested_amount.saturating_sub(employee_account.total_withdrawn);
        
        // Ensure there is a positive amount to claim
        if claimable_amount == 0 {
            return Err(ErrorCode::NothingToClaim.into());
        }

        // Setup the CPI to transfer tokens from the treasury to the employee
        let transfer_cpi_accounts = TransferChecked {
            from: ctx.accounts.treasury_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.employee_token_account.to_account_info(),
            authority: ctx.accounts.treasury_token_account.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.key();
        
        // The treasury token account is a PDA owned by the program, so we must sign with its seeds
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vesting_treasury",
            ctx.accounts.vesting_account.company_name.as_ref(),
            &[ctx.accounts.vesting_account.treasury_bump],
        ]];
        
        let cpi_context =
            CpiContext::new(cpi_program, transfer_cpi_accounts).with_signer(signer_seeds);
            
        let decimals = ctx.accounts.mint.decimals;
        
        // Perform the transfer
        token_interface::transfer_checked(cpi_context, claimable_amount, decimals)?;
        
        // Update the withdrawn amount in the employee's account
        employee_account.total_withdrawn = employee_account.total_withdrawn.saturating_add(claimable_amount);
        
        Ok(())
    }
}

/// Accounts for `create_vesting_account` instruction.
#[derive(Accounts)]
#[instruction(company_name: String)]
pub struct CreateVestingAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    
    /// The vesting account state PDA
    #[account(
        init,
        space = 8 + VestingAccount::INIT_SPACE,
        payer = signer,
        seeds = [company_name.as_ref()],
        bump
    )]
    pub vesting_account: Account<'info, VestingAccount>,
    
    /// The SPL Token mint for this vesting schedule
    pub mint: InterfaceAccount<'info, Mint>,
    
    /// The treasury token account PDA to hold the locked tokens
    #[account(
        init,
        token::mint = mint,
        token::authority = treasury_token_account,
        payer = signer,
        seeds = [b"vesting_treasury", company_name.as_bytes()],
        bump
    )]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

/// Accounts for `create_employee_vesting` instruction.
#[derive(Accounts)]
pub struct CreateEmployeeAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// The employee who will receive the vested tokens
    /// `SystemAccount` because it doesn't need to be initialized on-chain yet
    pub beneficiary: SystemAccount<'info>,
    
    /// The vesting account (must be owned by the caller `owner`)
    #[account(has_one = owner)]
    pub vesting_account: Account<'info, VestingAccount>,
    
    /// The employee's specific vesting schedule account PDA
    #[account(
        init,
        space = 8 + EmployeeAccount::INIT_SPACE,
        payer = owner,
        seeds = [b"employee_vesting", beneficiary.key().as_ref(), vesting_account.key().as_ref()],
        bump
    )]
    pub employee_account: Account<'info, EmployeeAccount>,
    
    pub system_program: Program<'info, System>,
}

/// Accounts for `claim_tokens` instruction.
#[derive(Accounts)]
#[instruction(company_name: String)]
pub struct ClaimTokens<'info> {
    /// The employee claiming their vested tokens
    #[account(mut)]
    pub beneficiary: Signer<'info>,
    
    /// The employee's specific vesting schedule account PDA
    #[account(
        mut,
        seeds = [b"employee_vesting", beneficiary.key().as_ref(), vesting_account.key().as_ref()],
        bump = employee_account.bump,
        has_one = beneficiary,
        has_one = vesting_account
    )]
    pub employee_account: Account<'info, EmployeeAccount>,
    
    /// The main vesting account for the company
    #[account(
        mut,
        seeds = [company_name.as_ref()],
        bump = vesting_account.bump,
        has_one = treasury_token_account,
        has_one = mint
    )]
    pub vesting_account: Account<'info, VestingAccount>,
    
    /// The SPL Token mint for the tokens being claimed
    pub mint: InterfaceAccount<'info, Mint>,
    
    /// The treasury token account holding the vested tokens
    #[account(mut)]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    
    /// The employee's token account to receive the claimed tokens
    /// Automatically initialized if it doesn't exist
    #[account(
        init_if_needed,
        payer = beneficiary,
        associated_token::mint = mint,
        associated_token::authority = beneficiary,
        associated_token::token_program = token_program
    )]
    pub employee_token_account: InterfaceAccount<'info, TokenAccount>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// Global vesting account storing configuration for a company's vesting program
#[account]
#[derive(InitSpace, Debug)]
pub struct VestingAccount {
    /// Administrator / owner of the vesting program
    pub owner: Pubkey,
    /// Token Mint
    pub mint: Pubkey,
    /// Vault account holding the tokens
    pub treasury_token_account: Pubkey,
    /// Company name string
    #[max_len(50)]
    pub company_name: String,
    pub treasury_bump: u8,
    pub bump: u8,
}

/// Employee-specific vesting schedule state
#[account]
#[derive(InitSpace, Debug)]
pub struct EmployeeAccount {
    /// Address of the employee
    pub beneficiary: Pubkey,
    /// When the vesting schedule begins (unix timestamp)
    pub start_time: i64,
    /// When the vesting schedule ends (unix timestamp)
    pub end_time: i64,
    /// Total number of tokens to be vested over the period
    pub total_amount: u64,
    /// Number of tokens already claimed
    pub total_withdrawn: u64,
    /// When the first portion of tokens becomes claimable
    pub cliff_time: i64,
    /// Reference to the global vesting account
    pub vesting_account: Pubkey,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Claiming is not available yet.")]
    ClaimNotAvailableYet,
    #[msg("There is nothing to claim.")]
    NothingToClaim,
    #[msg("The start time must be before the end time and cliff time must be valid.")]
    InvalidVestingPeriod,
    #[msg("The total amount must be greater than zero.")]
    InvalidAmount,
    #[msg("The company name must not exceed 50 characters.")]
    CompanyNameTooLong,
    #[msg("Math calculation overflowed.")]
    CalculationOverflow,
}
