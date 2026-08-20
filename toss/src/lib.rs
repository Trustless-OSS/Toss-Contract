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
    pub fn initialize(
        env: Env,
        repo_id: u64,
        maintainer: Address,
        platform: Address,
        token: Address,
    ) -> Result<(), ContractError> {
        admin::initialize(env, repo_id, maintainer, platform, token)
    }

    pub fn deposit_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        funding::deposit_funds(env, amount)
    }

    pub fn withdraw_funds(env: Env, amount: i128) -> Result<(), ContractError> {
        funding::withdraw_funds(env, amount)
    }

    pub fn create_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
        milestones::create_milestone(env, issue_id, reward)
    }

    pub fn update_milestone(env: Env, issue_id: u64, reward: i128) -> Result<(), ContractError> {
        milestones::update_milestone(env, issue_id, reward)
    }

    pub fn assign_contributor(
        env: Env,
        issue_id: u64,
        contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        milestones::assign_contributor(env, issue_id, contributor)
    }

    pub fn reassign_contributor(
        env: Env,
        issue_id: u64,
        new_contributor: PayoutTarget,
    ) -> Result<(), ContractError> {
        milestones::reassign_contributor(env, issue_id, new_contributor)
    }

    pub fn release_funds(env: Env, issue_id: u64, amount: i128) -> Result<(), ContractError> {
        payouts::release_funds(env, issue_id, amount)
    }

    pub fn cancel_milestone(env: Env, issue_id: u64) -> Result<(), ContractError> {
        milestones::cancel_milestone(env, issue_id)
    }

    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        admin::transfer_admin(env, new_admin)
    }

    pub fn update_platform(env: Env, new_platform: Address) -> Result<(), ContractError> {
        admin::update_platform(env, new_platform)
    }

    pub fn update_maintainer(env: Env, new_maintainer: Address) -> Result<(), ContractError> {
        admin::update_maintainer(env, new_maintainer)
    }

    pub fn pause_escrow(env: Env, reason: Option<Bytes>) -> Result<(), ContractError> {
        admin::pause_escrow(env, reason)
    }

    pub fn resume_escrow(env: Env) -> Result<(), ContractError> {
        admin::resume_escrow(env)
    }

    pub fn get_escrow(env: Env) -> Result<EscrowState, ContractError> {
        funding::get_escrow(env)
    }

    pub fn get_milestone(env: Env, issue_id: u64) -> Result<Milestone, ContractError> {
        milestones::get_milestone(env, issue_id)
    }

    pub fn get_balance(env: Env) -> Result<BalanceInfo, ContractError> {
        funding::get_balance(env)
    }

    pub fn list_milestones(
        env: Env,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Milestone>, ContractError> {
        milestones::list_milestones(env, offset, limit)
    }

    pub fn get_milestone_count(env: Env) -> u32 {
        milestones::get_milestone_count(env)
    }
}
