use crate::error::ContractError;
use crate::types::{EscrowState, Milestone};
use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
pub enum StorageKey {
    Escrow,         // EscrowState
    Milestone(u64), // issue_id → Milestone
    EscrowIssueIds, // Vec<u64>
    Admin,          // Address — can call initialize_escrow
}

fn ttl_params(env: &Env) -> (u32, u32) {
    let max = env.storage().max_ttl();
    (max / 2, max)
}

fn bump_instance(env: &Env) {
    let (threshold, extend_to) = ttl_params(env);
    env.storage().instance().extend_ttl(threshold, extend_to);
}

fn bump_persistent(env: &Env, key: &StorageKey) {
    let (threshold, extend_to) = ttl_params(env);
    env.storage()
        .persistent()
        .extend_ttl(key, threshold, extend_to);
}

pub fn get_escrow(env: &Env) -> Result<EscrowState, ContractError> {
    bump_instance(env);
    let key = StorageKey::Escrow;
    let escrow = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::EscrowNotFound)?;
    bump_persistent(env, &key);
    Ok(escrow)
}

pub fn set_escrow(env: &Env, escrow: &EscrowState) {
    bump_instance(env);
    let key = StorageKey::Escrow;
    env.storage().persistent().set(&key, escrow);
    bump_persistent(env, &key);
}

pub fn has_escrow(env: &Env) -> bool {
    bump_instance(env);
    let key = StorageKey::Escrow;
    let present = env.storage().persistent().has(&key);
    if present {
        bump_persistent(env, &key);
    }
    present
}

pub fn get_milestone(env: &Env, issue_id: u64) -> Result<Milestone, ContractError> {
    bump_instance(env);
    let key = StorageKey::Milestone(issue_id);
    let milestone = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::MilestoneNotFound)?;
    bump_persistent(env, &key);
    Ok(milestone)
}

pub fn set_milestone(env: &Env, issue_id: u64, milestone: &Milestone) {
    bump_instance(env);
    let key = StorageKey::Milestone(issue_id);
    env.storage().persistent().set(&key, milestone);
    bump_persistent(env, &key);
}

pub fn get_issue_ids(env: &Env) -> Vec<u64> {
    bump_instance(env);
    let key = StorageKey::EscrowIssueIds;
    if !env.storage().persistent().has(&key) {
        return Vec::new(env);
    }
    bump_persistent(env, &key);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn push_issue_id(env: &Env, issue_id: u64) {
    bump_instance(env);
    let key = StorageKey::EscrowIssueIds;
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    ids.push_back(issue_id);
    env.storage().persistent().set(&key, &ids);
    bump_persistent(env, &key);
}

pub fn set_issue_ids(env: &Env, ids: &Vec<u64>) {
    bump_instance(env);
    let key = StorageKey::EscrowIssueIds;
    env.storage().persistent().set(&key, ids);
    bump_persistent(env, &key);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    bump_instance(env);
    let key = StorageKey::Admin;
    let admin = env.storage().persistent().get(&key);
    if admin.is_some() {
        bump_persistent(env, &key);
    }
    admin
}

pub fn set_admin(env: &Env, admin: &Address) {
    bump_instance(env);
    let key = StorageKey::Admin;
    env.storage().persistent().set(&key, admin);
    bump_persistent(env, &key);
}
