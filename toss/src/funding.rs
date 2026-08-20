use crate::accounting::{checked_add_balance, checked_sub_balance, compute_available};
use crate::error::ContractError;
use crate::types::{BalanceInfo, EscrowState};
use crate::{auth, events, storage};
use soroban_sdk::{token, Env};

pub(crate) fn deposit_funds(env: Env, amount: i128) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&escrow)?;

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let new_total_deposited = checked_add_balance(escrow.total_deposited, amount)?;
    let token_client = token::Client::new(&env, &escrow.token);
    token_client.transfer(&escrow.maintainer, env.current_contract_address(), &amount);

    escrow.total_deposited = new_total_deposited;
    storage::set_escrow(&env, &escrow);
    events::emit_funds_deposited(&env, amount, escrow.total_deposited);

    Ok(())
}

pub(crate) fn withdraw_funds(env: Env, amount: i128) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&escrow)?;

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    }

    let available = compute_available(&escrow)?;
    if amount > available {
        return Err(ContractError::WithdrawExceedsAvailable);
    }

    let new_total_deposited = checked_sub_balance(escrow.total_deposited, amount)?;
    let token_client = token::Client::new(&env, &escrow.token);
    token_client.transfer(&env.current_contract_address(), &escrow.maintainer, &amount);

    escrow.total_deposited = new_total_deposited;
    storage::set_escrow(&env, &escrow);

    let new_available = compute_available(&escrow)?;
    events::emit_funds_withdrawn(&env, amount, new_available);

    Ok(())
}

pub(crate) fn get_escrow(env: Env) -> Result<EscrowState, ContractError> {
    storage::get_escrow(&env)
}

pub(crate) fn get_balance(env: Env) -> Result<BalanceInfo, ContractError> {
    let escrow = storage::get_escrow(&env)?;
    let total_deposited = escrow.total_deposited;
    let reserved = escrow.reserved;
    let total_released = escrow.total_released;
    let available = compute_available(&escrow)?;

    Ok(BalanceInfo {
        total_deposited,
        reserved,
        available,
        total_released,
    })
}
