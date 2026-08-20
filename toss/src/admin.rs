use crate::error::ContractError;
use crate::{auth, events, storage, types::EscrowState};
use soroban_sdk::{Address, Bytes, Env, Vec};

pub(crate) fn initialize(
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

pub(crate) fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
    let admin = storage::get_admin(&env).ok_or(ContractError::NotAdmin)?;
    auth::require_admin(&admin);
    let escrow = storage::get_escrow(&env)?;
    auth::require_active(&env, &escrow);

    storage::set_admin(&env, &new_admin);
    events::emit_admin_transferred(&env, admin, new_admin);

    Ok(())
}

pub(crate) fn update_platform(env: Env, new_platform: Address) -> Result<(), ContractError> {
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

pub(crate) fn update_maintainer(env: Env, new_maintainer: Address) -> Result<(), ContractError> {
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

pub(crate) fn pause_escrow(env: Env, reason: Option<Bytes>) -> Result<(), ContractError> {
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

pub(crate) fn resume_escrow(env: Env) -> Result<(), ContractError> {
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
