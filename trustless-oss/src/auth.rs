use crate::types::EscrowState;
use soroban_sdk::{panic_with_error, Env};

pub fn require_platform(_env: &Env, escrow: &EscrowState) {
    escrow.platform.require_auth();
}

pub fn require_maintainer(_env: &Env, escrow: &EscrowState) {
    escrow.maintainer.require_auth();
}

pub fn require_admin(env: &Env) {
    let stored = crate::storage::get_admin(env);
    if let Some(admin) = stored {
        admin.require_auth();
    } else {
        panic_with_error!(env, crate::error::ContractError::NotAdmin);
    }
}

pub fn require_active(env: &Env, escrow: &EscrowState) {
    if !escrow.is_active {
        panic_with_error!(env, crate::error::ContractError::EscrowInactive);
    }
}
