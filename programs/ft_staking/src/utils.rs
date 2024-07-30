use crate::state::*;
use anchor_lang::prelude::*;

pub fn _get_pending_reward(_global_state: &GlobalState, staked_info: &StakedInfo) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    let staked_period = now.checked_sub(staked_info.staked_timestamp).unwrap() as u64;
    let mut apy: u64 = 20;

    if staked_info.staked_amount > 1_000_000_000_000 {
        apy = 110;
    } else if staked_info.staked_amount > 5_000_000_000_000 {
        apy = 250;
    } else if staked_info.staked_amount > 10_000_000_000_000 {
        apy = 380;
    }

    let pending_reward = staked_period
        .checked_mul(staked_info.staked_amount)
        .unwrap()
        // .checked_mul(global_state.precision as u64)
        // .unwrap()
        .checked_mul(apy)
        .unwrap()
        .checked_div(365 * 24 * 60 * 60 * 100)
        .unwrap()
        .checked_add(staked_info.reward_debt)
        .unwrap();

    Ok(pending_reward)
}
