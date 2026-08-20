use crate::accounting::{checked_add_balance, checked_sub_balance};
use crate::error::ContractError;
use crate::types::{MilestoneStatus, PayoutTarget};
use crate::{auth, cctp, events, storage};
use soroban_sdk::Env;

pub(crate) fn release_funds(env: Env, issue_id: u64, amount: i128) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_platform(&escrow);
    auth::require_active(&escrow)?;

    let mut milestone = storage::get_milestone(&env, issue_id)?;
    if milestone.status != MilestoneStatus::Active {
        return Err(ContractError::MilestoneNotActive);
    }

    if amount <= 0 {
        return Err(ContractError::ZeroAmount);
    } else if amount > milestone.reward {
        return Err(ContractError::ReleaseTooLarge);
    }

    let contributor = milestone.contributor.clone();
    let actual_release_amount = match contributor {
        PayoutTarget::Stellar(_) => amount,
        PayoutTarget::Cctp(_, _) => amount - cctp::cctp_remainder(amount),
        PayoutTarget::None => return Err(ContractError::ContributorNotSet),
    };

    escrow.reserved = checked_sub_balance(escrow.reserved, milestone.reward)?;
    escrow.total_released = checked_add_balance(escrow.total_released, actual_release_amount)?;
    milestone.status = MilestoneStatus::Released;
    milestone.actual_released = actual_release_amount;
    milestone.released_at = Some(env.ledger().timestamp());

    storage::set_escrow(&env, &escrow);
    storage::set_milestone(&env, issue_id, &milestone);
    cctp::cc_release_fund(&env, &escrow.token, &contributor, actual_release_amount)?;

    let returned_to_pool = milestone
        .reward
        .checked_sub(actual_release_amount)
        .ok_or(ContractError::BalanceInvariantBroken)?;
    events::emit_funds_released(
        &env,
        issue_id,
        contributor,
        actual_release_amount,
        returned_to_pool,
    );

    Ok(())
}
