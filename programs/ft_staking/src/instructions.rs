use crate::{constants::*, errors::*, events::*, state::*, utils::*};
use anchor_lang::prelude::Account;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use solana_program::{program::invoke, program::invoke_signed, system_instruction};
// use mpl_token_metadata::types::DataV2;
use std::mem::size_of;

#[access_control(ctx.accounts.validate())]
pub fn initialize(ctx: Context<Initialize>, new_owner: Pubkey) -> Result<()> {
    let accts = ctx.accounts;
    accts.global_state.is_initialized = 1;
    accts.global_state.is_paused = false;
    accts.global_state.owner = new_owner;
    accts.global_state.vault = accts.vault.key();
    accts.global_state.ant_coin = Pubkey::try_from(ANT_COIN).unwrap();
    // accts.global_state.precision = PRECISION;
    accts.global_state.ant_food_token = Pubkey::try_from(ANT_FOOD_TOKEN_ID).unwrap();
    // accts.global_state.stake_fee_amount = STAKE_FEE_AMOUNT;
    // accts.global_state.max_amount_for_stake = MAX_AMOUNT_FOR_STAKE;
    // accts.global_state.cycle_staked_amount = CYCLE_STAKED_AMOUNT;
    // accts.global_state.cycle_timestamp = CYCLE_TIMESTAMP;
    // accts.global_state.antc_price = antc_price;
    // accts.global_state.antc_expo = antc_expo;

    accts.minter.minter_key = new_owner;
    accts.minter.is_minter = true;

    let rent = Rent::default();
    let required_lamports = rent
        .minimum_balance(0)
        .max(1)
        .saturating_sub(accts.vault.to_account_info().lamports());
    msg!("required lamports = {:?}", required_lamports);
    invoke(
        &system_instruction::transfer(&accts.owner.key(), &accts.vault.key(), required_lamports),
        &[
            accts.owner.to_account_info().clone(),
            accts.vault.clone(),
            accts.system_program.to_account_info().clone(),
        ],
    )?;

    Ok(())
}

pub fn stake(ctx: Context<Stake>, antc_amount: u64) -> Result<()> {
    let accts = ctx.accounts;

    let mut pending_reward = 0;
    let now = Clock::get()?.unix_timestamp;
    if accts.staked_info.staked_amount > 0 {
        pending_reward = _get_pending_reward(&accts.global_state, &accts.staked_info).unwrap();
    }
    accts.staked_info.reward_debt += pending_reward;
    accts.staked_info.staked_amount += antc_amount;
    accts.staked_info.staked_timestamp = now;

    // transfer antc coin
    let cpi_ctx = CpiContext::new(
        accts.token_program.to_account_info(),
        Transfer {
            from: accts.user_ant_coin_account.to_account_info(),
            to: accts.ant_coin_vault_account.to_account_info(),
            authority: accts.user.to_account_info(),
        },
    );
    transfer(cpi_ctx, antc_amount)?;

    emit!(FoodGatheringStaked {
        staker: accts.user.key(),
        antc_stake_amount: antc_amount
    });

    Ok(())
}

pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
    let accts = ctx.accounts;

    require!(
        accts.staked_info.staked_amount > 0,
        FoodGatheringError::NotStaked
    );

    let pending_reward = _get_pending_reward(&accts.global_state, &accts.staked_info).unwrap();

    let binding = accts.global_state.owner;
    let (_, bump) =
        Pubkey::find_program_address(&[GLOBAL_STATE_SEED, binding.as_ref()], ctx.program_id);
    let vault_seeds = &[GLOBAL_STATE_SEED, binding.as_ref(), &[bump]];
    let signer = &[&vault_seeds[..]];

    // transfer ant food token
    if pending_reward > 0 {
        let cpi_ctx = CpiContext::new(
            accts.token_program.to_account_info(),
            Transfer {
                from: accts.ant_food_token_vault_account.to_account_info().clone(),
                to: accts.user_ant_food_token_account.to_account_info().clone(),
                authority: accts.global_state.to_account_info().clone(),
            },
        );
        transfer(
            cpi_ctx.with_signer(signer),
            pending_reward, // .checked_div(accts.global_state.precision as u64)
                            // .unwrap(),
        )?;
    }
    // transfer antc coin
    let cpi_ctx = CpiContext::new(
        accts.token_program.to_account_info(),
        Transfer {
            from: accts.ant_coin_vault_account.to_account_info(),
            to: accts.user_ant_coin_account.to_account_info(),
            authority: accts.global_state.to_account_info(),
        },
    );
    transfer(cpi_ctx.with_signer(signer), accts.staked_info.staked_amount)?;
    accts.staked_info.staked_amount = 0;
    accts.staked_info.reward_debt = 0;

    emit!(FoodGatheringUnStaked {
        staker: accts.user.key(),
        antc_stake_amount: accts.staked_info.staked_amount,
        reward_ant_food_amount: pending_reward // .checked_div(accts.global_state.precision as u64)
                                               // .unwrap()
    });

    Ok(())
}

pub fn claim(ctx: Context<Claim>) -> Result<()> {
    let accts = ctx.accounts;

    require!(
        accts.staked_info.staked_amount > 0,
        FoodGatheringError::NotStaked
    );

    let pending_reward = _get_pending_reward(&accts.global_state, &accts.staked_info).unwrap();

    let binding = accts.global_state.owner;
    let (_, bump) =
        Pubkey::find_program_address(&[GLOBAL_STATE_SEED, binding.as_ref()], ctx.program_id);
    let vault_seeds = &[GLOBAL_STATE_SEED, binding.as_ref(), &[bump]];
    let signer = &[&vault_seeds[..]];

    // transfer ant food token
    if pending_reward > 0 {
        let cpi_ctx = CpiContext::new(
            accts.token_program.to_account_info(),
            Transfer {
                from: accts.ant_food_token_vault_account.to_account_info().clone(),
                to: accts.user_ant_food_token_account.to_account_info().clone(),
                authority: accts.global_state.to_account_info().clone(),
            },
        );
        transfer(
            cpi_ctx.with_signer(signer),
            pending_reward, // .checked_div(accts.global_state.precision as u64)
                            // .unwrap(),
        )?;
    }

    accts.staked_info.reward_debt = 0;

    emit!(FoodGatheringUnStaked {
        staker: accts.user.key(),
        antc_stake_amount: accts.staked_info.staked_amount,
        reward_ant_food_amount: pending_reward // .checked_div(accts.global_state.precision as u64)
                                               // .unwrap()
    });

    Ok(())
}
// minter functions
pub fn deposit_ant_food_token(ctx: Context<DepositAntFoodToken>, amount: u64) -> Result<()> {
    let accts = ctx.accounts;

    let cpi_ctx = CpiContext::new(
        accts.token_program.to_account_info(),
        Transfer {
            from: accts.minter_ant_food_token_account.to_account_info(),
            to: accts.ant_food_token_vault_account.to_account_info(),
            authority: accts.minter_key.to_account_info(),
        },
    );

    transfer(cpi_ctx, amount)?;

    Ok(())
}

pub fn set_ant_food_token(ctx: Context<SetAntFoodToken>) -> Result<()> {
    let accts = ctx.accounts;
    accts.global_state.ant_food_token = accts.new_ant_food_token.key();

    Ok(())
}

pub fn set_ant_coin(ctx: Context<SetAntCoin>) -> Result<()> {
    let accts = ctx.accounts;
    accts.global_state.ant_coin = accts.ant_coin.key();

    Ok(())
}

// owner functions
pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    let accts = ctx.accounts;
    accts.global_state.is_paused = paused;

    Ok(())
}

pub fn set_minter_role(ctx: Context<SetMinterRole>, is_minter: bool) -> Result<()> {
    let accts = ctx.accounts;
    accts.minter.minter_key = accts.minter_key.key();
    accts.minter.is_minter = is_minter;

    Ok(())
}

pub fn withdraw_sol(ctx: Context<WithdrawSOL>, amount: u64) -> Result<()> {
    let accts = ctx.accounts;

    let (_, bump) = Pubkey::find_program_address(&[VAULT_SEED], &crate::ID);

    invoke_signed(
        &system_instruction::transfer(&accts.vault.key(), &accts.owner.key(), amount),
        &[
            accts.vault.to_account_info().clone(),
            accts.owner.to_account_info().clone(),
            accts.system_program.to_account_info().clone(),
        ],
        &[&[VAULT_SEED, &[bump]]],
    )?;

    Ok(())
}

pub fn withdraw_token(ctx: Context<WithdrawToken>, amount: u64) -> Result<()> {
    let accts = ctx.accounts;

    let binding = accts.owner.key();
    let (_, bump) =
        Pubkey::find_program_address(&[GLOBAL_STATE_SEED, binding.as_ref()], ctx.program_id);
    let vault_seeds = &[GLOBAL_STATE_SEED, binding.as_ref(), &[bump]];
    let signer = &[&vault_seeds[..]];

    let cpi_ctx = CpiContext::new(
        accts.token_program.to_account_info(),
        Transfer {
            from: accts.token_vault_account.to_account_info().clone(),
            to: accts.token_owner_account.to_account_info().clone(),
            authority: accts.global_state.to_account_info().clone(),
        },
    );
    transfer(cpi_ctx.with_signer(signer), amount)?;

    Ok(())
}

pub fn get_pending_reward(ctx: Context<GetPendingReward>) -> Result<u64> {
    let accts = ctx.accounts;
    _get_pending_reward(&accts.global_state, &accts.staked_info)
}

#[derive(Accounts)]
#[instruction(new_owner: Pubkey)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        seeds = [GLOBAL_STATE_SEED, owner.key().as_ref()],
        bump,
        space = 8 + size_of::<GlobalState>(),
        payer = owner,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    /// CHECK: this should be set by admin
    pub vault: AccountInfo<'info>, // to receive SOL

    #[account(
        init,
        seeds = [MINTER_STATE_SEED, new_owner.key().as_ref()],
        bump,
        space = 8 + size_of::<Minter>(),
        payer = owner,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    pub fn validate(&self) -> Result<()> {
        if self.global_state.is_initialized == 1 {
            require!(
                self.global_state.owner.eq(&self.owner.key()),
                FoodGatheringError::NotAllowedOwner
            )
        }
        Ok(())
    }
}

// TODO Don't forget that the data account size can't be adjusted, so make sure you allocate it as much as you need.
// #[account]
#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
        constraint = global_state.is_paused == false,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [STAKED_INFO_SEED, user.key().as_ref()],
        bump,
        space = 8 + size_of::<StakedInfo>(),
    )]
    pub staked_info: Account<'info, StakedInfo>,

    #[account(
        mut,
        address = global_state.ant_coin
    )]
    pub ant_coin: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [TOKEN_VAULT_SEED, ant_coin.key().as_ref()],
        bump,
        token::mint = ant_coin,
        token::authority = global_state,
    )]
    ant_coin_vault_account: Box<Account<'info, TokenAccount>>,

    // user account for ant coin
    #[account(mut)]
    pub user_ant_coin_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
        constraint = global_state.is_paused == false,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [STAKED_INFO_SEED, user.key().as_ref()],
        bump,
    )]
    pub staked_info: Account<'info, StakedInfo>,

    #[account(
        mut,
        address = global_state.ant_coin
    )]
    pub ant_coin: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, ant_coin.key().as_ref()],
        bump,
        token::mint = ant_coin,
        token::authority = global_state,
    )]
    pub ant_coin_vault_account: Box<Account<'info, TokenAccount>>,

    // user account for ant coin
    #[account(mut)]
    user_ant_coin_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        address = global_state.ant_food_token
    )]
    pub ant_food_token: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, ant_food_token.key().as_ref()],
        bump,
        token::mint = ant_food_token,
        token::authority = global_state,
    )]
    pub ant_food_token_vault_account: Box<Account<'info, TokenAccount>>,

    // user account for ant food coin
    #[account(mut)]
    pub user_ant_food_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
        constraint = global_state.is_paused == false,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [STAKED_INFO_SEED, user.key().as_ref()],
        bump,
    )]
    pub staked_info: Account<'info, StakedInfo>,

    #[account(
        mut,
        address = global_state.ant_food_token
    )]
    pub ant_food_token: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, ant_food_token.key().as_ref()],
        bump,
        token::mint = ant_food_token,
        token::authority = global_state,
    )]
    pub ant_food_token_vault_account: Box<Account<'info, TokenAccount>>,

    // user account for ant food coin
    #[account(mut)]
    pub user_ant_food_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositAntFoodToken<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    #[account(
        mut,
        address = global_state.ant_food_token
    )]
    pub ant_food_token: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = minter_key,
        seeds = [TOKEN_VAULT_SEED, ant_food_token.key().as_ref()],
        bump,
        token::mint = ant_food_token,
        token::authority = global_state,
    )]
    pub ant_food_token_vault_account: Box<Account<'info, TokenAccount>>,

    // minter account for ant food token
    #[account(mut)]
    pub minter_ant_food_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetStakeFeeAmount<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetMaxAmountForStake<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetCycleStakedAmount<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetCycleTimestamp<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetAntFoodToken<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub new_ant_food_token: Box<Account<'info, Mint>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetAntCoin<'info> {
    #[account(mut)]
    pub minter_key: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        has_one = minter_key,
        constraint = minter.is_minter == true,
    )]
    pub minter: Account<'info, Minter>,

    pub ant_coin: Box<Account<'info, Mint>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        has_one = owner,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetMinterRole<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        has_one = owner,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    /// CHECK: this should be checked with address in global_state
    pub minter_key: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [MINTER_STATE_SEED, minter_key.key().as_ref()],
        bump,
        space = 8 + size_of::<Minter>(),
        payer = owner,
    )]
    pub minter: Account<'info, Minter>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
        has_one = owner,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        address = global_state.vault
    )]
    /// CHECK: this should be checked with address in global_state
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct WithdrawToken<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED, owner.key().as_ref()],
        bump,
        has_one = owner,
        constraint = global_state.is_initialized == 1,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        address = global_state.vault
    )]
    /// CHECK: this should be checked with address in global_state
    pub vault: AccountInfo<'info>,

    #[account(mut)]
    pub token_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = global_state,
    )]
    token_vault_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = token_mint,
        associated_token::authority = owner
    )]
    token_owner_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct GetPendingReward<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [GLOBAL_STATE_SEED, global_state.owner.as_ref()],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    /// CHECK: this should be checked with address in global_state
    pub staker: AccountInfo<'info>,

    #[account(
        seeds = [STAKED_INFO_SEED, staker.key().as_ref()],
        bump,
    )]
    pub staked_info: Account<'info, StakedInfo>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
