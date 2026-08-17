use crate::types::{
    AdminTransferred, ContributorAssigned, ContributorReassigned, EscrowInitialized,
    EscrowPaused, EscrowResumed, FundsDeposited, FundsReleased, FundsWithdrawn,
    MaintainerUpdated, MilestoneCancelled, MilestoneCreated, MilestoneUpdated, PartialRelease,
    PayoutTarget, PlatformUpdated,
};
use soroban_sdk::{Address, Env};

pub fn emit_escrow_initialized(env: &Env, repo_id: u64, maintainer: Address) {
    EscrowInitialized {
        repo_id,
        maintainer,
    }
    .publish(env);
}

pub fn emit_funds_deposited(env: &Env, amount: i128, new_total: i128) {
    FundsDeposited { amount, new_total }.publish(env);
}

pub fn emit_funds_withdrawn(env: &Env, amount: i128, new_available: i128) {
    FundsWithdrawn {
        amount,
        new_available,
    }
    .publish(env);
}

pub fn emit_milestone_created(env: &Env, issue_id: u64, reward: i128) {
    MilestoneCreated { issue_id, reward }.publish(env);
}

pub fn emit_milestone_updated(env: &Env, issue_id: u64, old_reward: i128, new_reward: i128) {
    MilestoneUpdated {
        issue_id,
        old_reward,
        new_reward,
    }
    .publish(env);
}

pub fn emit_contributor_assigned(env: &Env, issue_id: u64, contributor: PayoutTarget) {
    ContributorAssigned {
        issue_id,
        contributor,
    }
    .publish(env);
}

pub fn emit_contributor_reassigned(env: &Env, issue_id: u64, new_contributor: PayoutTarget) {
    ContributorReassigned {
        issue_id,
        new_contributor,
    }
    .publish(env);
}

pub fn emit_funds_released(env: &Env, issue_id: u64, contributor: PayoutTarget, amount: i128) {
    FundsReleased {
        issue_id,
        contributor,
        amount,
    }
    .publish(env);
}

pub fn emit_partial_release(
    env: &Env,
    issue_id: u64,
    contributor: PayoutTarget,
    released: i128,
    returned_to_pool: i128,
) {
    PartialRelease {
        issue_id,
        contributor,
        released,
        returned_to_pool,
    }
    .publish(env);
}

pub fn emit_milestone_cancelled(env: &Env, issue_id: u64) {
    MilestoneCancelled { issue_id }.publish(env);
}

pub fn emit_admin_transferred(env: &Env, old_admin: Address, new_admin: Address) {
    AdminTransferred {
        old_admin,
        new_admin,
    }
    .publish(env);
}

pub fn emit_platform_updated(env: &Env, old_platform: Address, new_platform: Address) {
    PlatformUpdated {
        old_platform,
        new_platform,
    }
    .publish(env);
}

pub fn emit_maintainer_updated(env: &Env, old_maintainer: Address, new_maintainer: Address) {
    MaintainerUpdated {
        old_maintainer,
        new_maintainer,
    }
    .publish(env);
}

pub fn emit_escrow_paused(env: &Env) {
    EscrowPaused.publish(env);
}

pub fn emit_escrow_resumed(env: &Env) {
    EscrowResumed.publish(env);
}
