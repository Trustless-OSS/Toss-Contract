use crate::error::ContractError;
use crate::types::EscrowState;

fn ensure_nonnegative(amount: i128) -> Result<i128, ContractError> {
    if amount < 0 {
        Err(ContractError::BalanceInvariantBroken)
    } else {
        Ok(amount)
    }
}

pub fn compute_available(escrow: &EscrowState) -> Result<i128, ContractError> {
    let after_reserved = escrow
        .total_deposited
        .checked_sub(escrow.reserved)
        .ok_or(ContractError::BalanceInvariantBroken)?;
    let available = after_reserved
        .checked_sub(escrow.total_released)
        .ok_or(ContractError::BalanceInvariantBroken)?;

    ensure_nonnegative(available)
}

pub fn checked_add_balance(current: i128, amount: i128) -> Result<i128, ContractError> {
    let next = current
        .checked_add(amount)
        .ok_or(ContractError::BalanceInvariantBroken)?;
    ensure_nonnegative(next)
}

pub fn checked_sub_balance(current: i128, amount: i128) -> Result<i128, ContractError> {
    let next = current
        .checked_sub(amount)
        .ok_or(ContractError::BalanceInvariantBroken)?;
    ensure_nonnegative(next)
}
