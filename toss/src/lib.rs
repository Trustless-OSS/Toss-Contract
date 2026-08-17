#![no_std]

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Bytes, Env, Vec};

pub mod auth;
pub mod cctp;
pub mod error;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use cctp::cc_release_fund;
use error::ContractError;
use types::{BalanceInfo, EscrowState, Milestone, MilestoneStatus, PayoutTarget};

/// Hard cap on the number of milestones returned by a single `list_milestones` page.
pub const MAX_PAGE_SIZE: u32 = 50;

#[contract]
pub struct TOSSContract;

#[contractimpl]
impl TOSSContract {
    /// Initializes the single-repo escrow state with the maintainer, platform, and token configurations.
    pub fn initialize(
        env: Env,
        repo_id: u64,
        maintainer: Address,
        platform: Address,
        token: Address,
    ) -> Result<(), ContractError> {
        if storage::has_escrow(&env) {
            return Err(ContractError::EscrowAlreadyExists);
        }

        let stored_admin = storage::get_admin(&env);
        if let Some(admin) = stored_admin {
            admin.require_auth();
        } else {
            maintainer.require_auth();
            storage::set_admin(&env, &maintainer);
        }

        let escrow = EscrowState {
            repo_id,
            maintainer: maintainer.clone(),
            platform,
            token,
            total_deposited: 0,
            reserved: 0,
            total_released: 0,
            created_at: env.ledger().timestamp(),
            is_active: true,
        };

        storage::set_escrow(&env, &escrow);
        storage::set_issue_ids(&env, &Vec::new(&env));

        events::emit_escrow_initialized(&env, repo_id, maintainer);

        Ok(())
    }

    /// Deposits USDC into the contract to fund upcoming milestones.
    pub fn deposit_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_maintainer(&escrow);
        auth::require_active(&env, &escrow);

        if amount <= 0 {
            panic_with_error!(&env, ContractError::ZeroAmount);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&escrow.maintainer, env.current_contract_address(), &amount);

        escrow.total_deposited += amount;
        storage::set_escrow(&env, &escrow);
        events::emit_funds_deposited(&env, amount, escrow.total_deposited);

        Ok(())
    }

    /// Withdraws unreserved USDC funds back to the maintainer.
    pub fn withdraw_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_maintainer(&escrow);
        auth::require_active(&env, &escrow);

        if amount <= 0 {
            panic_with_error!(&env, ContractError::ZeroAmount);
        }

        let available = escrow
            .total_deposited
            .checked_sub(escrow.reserved)
            .unwrap_or(0)
            .checked_sub(escrow.total_released)
            .unwrap_or(0);

        if amount > available {
            panic_with_error!(&env, ContractError::WithdrawExceedsAvailable);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.maintainer, &amount);

        escrow.total_deposited -= amount;
        storage::set_escrow(&env, &escrow);

        let new_available = escrow
            .total_deposited
            .checked_sub(escrow.reserved)
            .unwrap_or(0)
            .checked_sub(escrow.total_released)
            .unwrap_or(0);
        events::emit_funds_withdrawn(&env, amount, new_available);

        Ok(())
    }

    /// Creates a new pending milestone, reserving the specified reward amount.
    pub fn create_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_maintainer(&escrow);
        auth::require_active(&env, &escrow);

        if reward <= 0 {
            panic_with_error!(&env, ContractError::ZeroAmount);
        }

        if storage::get_milestone(&env, issue_id).is_ok() {
            return Err(ContractError::DuplicateIssueId);
        }

        let balance = Self::get_balance(env.clone())?;
        if reward > balance.available {
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

        escrow.reserved += reward;
        storage::set_escrow(&env, &escrow);
        storage::set_milestone(&env, issue_id, &milestone);
        storage::push_issue_id(&env, issue_id);

        events::emit_milestone_created(&env, issue_id, reward);

        Ok(())
    }

    /// Updates the reward of a pending milestone before any work begins.
    ///
    /// Mirrors `create_milestone`'s authorization: maintainer-only and requires an
    /// active escrow. Only milestones still in `Pending` status may be edited — once
    /// a contributor is assigned (`Active`) or the milestone is `Released`/`Cancelled`
    /// the reward is immutable. The `issue_id` itself never changes; this is a reward
    /// edit only (titles are not stored on-chain).
    ///
    /// When the reward changes, `escrow.reserved` is adjusted by the delta. A reward
    /// increase is checked against the currently available balance and rejected with
    /// `InsufficientBalance` if it doesn't fit; a decrease frees the difference back
    /// into the available pool.
    pub fn update_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
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
        let delta = reward - old_reward;

        // Only an increase can outgrow the pool; `available` already nets out the
        // milestone's current reservation, so the extra `delta` must fit on top of it.
        if delta > 0 {
            let balance = Self::get_balance(env.clone())?;
            if delta > balance.available {
                return Err(ContractError::InsufficientBalance);
            }
        }

        escrow.reserved += delta;
        milestone.reward = reward;

        storage::set_escrow(&env, &escrow);
        storage::set_milestone(&env, issue_id, &milestone);

        events::emit_milestone_updated(&env, issue_id, old_reward, reward);

        Ok(())
    }

    /// Assigns a contributor to a pending milestone and moves it to active status.
    pub fn assign_contributor(
        env: Env,
        issue_id: u64,
        contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        if let PayoutTarget::Cctp(domain, ref recipient) = contributor {
            if !cctp::is_supported_domain(domain) {
                return Err(ContractError::InvalidDomain);
            }
            if recipient.iter().all(|b| b == 0) {
                return Err(ContractError::EmptyRecipient);
            }
            if !cctp::has_valid_padding(domain, recipient) {
                return Err(ContractError::InvalidCctpRecipientPadding);
            }
        }

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

    /// Reassigns an active milestone to a new contributor.
    pub fn reassign_contributor(
        env: Env,
        issue_id: u64,
        new_contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        if let PayoutTarget::Cctp(domain, ref recipient) = new_contributor {
            if !cctp::is_supported_domain(domain) {
                return Err(ContractError::InvalidDomain);
            }
            if recipient.iter().all(|b| b == 0) {
                return Err(ContractError::EmptyRecipient);
            }
            if !cctp::has_valid_padding(domain, recipient) {
                return Err(ContractError::InvalidCctpRecipientPadding);
            }
        }

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

    /// Releases the fully reserved reward amount to the assigned contributor upon completion.
    pub fn release_funds(env: Env, issue_id: u64) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_platform(&escrow);
        auth::require_active(&env, &escrow);

        let mut milestone = storage::get_milestone(&env, issue_id)?;

        if milestone.status != MilestoneStatus::Active {
            panic_with_error!(&env, ContractError::MilestoneNotActive);
        }

        let reward = milestone.reward;
        let contributor = milestone.contributor.clone();

        let release_amount = match contributor {
            PayoutTarget::Stellar(_) => reward,
            PayoutTarget::Cctp(_, _) => cctp::truncate_to_6_decimals(reward),
            PayoutTarget::None => return Err(ContractError::ContributorNotSet),
        };

        escrow.reserved -= reward;
        escrow.total_released += release_amount;

        milestone.status = MilestoneStatus::Released;
        milestone.actual_released = release_amount;
        milestone.released_at = Some(env.ledger().timestamp());

        cc_release_fund(&env, &escrow.token, &contributor, release_amount)?;

        storage::set_escrow(&env, &escrow);
        storage::set_milestone(&env, issue_id, &milestone);

        events::emit_funds_released(&env, issue_id, contributor, release_amount);

        Ok(())
    }

    /// Releases a partial reward amount to the contributor and returns the remainder to the available pool.
    pub fn partial_release(
        env: Env,
        issue_id: u64,
        release_amount: i128,
    ) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_platform(&escrow);
        auth::require_active(&env, &escrow);

        let mut milestone = storage::get_milestone(&env, issue_id)?;

        if milestone.status != MilestoneStatus::Active {
            panic_with_error!(&env, ContractError::MilestoneNotActive);
        }

        if release_amount > milestone.reward {
            panic_with_error!(&env, ContractError::ReleaseTooLarge);
        }

        let contributor = milestone.contributor.clone();

        let actual_release_amount = match contributor {
            PayoutTarget::Stellar(_) => release_amount,
            PayoutTarget::Cctp(_, _) => cctp::truncate_to_6_decimals(release_amount),
            PayoutTarget::None => return Err(ContractError::ContributorNotSet),
        };

        escrow.reserved -= milestone.reward;
        escrow.total_released += actual_release_amount;

        milestone.status = MilestoneStatus::Released;
        milestone.actual_released = actual_release_amount;
        milestone.released_at = Some(env.ledger().timestamp());

        cc_release_fund(&env, &escrow.token, &contributor, actual_release_amount)?;

        storage::set_escrow(&env, &escrow);
        storage::set_milestone(&env, issue_id, &milestone);

        let returned_to_pool = milestone.reward - actual_release_amount;
        events::emit_partial_release(
            &env,
            issue_id,
            contributor,
            actual_release_amount,
            returned_to_pool,
        );

        Ok(())
    }

    /// Cancels a milestone and un-reserves the funds, returning them to the available pool.
    pub fn cancel_milestone(env: Env, issue_id: u64) -> Result<(), ContractError> {
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_maintainer(&escrow);
        auth::require_active(&env, &escrow);

        let mut milestone = storage::get_milestone(&env, issue_id)?;

        if milestone.status != MilestoneStatus::Pending
            && milestone.status != MilestoneStatus::Active
        {
            return Err(ContractError::MilestoneNotActive);
        }

        escrow.reserved -= milestone.reward;
        milestone.status = MilestoneStatus::Cancelled;
        storage::set_escrow(&env, &escrow);
        storage::set_milestone(&env, issue_id, &milestone);

        events::emit_milestone_cancelled(&env, issue_id);

        Ok(())
    }

    /// Transfers the stored admin to a new address.
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
        auth::require_admin(&admin);
        let escrow = storage::get_escrow(&env)?;
        auth::require_active(&env, &escrow);

        storage::set_admin(&env, &new_admin);
        events::emit_admin_transferred(&env, admin, new_admin);

        Ok(())
    }

    /// Updates the platform address on the escrow.
    pub fn update_platform(env: Env, new_platform: Address) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
        auth::require_admin(&admin);
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_active(&env, &escrow);

        let old_platform = escrow.platform.clone();
        escrow.platform = new_platform.clone();
        storage::set_escrow(&env, &escrow);
        events::emit_platform_updated(&env, old_platform, new_platform);

        Ok(())
    }

    /// Updates the maintainer address on the escrow.
    pub fn update_maintainer(env: Env, new_maintainer: Address) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
        auth::require_admin(&admin);
        let mut escrow = storage::get_escrow(&env)?;
        auth::require_active(&env, &escrow);

        let old_maintainer = escrow.maintainer.clone();
        escrow.maintainer = new_maintainer.clone();
        storage::set_escrow(&env, &escrow);
        events::emit_maintainer_updated(&env, old_maintainer, new_maintainer);

        Ok(())
    }

    /// Pauses the escrow, blocking all state-changing operations until resumed.
    ///
    /// Only the stored admin may call this method. The escrow must be active.
    /// Query methods (get_escrow, get_milestone, get_balance, list_milestones) remain
    /// callable while paused. No balances or reserved amounts are touched — only
    /// is_active is set to false.
    pub fn pause_escrow(env: Env, reason: Option<Bytes>) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
        auth::require_admin(&admin);
        let mut escrow = storage::get_escrow(&env)?;

        if !escrow.is_active {
            return Err(ContractError::EscrowInactive);
        }

        escrow.is_active = false;
        storage::set_escrow(&env, &escrow);
        events::emit_escrow_paused(&env, escrow.repo_id, admin.clone(), reason);

        Ok(())
    }

    /// Resumes the escrow, re-enabling all state-changing operations.
    ///
    /// Only the stored admin may call this method. The escrow must be paused (inactive).
    pub fn resume_escrow(env: Env) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
        auth::require_admin(&admin);
        let mut escrow = storage::get_escrow(&env)?;

        if escrow.is_active {
            return Err(ContractError::EscrowAlreadyActive);
        }

        escrow.is_active = true;
        storage::set_escrow(&env, &escrow);
        events::emit_escrow_resumed(&env, escrow.repo_id, admin.clone());

        Ok(())
    }

    /// Retrieves the global state for this repository's escrow.
    pub fn get_escrow(env: Env) -> Result<EscrowState, ContractError> {
        storage::get_escrow(&env)
    }

    /// Retrieves the details and current status of a specific milestone by its issue ID.
    pub fn get_milestone(env: Env, issue_id: u64) -> Result<Milestone, ContractError> {
        storage::get_milestone(&env, issue_id)
    }

    /// Returns the overall balance information including deposited, reserved, and available amounts.
    pub fn get_balance(env: Env) -> Result<BalanceInfo, ContractError> {
        let escrow = storage::get_escrow(&env)?;
        let total_deposited = escrow.total_deposited;
        let reserved = escrow.reserved;
        let total_released = escrow.total_released;
        let available = total_deposited
            .checked_sub(reserved)
            .unwrap_or(0)
            .checked_sub(total_released)
            .unwrap_or(0);
        Ok(BalanceInfo {
            total_deposited,
            reserved,
            available,
            total_released,
        })
    }

    /// Returns a paginated slice of the milestones created for this repository.
    ///
    /// `offset` is the index of the first milestone to return and `limit` is the
    /// requested page size. A zero `limit` is rejected with `ZeroPageLimit`;
    /// larger limits are clamped to `MAX_PAGE_SIZE`. Offsets at or past the end
    /// of the list return an empty vector. `EscrowIssueIds` remains the index
    /// driving the listing.
    pub fn list_milestones(
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

    /// Returns the total number of milestones created for this repository.
    pub fn get_milestone_count(env: Env) -> u32 {
        storage::get_issue_ids(&env).len()
    }
}
