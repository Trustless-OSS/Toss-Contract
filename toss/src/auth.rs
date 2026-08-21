use crate::error::ContractError;
use crate::types::EscrowState;
use soroban_sdk::Address;

pub fn require_platform(escrow: &EscrowState) {
    escrow.platform.require_auth();
}

pub fn require_maintainer(escrow: &EscrowState) {
    escrow.maintainer.require_auth();
}

pub fn require_admin(admin: &Address) {
    admin.require_auth();
}

pub fn require_active(escrow: &EscrowState) -> Result<(), ContractError> {
    if !escrow.is_active {
        return Err(ContractError::EscrowInactive);
    }

    Ok(())
}
