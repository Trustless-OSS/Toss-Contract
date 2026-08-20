#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec};

pub mod accounting;
mod admin;
pub mod auth;
pub mod cctp;
pub mod error;
pub mod events;
mod funding;
mod milestones;
mod payouts;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use error::ContractError;
use types::{BalanceInfo, EscrowState, Milestone, PayoutTarget};

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
        admin::initialize(env, repo_id, maintainer, platform, token)
    }

    /// Deposits USDC into the contract to fund upcoming milestones.
    pub fn deposit_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        funding::deposit_funds(env, amount)
    }

    /// Withdraws unreserved USDC funds back to the maintainer.
    pub fn withdraw_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        funding::withdraw_funds(env, amount)
    }

    /// Creates a new pending milestone, reserving the specified reward amount.
    pub fn create_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
        milestones::create_milestone(env, issue_id, reward)
    }

    /// Updates the reward of a pending milestone before any work begins.
    ///
    /// Mirrors `create_milestone`'s authorization: maintainer-only and requires an
    /// active escrow. Only milestones still in `Pending` status may be edited - once
    /// a contributor is assigned (`Active`) or the milestone is `Released`/`Cancelled`
    /// the reward is immutable. The `issue_id` itself never changes; this is a reward
    /// edit only (titles are not stored on-chain).
    ///
    /// When the reward changes, `escrow.reserved` is adjusted by the delta. A reward
    /// increase is checked against the currently available balance and rejected with
    /// `InsufficientBalance` if it doesn't fit; a decrease frees the difference back
    /// into the available pool.
    pub fn update_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
        milestones::update_milestone(env, issue_id, reward)
    }

    /// Assigns a contributor to a pending milestone and moves it to active status.
    pub fn assign_contributor(
        env: Env,
        issue_id: u64,
        contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        milestones::assign_contributor(env, issue_id, contributor)
    }

    /// Reassigns an active milestone to a new contributor.
    pub fn reassign_contributor(
        env: Env,
        issue_id: u64,
        new_contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        milestones::reassign_contributor(env, issue_id, new_contributor)
    }

    /// Releases a payout to the assigned contributor upon completion.
    ///
    /// `amount == reward` is a full release; `amount < reward` pays part of the
    /// reward and returns the remainder to the available pool; `amount > reward`
    /// is rejected with `ReleaseTooLarge`. The released amount reported in the
    /// event is what actually left the contract (CCTP truncates to 6 decimals),
    /// with the unspent difference credited back to the pool.
    pub fn release_funds(env: Env, issue_id: u64, amount: i128) -> Result<(), ContractError> {
        payouts::release_funds(env, issue_id, amount)
    }

    /// Cancels a milestone and un-reserves the funds, returning them to the available pool.
    pub fn cancel_milestone(env: Env, issue_id: u64) -> Result<(), ContractError> {
        milestones::cancel_milestone(env, issue_id)
    }

    /// Transfers the stored admin to a new address.
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        admin::transfer_admin(env, new_admin)
    }

    /// Updates the platform address on the escrow.
    pub fn update_platform(env: Env, new_platform: Address) -> Result<(), ContractError> {
        admin::update_platform(env, new_platform)
    }

    /// Updates the maintainer address on the escrow.
    pub fn update_maintainer(env: Env, new_maintainer: Address) -> Result<(), ContractError> {
        admin::update_maintainer(env, new_maintainer)
    }

    /// Pauses the escrow, blocking all state-changing operations until resumed.
    ///
    /// Only the stored admin may call this method. The escrow must be active.
    /// Query methods remain callable while paused. No balances or reserved amounts
    /// are touched; only `is_active` is set to false.
    pub fn pause_escrow(env: Env, reason: Option<Bytes>) -> Result<(), ContractError> {
        admin::pause_escrow(env, reason)
    }

    /// Resumes the escrow, re-enabling all state-changing operations.
    ///
    /// Only the stored admin may call this method. The escrow must be paused (inactive).
    pub fn resume_escrow(env: Env) -> Result<(), ContractError> {
        admin::resume_escrow(env)
    }

    /// Retrieves the global state for this repository's escrow.
    pub fn get_escrow(env: Env) -> Result<EscrowState, ContractError> {
        funding::get_escrow(env)
    }

    /// Retrieves the details and current status of a specific milestone by its issue ID.
    pub fn get_milestone(env: Env, issue_id: u64) -> Result<Milestone, ContractError> {
        milestones::get_milestone(env, issue_id)
    }

    /// Returns the overall balance information including deposited, reserved, and available amounts.
    pub fn get_balance(env: Env) -> Result<BalanceInfo, ContractError> {
        funding::get_balance(env)
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
        milestones::list_milestones(env, offset, limit)
    }

    /// Returns the total number of milestones created for this repository.
    pub fn get_milestone_count(env: Env) -> u32 {
        milestones::get_milestone_count(env)
    }
}
