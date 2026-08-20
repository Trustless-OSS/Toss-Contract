use crate::accounting::{checked_add_balance, checked_sub_balance, compute_available};
use crate::error::ContractError;
use crate::types::{Milestone, MilestoneStatus, PayoutTarget};
use crate::{auth, cctp, events, storage, MAX_PAGE_SIZE};
use soroban_sdk::{panic_with_error, Env, Vec};

pub(crate) fn create_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&env, &escrow);

    if reward <= 0 {
        panic_with_error!(&env, ContractError::ZeroAmount);
    }
    if storage::get_milestone(&env, issue_id).is_ok() {
        return Err(ContractError::DuplicateIssueId);
    }

    let available = compute_available(&escrow)?;
    if reward > available {
        return Err(ContractError::InsufficientBalance);
    }

    let milestone = Milestone {
        issue_id,
        reward,
        contributor: PayoutTarget::None,
        status: MilestoneStatus::Pending,
        created_at: env.ledger().timestamp(),
        released_at: None,
        actual_released: 0,
    };

    escrow.reserved = checked_add_balance(escrow.reserved, reward)?;
    storage::set_escrow(&env, &escrow);
    storage::set_milestone(&env, issue_id, &milestone);
    storage::push_issue_id(&env, issue_id);
    events::emit_milestone_created(&env, issue_id, reward);

    Ok(())
}

pub(crate) fn update_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&env, &escrow);

    if reward <= 0 {
        panic_with_error!(&env, ContractError::ZeroAmount);
    }

    let mut milestone = storage::get_milestone(&env, issue_id)?;
    if milestone.status != MilestoneStatus::Pending {
        return Err(ContractError::MilestoneNotPending);
    }

    let old_reward = milestone.reward;
    let delta = reward
        .checked_sub(old_reward)
        .ok_or(ContractError::BalanceInvariantBroken)?;

    if delta > 0 {
        let available = compute_available(&escrow)?;
        if delta > available {
            return Err(ContractError::InsufficientBalance);
        }
    }

    escrow.reserved = checked_add_balance(escrow.reserved, delta)?;
    milestone.reward = reward;
    storage::set_escrow(&env, &escrow);
    storage::set_milestone(&env, issue_id, &milestone);
    events::emit_milestone_updated(&env, issue_id, old_reward, reward);

    Ok(())
}

pub(crate) fn assign_contributor(
    env: Env,
    issue_id: u64,
    contributor: PayoutTarget,
) -> Result<(), ContractError> {
    cctp::validate_cctp_target(&contributor)?;

    let escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&env, &escrow);

    let mut milestone = storage::get_milestone(&env, issue_id)?;
    if milestone.status != MilestoneStatus::Pending {
        return Err(ContractError::MilestoneNotPending);
    }

    milestone.contributor = contributor.clone();
    milestone.status = MilestoneStatus::Active;
    storage::set_milestone(&env, issue_id, &milestone);
    events::emit_contributor_assigned(&env, issue_id, contributor);

    Ok(())
}

pub(crate) fn reassign_contributor(
    env: Env,
    issue_id: u64,
    new_contributor: PayoutTarget,
) -> Result<(), ContractError> {
    cctp::validate_cctp_target(&new_contributor)?;

    let escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&env, &escrow);

    let mut milestone = storage::get_milestone(&env, issue_id)?;
    if milestone.status != MilestoneStatus::Active {
        return Err(ContractError::MilestoneNotActive);
    }

    milestone.contributor = new_contributor.clone();
    storage::set_milestone(&env, issue_id, &milestone);
    events::emit_contributor_reassigned(&env, issue_id, new_contributor);

    Ok(())
}

pub(crate) fn cancel_milestone(env: Env, issue_id: u64) -> Result<(), ContractError> {
    let mut escrow = storage::get_escrow(&env)?;
    auth::require_maintainer(&escrow);
    auth::require_active(&env, &escrow);

    let mut milestone = storage::get_milestone(&env, issue_id)?;
    if milestone.status != MilestoneStatus::Pending && milestone.status != MilestoneStatus::Active {
        return Err(ContractError::MilestoneNotActive);
    }

    escrow.reserved = checked_sub_balance(escrow.reserved, milestone.reward)?;
    milestone.status = MilestoneStatus::Cancelled;
    storage::set_escrow(&env, &escrow);
    storage::set_milestone(&env, issue_id, &milestone);
    events::emit_milestone_cancelled(&env, issue_id);

    Ok(())
}

pub(crate) fn get_milestone(env: Env, issue_id: u64) -> Result<Milestone, ContractError> {
    storage::get_milestone(&env, issue_id)
}

pub(crate) fn list_milestones(
    env: Env,
    offset: u32,
    limit: u32,
) -> Result<Vec<Milestone>, ContractError> {
    if limit == 0 {
        return Err(ContractError::ZeroPageLimit);
    }

    let issue_ids = storage::get_issue_ids(&env);
    let count = issue_ids.len();
    if offset >= count {
        return Ok(Vec::new(&env));
    }

    let page_limit = limit.min(MAX_PAGE_SIZE);
    let end = offset.saturating_add(page_limit).min(count);
    let mut milestones: Vec<Milestone> = Vec::new(&env);
    for i in offset..end {
        let id = issue_ids.get(i).unwrap();
        milestones.push_back(storage::get_milestone(&env, id)?);
    }

    Ok(milestones)
}

pub(crate) fn get_milestone_count(env: Env) -> u32 {
    storage::get_issue_ids(&env).len()
}
