use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock, program_option::COption, sysvar};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod reimburse {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let reimburse = &mut ctx.accounts.reimburse;
        reimburse.num_of_reqs = 0;
        reimburse.token_mint = ctx.accounts.token_mint.key();
        reimburse.treasury_vault = ctx.accounts.token_vault.key();
        reimburse.authority = ctx.accounts.authority.key();

        Ok(())
    }

    pub fn add_member(ctx: Context<AddMember>, member: bool, approver: bool) -> Result<()> {
        let user = &mut ctx.accounts.user_account;
        user.member = member;
        user.approver = approver;

        Ok(())
    }

    pub fn create_request(
        ctx: Context<CreateRequest>,
        description: String,
        url: String,
        amount: u64,
        date: u64,
        category: String
    ) -> Result<()> {
        let user = &mut ctx.accounts.user_account;
        let reimburse = &mut ctx.accounts.reimburse;
        let request = &mut ctx.accounts.reimbursement_request;

        if user.member == false {
            return Err(ErrorCode::NotAllowed.into());
        }

        request.id = 1;
        reimburse.num_of_reqs += 1;
        request.amount = amount;
        request.description = description;
        request.url = url;
        request.date = date;
        request.category = category;
        request.processed = false;
        request.approved = false;
        request.paid = false;
        request.member = ctx.accounts.user.key();

        
        Ok(())
    }

    pub fn process_request(ctx: Context<ProcessRequest>, approved: bool) -> Result<()> {
        let user = &mut ctx.accounts.user_account;
        let request = &mut ctx.accounts.reimbursement_request;

        
        if user.approver == false {
            return Err(ErrorCode::NotAllowed.into());
        }

        // Checks precondition
        if request.processed || request.paid {
            return Err(ErrorCode::AlreadyProcessed.into());
        }

        request.processed = true;

        request.approved = approved;
        
        Ok(())
    }

    pub fn pay_request(ctx: Context<PayRequest>) -> Result<()> {
        let reimburse = &mut ctx.accounts.reimburse;
        let request = &mut ctx.accounts.reimbursement_request;

        // Checks precondition
        if request.paid {
            return Err(ErrorCode::AlreadyPaid.into());
        }

        if request.approved == false {
            return Err(ErrorCode::NotApproved.into());
        }

        {
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.treasury_vault.to_account_info(),
                    to: ctx.accounts.token_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(), //todo use user account as signer
                },
            );
            token::transfer(cpi_ctx, request.amount)?;
        }

        request.paid = true;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub reimburse: Account<'info, Reimburse>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_mint: Box<Account<'info, Mint>>,
    #[account(
        constraint = token_vault.mint == token_mint.key(),
        constraint = token_vault.owner == authority.key(),
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct AddMember<'info> {
    #[account(
        mut,
        has_one = authority
    )]
    pub reimburse: Account<'info, Reimburse>,

    #[account(
        init,
        payer = authority,
        seeds = [
            reimburse.to_account_info().key().as_ref(),
            user.to_account_info().key().as_ref(),
        ],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub user: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CreateRequest<'info> {
    #[account(
        mut,
    )]
    pub reimburse: Account<'info, Reimburse>,

    #[account(
        mut,
        seeds = [
            reimburse.to_account_info().key().as_ref(),
            user.to_account_info().key().as_ref(),
        ],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init,
        payer = user,
        seeds = [
            reimburse.to_account_info().key().as_ref(),
            reimburse.num_of_reqs.to_string().as_ref()
        ],
        bump
    )]
    pub reimbursement_request: Account<'info, ReimbursementRequest>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct ProcessRequest<'info> {
    #[account(
        mut,
    )]
    pub reimburse: Account<'info, Reimburse>,

    #[account(
        mut,
        seeds = [
            reimburse.to_account_info().key().as_ref(),
            user.to_account_info().key().as_ref(),
        ],
        bump,
    )]
    pub user_account: Account<'info, User>,

    pub reimbursement_request: Account<'info, ReimbursementRequest>,

    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct PayRequest<'info> {
    #[account(
        mut,
        has_one = treasury_vault
    )]
    pub reimburse: Account<'info, Reimburse>,

    #[account(
        constraint = token_vault.mint == reimburse.token_mint,
    )]
    pub treasury_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        constraint = token_vault.mint == reimburse.token_mint,
        constraint = token_vault.owner == reimbursement_request.member,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    pub reimbursement_request: Account<'info, ReimbursementRequest>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct User {
    pub member: bool,
    pub approver: bool,
}

#[account]
pub struct Reimburse {
    /// authority holding the treasury wallet 
    pub authority: Pubkey,

    /// number of requests
    pub num_of_reqs: u128,

    /// mint of the token given as reimbursement
    pub token_mint: Pubkey,

    /// treasury vault holding the token 
    pub treasury_vault: Pubkey,
}

#[account]
#[derive(Default)]
pub struct ReimbursementRequest {
    pub id: u128,
    pub amount: u64,
    pub date: u64, // in unix timestamp
    pub category: String,
    pub description: String,
    pub url: String,
    pub processed: bool,
    pub approved: bool,
    pub paid: bool,
    pub member: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not allowed to do this action")]
    NotAllowed,
    #[msg("Reimbursement request has been processed already")]
    AlreadyProcessed,
    #[msg("Reimbursement request has been paid already")]
    AlreadyPaid,
    #[msg("Reimbursement request has not been approved")]
    NotApproved,
}