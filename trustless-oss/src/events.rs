use crate::types::PayoutTarget;
use soroban_sdk::{symbol_short, Address, Env, Symbol};

// Event topic symbols. Plain `Symbol`s replace the former `#[contracttype]`
// `EventKey` enum: topics only need to be `Val`-convertible, not
// XDR-serializable as a contract type, and `Symbol` topics are the
// conventional Soroban pattern. Note the on-chain topic encoding changes
// from an enum value to a symbol — event consumers must match on these.
const ESCROW_INITIALIZED: Symbol = symbol_short!("esc_init");
const FUNDS_DEPOSITED: Symbol = symbol_short!("deposited");
const FUNDS_WITHDRAWN: Symbol = symbol_short!("withdrawn");
const MILESTONE_CREATED: Symbol = symbol_short!("ms_create");
const CONTRIBUTOR_ASSIGNED: Symbol = symbol_short!("assigned");
const CONTRIBUTOR_REASSIGNED: Symbol = symbol_short!("reassign");
const FUNDS_RELEASED: Symbol = symbol_short!("released");
const PARTIAL_RELEASE: Symbol = symbol_short!("part_rel");
const MILESTONE_CANCELLED: Symbol = symbol_short!("ms_cancel");

pub fn emit_escrow_initialized(env: &Env, repo_id: u64, maintainer: Address) {
    let topics = (ESCROW_INITIALIZED, repo_id, maintainer);
    env.events().publish(topics, ());
}

pub fn emit_funds_deposited(env: &Env, amount: i128, new_total: i128) {
    let topics = (FUNDS_DEPOSITED, amount, new_total);
    env.events().publish(topics, ());
}

pub fn emit_funds_withdrawn(env: &Env, amount: i128, new_available: i128) {
    let topics = (FUNDS_WITHDRAWN, amount, new_available);
    env.events().publish(topics, ());
}

pub fn emit_milestone_created(env: &Env, issue_id: u64, reward: i128) {
    let topics = (MILESTONE_CREATED, issue_id, reward);
    env.events().publish(topics, ());
}

pub fn emit_contributor_assigned(env: &Env, issue_id: u64, contributor: PayoutTarget) {
    let topics = (CONTRIBUTOR_ASSIGNED, issue_id, contributor);
    env.events().publish(topics, ());
}

pub fn emit_contributor_reassigned(env: &Env, issue_id: u64, new_contributor: PayoutTarget) {
    let topics = (CONTRIBUTOR_REASSIGNED, issue_id, new_contributor);
    env.events().publish(topics, ());
}

pub fn emit_funds_released(env: &Env, issue_id: u64, contributor: PayoutTarget, amount: i128) {
    let topics = (FUNDS_RELEASED, issue_id, contributor, amount);
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
        PARTIAL_RELEASE,
        issue_id,
        contributor,
        released,
        returned_to_pool,
    );
    env.events().publish(topics, ());
}

pub fn emit_milestone_cancelled(env: &Env, issue_id: u64) {
    let topics = (MILESTONE_CANCELLED, issue_id);
    env.events().publish(topics, ());
}
