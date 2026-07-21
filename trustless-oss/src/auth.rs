use crate::types::EscrowState;
use soroban_sdk::{panic_with_error, Env};

pub fn require_platform(escrow: &EscrowState) {
    escrow.platform.require_auth();
}

pub fn require_maintainer(escrow: &EscrowState) {
    escrow.maintainer.require_auth();
}

pub fn require_active(env: &Env, escrow: &EscrowState) {
    if !escrow.is_active {
        panic_with_error!(env, crate::error::ContractError::EscrowInactive);
    }
}
