use crate::types::PayoutTarget;
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    EscrowInitialized,
    FundsDeposited,
    FundsWithdrawn,
    MilestoneCreated,
    ContributorAssigned,
    ContributorReassigned,
    FundsReleased,
    PartialRelease,
    MilestoneCancelled,
}

pub fn emit_escrow_initialized(env: &Env, repo_id: u64, maintainer: Address) {
    let topics = (DataKey::EscrowInitialized, repo_id, maintainer);
    env.events().publish(topics, ());
}

pub fn emit_funds_deposited(env: &Env, amount: i128, new_total: i128) {
    let topics = (DataKey::FundsDeposited, amount, new_total);
    env.events().publish(topics, ());
}

pub fn emit_funds_withdrawn(env: &Env, amount: i128, new_available: i128) {
    let topics = (DataKey::FundsWithdrawn, amount, new_available);
    env.events().publish(topics, ());
}

pub fn emit_milestone_created(env: &Env, issue_id: u64, reward: i128) {
    let topics = (DataKey::MilestoneCreated, issue_id, reward);
    env.events().publish(topics, ());
}

pub fn emit_contributor_assigned(env: &Env, issue_id: u64, contributor: PayoutTarget) {
    let topics = (DataKey::ContributorAssigned, issue_id, contributor);
    env.events().publish(topics, ());
}

pub fn emit_contributor_reassigned(env: &Env, issue_id: u64, new_contributor: PayoutTarget) {
    let topics = (DataKey::ContributorReassigned, issue_id, new_contributor);
    env.events().publish(topics, ());
}

pub fn emit_funds_released(env: &Env, issue_id: u64, contributor: PayoutTarget, amount: i128) {
    let topics = (DataKey::FundsReleased, issue_id, contributor, amount);
    env.events().publish(topics, ());
}

pub fn emit_partial_release(
    env: &Env,
    issue_id: u64,
    contributor: PayoutTarget,
    released: i128,
    returned_to_pool: i128,
) {
    let topics = (
        DataKey::PartialRelease,
        issue_id,
        contributor,
        released,
        returned_to_pool,
    );
    env.events().publish(topics, ());
}

pub fn emit_milestone_cancelled(env: &Env, issue_id: u64) {
    let topics = (DataKey::MilestoneCancelled, issue_id);
    env.events().publish(topics, ());
}
