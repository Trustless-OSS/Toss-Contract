#![cfg(test)]

use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::Events as _;

use super::*;
use crate::error::ContractError;
use crate::types::MilestoneStatus;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{token, Address, Env, String, Vec};

fn setup_env() -> (Env, soroban_sdk::Address) {
    let env = Env::default();
    env.ledger().set(LedgerInfo {
        timestamp: 12345,
        protocol_version: 23,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10000,
        min_persistent_entry_ttl: 10000,
        max_entry_ttl: 200000,
    });
    let contract_id = env.register_contract(None, TrustlessOssContract);
    (env, contract_id)
}

fn client(env: &Env, contract_id: &soroban_sdk::Address) -> TrustlessOssContractClient<'static> {
    TrustlessOssContractClient::new(env, contract_id)
}

fn addresses(env: &Env) -> (Address, Address, Address) {
    let maintainer = Address::generate(env);
    let platform = Address::generate(env);
    let token = Address::generate(env);
    (maintainer, platform, token)
}

// ---------------------------------------------------------------------------
// initialize – success path
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_success() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let escrow = c.get_escrow();
    assert_eq!(escrow.repo_id, 1);
    assert_eq!(escrow.maintainer, maintainer);
    assert_eq!(escrow.platform, platform);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.total_deposited, 0);
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 0);
    assert_eq!(escrow.created_at, 12345);
    assert!(escrow.is_active);
}

#[test]
fn test_initialize_sets_admin() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    env.as_contract(&contract_id, || {
        let stored_admin = storage::get_admin(&env);
        assert_eq!(stored_admin, Some(maintainer));
    });
}

#[test]
fn test_initialize_balance_after_init() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let balance = c.get_balance();
    assert_eq!(balance.total_deposited, 0);
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 0);
    assert_eq!(balance.total_released, 0);
}

#[test]
fn test_initialize_emits_event() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let events = env.events().all();
    assert_eq!(events.len(), 1);
}

// ---------------------------------------------------------------------------
// initialize – error paths
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_rejects_double_init() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let result = c.try_initialize(&2, &maintainer, &platform, &token);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// storage – EscrowState
// ---------------------------------------------------------------------------

#[test]
fn test_storage_escrow_roundtrip() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let escrow = c.get_escrow();
    assert_eq!(escrow.repo_id, 1);
    assert_eq!(escrow.maintainer, maintainer);
    assert_eq!(escrow.platform, platform);
}

#[test]
fn test_get_escrow_before_init_panics() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);

    let result = c.try_get_escrow();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// storage – Milestone
// ---------------------------------------------------------------------------

#[test]
fn test_storage_milestone_roundtrip() {
    let (env, contract_id) = setup_env();

    let milestone = Milestone {
        issue_id: 100,
        title: String::from_str(&env, "Fix critical bug"),
        reward: 50_000_000,
        contributor: PayoutTarget {
            stellar_address: None,
        },
        status: MilestoneStatus::Pending,
        created_at: 1000,
        released_at: None,
        actual_released: 0,
    };

    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 100, &milestone);
    });

    env.as_contract(&contract_id, || {
        let loaded = storage::get_milestone(&env, 100).unwrap();
        assert_eq!(loaded.issue_id, 100);
        assert_eq!(loaded.title, String::from_str(&env, "Fix critical bug"));
        assert_eq!(loaded.reward, 50_000_000);
        assert_eq!(loaded.contributor.stellar_address, None);
        assert_eq!(loaded.status, MilestoneStatus::Pending);
        assert_eq!(loaded.created_at, 1000);
        assert_eq!(loaded.released_at, None);
        assert_eq!(loaded.actual_released, 0);
    });
}

// ---------------------------------------------------------------------------
// storage – Admin
// ---------------------------------------------------------------------------

#[test]
fn test_storage_admin_roundtrip() {
    let (env, contract_id) = setup_env();

    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        storage::set_admin(&env, &admin);
    });

    env.as_contract(&contract_id, || {
        let loaded = storage::get_admin(&env);
        assert_eq!(loaded, Some(admin));
    });
}

#[test]
fn test_storage_admin_returns_none_when_not_set() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let loaded = storage::get_admin(&env);
        assert_eq!(loaded, None);
    });
}

// ---------------------------------------------------------------------------
// storage – Issue IDs
// ---------------------------------------------------------------------------

#[test]
fn test_storage_issue_ids_empty_initially() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let ids = storage::get_issue_ids(&env);
        assert_eq!(ids.len(), 0);
    });
}

#[test]
fn test_storage_issue_ids_push_and_retrieve() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        storage::push_issue_id(&env, 10);
        storage::push_issue_id(&env, 20);
        storage::push_issue_id(&env, 30);

        let ids = storage::get_issue_ids(&env);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), 10);
        assert_eq!(ids.get(1).unwrap(), 20);
        assert_eq!(ids.get(2).unwrap(), 30);
    });
}

#[test]
fn test_storage_set_issue_ids() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let mut ids: Vec<u64> = Vec::new(&env);
        ids.push_back(1);
        ids.push_back(2);
        storage::set_issue_ids(&env, &ids);
    });

    env.as_contract(&contract_id, || {
        let loaded = storage::get_issue_ids(&env);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get(0).unwrap(), 1);
        assert_eq!(loaded.get(1).unwrap(), 2);
    });
}

// ---------------------------------------------------------------------------
// storage – TTL extension
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_extended_on_escrow_write() {
    let (env, contract_id) = setup_env();

    let escrow = EscrowState {
        repo_id: 1,
        maintainer: Address::generate(&env),
        platform: Address::generate(&env),
        token: Address::generate(&env),
        total_deposited: 0,
        reserved: 0,
        total_released: 0,
        created_at: 100,
        is_active: true,
    };

    env.as_contract(&contract_id, || {
        storage::set_escrow(&env, &escrow);
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&storage::StorageKey::Escrow);
        assert!(ttl >= 100_000);
    });
}

#[test]
fn test_ttl_extended_on_milestone_write() {
    let (env, contract_id) = setup_env();

    let milestone = Milestone {
        issue_id: 1,
        title: String::from_str(&env, "Test"),
        reward: 100_000_000,
        contributor: PayoutTarget {
            stellar_address: None,
        },
        status: MilestoneStatus::Pending,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };

    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 1, &milestone);
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&storage::StorageKey::Milestone(1));
        assert!(ttl >= 100_000);
    });
}

#[test]
fn test_ttl_extended_on_admin_write() {
    let (env, contract_id) = setup_env();

    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        storage::set_admin(&env, &admin);
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&storage::StorageKey::Admin);
        assert!(ttl >= 100_000);
    });
}

#[test]
fn test_ttl_extended_on_issue_ids_write() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        storage::push_issue_id(&env, 42);
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&storage::StorageKey::EscrowIssueIds);
        assert!(ttl >= 100_000);
    });
}

// ---------------------------------------------------------------------------
// get_balance
// ---------------------------------------------------------------------------

#[test]
fn test_get_balance_after_initialize() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let balance = c.get_balance();
    assert_eq!(balance.total_deposited, 0);
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 0);
    assert_eq!(balance.total_released, 0);
}

// ---------------------------------------------------------------------------
// list_milestones
// ---------------------------------------------------------------------------

#[test]
fn test_list_milestones_empty_after_init() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let milestones = c.list_milestones();
    assert_eq!(milestones.len(), 0);
}

// ---------------------------------------------------------------------------
// has_escrow
// ---------------------------------------------------------------------------

#[test]
fn test_has_escrow_before_and_after_init() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        assert!(!storage::has_escrow(&env));
    });

    let c = client(&env, &contract_id);
    env.mock_all_auths();
    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    env.as_contract(&contract_id, || {
        assert!(storage::has_escrow(&env));
    });
}

// ---------------------------------------------------------------------------
// release_funds edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_release_funds_not_active_panics() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 1,
        title: String::from_str(&env, "Test"),
        reward: 100,
        contributor: PayoutTarget {
            stellar_address: Some(Address::generate(&env)),
        },
        status: MilestoneStatus::Pending,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };
    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 1, &milestone);
    });

    let result = c.try_release_funds(&1);
    assert!(result.is_err());
}

#[test]
fn test_release_funds_contributor_not_set() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 2,
        title: String::from_str(&env, "Test 2"),
        reward: 100,
        contributor: PayoutTarget {
            stellar_address: None,
        },
        status: MilestoneStatus::Active,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };
    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 2, &milestone);
    });

    let result = c.try_release_funds(&2);
    assert!(result.is_err());
}

#[test]
fn test_partial_release_too_large() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 3,
        title: String::from_str(&env, "Test 3"),
        reward: 100,
        contributor: PayoutTarget {
            stellar_address: Some(Address::generate(&env)),
        },
        status: MilestoneStatus::Active,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };
    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 3, &milestone);
    });

    let result = c.try_partial_release(&3, &150);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Funding mechanics – helpers
// ---------------------------------------------------------------------------

struct FundingSetup {
    env: Env,
    contract_id: Address,
    client: TrustlessOssContractClient<'static>,
    maintainer: Address,
    token: Address,
}

fn setup_funding_env(initial_mint: i128) -> FundingSetup {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let platform = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();

    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap()
        .unwrap();

    if initial_mint > 0 {
        let sac = token::StellarAssetClient::new(&env, &token);
        sac.mint(&maintainer, &initial_mint);
    }

    FundingSetup {
        env,
        contract_id,
        client: c,
        maintainer,
        token,
    }
}

// ---------------------------------------------------------------------------
// deposit_funds
// ---------------------------------------------------------------------------

#[test]
fn test_deposit_funds_success() {
    let setup = setup_funding_env(500);
    let token_client = token::Client::new(&setup.env, &setup.token);

    setup.client.try_deposit_funds(&200).unwrap().unwrap();

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.total_deposited, 200);

    let balance = setup.client.get_balance();
    assert_eq!(balance.total_deposited, 200);
    assert_eq!(balance.available, 200);

    assert_eq!(token_client.balance(&setup.contract_id), 200);
    assert_eq!(token_client.balance(&setup.maintainer), 300);
}

#[test]
fn test_deposit_emits_event() {
    let setup = setup_funding_env(100);
    let events_before = setup.env.events().all().len();

    setup.client.try_deposit_funds(&50).unwrap().unwrap();

    assert!(setup.env.events().all().len() > events_before);
}

#[test]
fn test_deposit_zero_amount_panics() {
    let setup = setup_funding_env(100);
    let result = setup.client.try_deposit_funds(&0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_deposit_negative_amount_panics() {
    let setup = setup_funding_env(100);
    let result = setup.client.try_deposit_funds(&-1);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_deposit_requires_maintainer() {
    let setup = setup_funding_env(100);
    setup.env.set_auths(&[]);
    setup.client.deposit_funds(&50);
}

// ---------------------------------------------------------------------------
// withdraw_funds
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_funds_success() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let token_client = token::Client::new(&setup.env, &setup.token);

    setup.client.try_withdraw_funds(&400).unwrap().unwrap();

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.total_deposited, 600);

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 600);

    assert_eq!(token_client.balance(&setup.contract_id), 600);
    assert_eq!(token_client.balance(&setup.maintainer), 400);
}

#[test]
fn test_withdraw_up_to_available() {
    let setup = setup_funding_env(500);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();

    setup.client.try_withdraw_funds(&500).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 0);
    assert_eq!(balance.total_deposited, 0);
}

#[test]
fn test_withdraw_exceeds_available_panics() {
    let setup = setup_funding_env(500);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();

    let result = setup.client.try_withdraw_funds(&501);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::WithdrawExceedsAvailable
    );
}

#[test]
fn test_withdraw_zero_amount_panics() {
    let setup = setup_funding_env(500);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();

    let result = setup.client.try_withdraw_funds(&0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_withdraw_respects_reserved() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved = 300;
        storage::set_escrow(&setup.env, &escrow);

        let milestone = Milestone {
            issue_id: 99,
            title: String::from_str(&setup.env, "Reserved milestone"),
            reward: 300,
            contributor: PayoutTarget {
                stellar_address: Some(Address::generate(&setup.env)),
            },
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 99, &milestone);
    });

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 700);

    setup.client.try_withdraw_funds(&700).unwrap().unwrap();

    let result = setup.client.try_withdraw_funds(&1);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::WithdrawExceedsAvailable
    );
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_withdraw_requires_maintainer() {
    let setup = setup_funding_env(500);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();

    setup.env.set_auths(&[]);
    setup.client.withdraw_funds(&100);
}

// ---------------------------------------------------------------------------
// Stellar payouts
// ---------------------------------------------------------------------------

#[test]
fn test_release_funds_transfers_to_stellar_contributor() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
            title: String::from_str(&setup.env, "Stellar payout"),
            reward: 500,
            contributor: PayoutTarget {
                stellar_address: Some(contributor.clone()),
            },
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 1, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    setup.client.try_release_funds(&1).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 500);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 500);

    let token_client = token::Client::new(&setup.env, &setup.token);
    assert_eq!(token_client.balance(&contributor), 500);
    assert_eq!(token_client.balance(&setup.contract_id), 500);
}

#[test]
fn test_partial_release_transfers_to_stellar_contributor() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
            title: String::from_str(&setup.env, "Partial Stellar payout"),
            reward: 500,
            contributor: PayoutTarget {
                stellar_address: Some(contributor.clone()),
            },
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 1, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    setup.client.try_partial_release(&1, &400).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 400);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 400);

    let token_client = token::Client::new(&setup.env, &setup.token);
    assert_eq!(token_client.balance(&contributor), 400);
    assert_eq!(token_client.balance(&setup.contract_id), 600);
}

// ---------------------------------------------------------------------------
// E2E lifecycle tests — full contributor reward flows
// ---------------------------------------------------------------------------
//
// Each test below exercises a complete flow through the public contract API
// without bypassing any entry-point via `as_contract`. The `setup_e2e_env`
// helper extends `setup_funding_env` to expose the platform address, which is
// required for `release_funds` and `partial_release`.

struct E2ESetup {
    env: Env,
    contract_id: Address,
    client: TrustlessOssContractClient<'static>,
    maintainer: Address,
    platform: Address,
    contributor: Address,
    token: Address,
}

/// Builds a fully-initialised environment with a real SAC token, mints
/// `initial_mint` stroops to the maintainer, and exposes both the maintainer
/// and the platform address so E2E tests can drive both roles.
fn setup_e2e_env(initial_mint: i128) -> E2ESetup {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let platform = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();

    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap()
        .unwrap();

    if initial_mint > 0 {
        let sac = token::StellarAssetClient::new(&env, &token);
        sac.mint(&maintainer, &initial_mint);
    }

    E2ESetup {
        env,
        contract_id,
        client: c,
        maintainer,
        platform,
        contributor,
        token,
    }
}

// ---------------------------------------------------------------------------
// E2E 1: Full release flow
// initialize → deposit_funds → create_milestone → assign_contributor →
// release_funds
// Verify: token balance moved to contributor, reserved == 0,
//         total_released correct, milestone status is Released.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_full_release_flow() {
    let setup = setup_e2e_env(1_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);

    // Deposit 500_000 into escrow.
    setup.client.try_deposit_funds(&500_000).unwrap().unwrap();
    assert_eq!(token_client.balance(&setup.contract_id), 500_000);

    // Create a milestone for issue #42 with a 300_000 reward.
    setup
        .client
        .try_create_milestone(
            &42,
            &String::from_str(&setup.env, "Add CI pipeline"),
            &300_000,
        )
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 300_000);
    assert_eq!(balance.available, 200_000);

    // Assign the contributor — milestone moves to Active.
    setup
        .client
        .try_assign_contributor(
            &42,
            &PayoutTarget {
                stellar_address: Some(setup.contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();

    let ms = setup.client.get_milestone(&42);
    assert_eq!(ms.status, MilestoneStatus::Active);

    // Platform releases the full reward.
    setup.client.try_release_funds(&42).unwrap().unwrap();

    // Milestone is Released, full reward transferred.
    let ms = setup.client.get_milestone(&42);
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.actual_released, 300_000);
    assert!(ms.released_at.is_some());

    // Escrow accounting.
    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 300_000);

    // Token balances.
    assert_eq!(token_client.balance(&setup.contributor), 300_000);
    assert_eq!(token_client.balance(&setup.contract_id), 200_000);
}

// ---------------------------------------------------------------------------
// E2E 2: Partial release flow
// initialize → deposit_funds → create_milestone → assign_contributor →
// partial_release
// Verify: contributor receives partial amount, remainder returned to available
//         pool, accounting correct.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_partial_release_flow() {
    let setup = setup_e2e_env(1_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);

    setup.client.try_deposit_funds(&600_000).unwrap().unwrap();

    setup
        .client
        .try_create_milestone(
            &10,
            &String::from_str(&setup.env, "Fix memory leak"),
            &400_000,
        )
        .unwrap()
        .unwrap();

    setup
        .client
        .try_assign_contributor(
            &10,
            &PayoutTarget {
                stellar_address: Some(setup.contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();

    // Partial release: pay out 250_000 of the 400_000 reward.
    setup
        .client
        .try_partial_release(&10, &250_000)
        .unwrap()
        .unwrap();

    // Milestone is Released with the partial amount recorded.
    let ms = setup.client.get_milestone(&10);
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.actual_released, 250_000);

    // Escrow: reserved drops by full reward; only partial goes to total_released.
    // The leftover 150_000 is back in the available pool.
    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 250_000);

    let balance = setup.client.get_balance();
    // total_deposited(600_000) - reserved(0) - total_released(250_000) = 350_000
    assert_eq!(balance.available, 350_000);

    // Token balances.
    assert_eq!(token_client.balance(&setup.contributor), 250_000);
    assert_eq!(token_client.balance(&setup.contract_id), 350_000);
}

// ---------------------------------------------------------------------------
// E2E 3: Reassign then release
// create_milestone → assign_contributor → reassign_contributor → release_funds
// Verify: funds go to the new contributor, not the original one.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_reassign_then_release() {
    let setup = setup_e2e_env(500_000);
    let token_client = token::Client::new(&setup.env, &setup.token);

    setup.client.try_deposit_funds(&500_000).unwrap().unwrap();

    setup
        .client
        .try_create_milestone(
            &20,
            &String::from_str(&setup.env, "Refactor auth module"),
            &200_000,
        )
        .unwrap()
        .unwrap();

    let original_contributor = Address::generate(&setup.env);
    let new_contributor = Address::generate(&setup.env);

    // Assign original contributor.
    setup
        .client
        .try_assign_contributor(
            &20,
            &PayoutTarget {
                stellar_address: Some(original_contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();

    // Reassign to a new contributor.
    setup
        .client
        .try_reassign_contributor(
            &20,
            &PayoutTarget {
                stellar_address: Some(new_contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();

    // Confirm stored contributor is the new one.
    let ms = setup.client.get_milestone(&20);
    assert_eq!(
        ms.contributor.stellar_address,
        Some(new_contributor.clone())
    );
    assert_eq!(ms.status, MilestoneStatus::Active);

    // Release to the new contributor.
    setup.client.try_release_funds(&20).unwrap().unwrap();

    // New contributor received the reward; original contributor received nothing.
    assert_eq!(token_client.balance(&new_contributor), 200_000);
    assert_eq!(token_client.balance(&original_contributor), 0);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 200_000);
}

// ---------------------------------------------------------------------------
// E2E 4: Cancel from Pending
// create_milestone → cancel_milestone
// Verify: reserved decremented, available pool restored, milestone Cancelled.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_cancel_from_pending() {
    let setup = setup_e2e_env(300_000);
    setup.client.try_deposit_funds(&300_000).unwrap().unwrap();

    setup
        .client
        .try_create_milestone(
            &30,
            &String::from_str(&setup.env, "Write docs"),
            &100_000,
        )
        .unwrap()
        .unwrap();

    // Before cancel: available is reduced by reserved.
    let balance_before = setup.client.get_balance();
    assert_eq!(balance_before.reserved, 100_000);
    assert_eq!(balance_before.available, 200_000);

    // Cancel the pending milestone.
    setup.client.try_cancel_milestone(&30).unwrap().unwrap();

    let ms = setup.client.get_milestone(&30);
    assert_eq!(ms.status, MilestoneStatus::Cancelled);

    // After cancel: reserved returns to pool.
    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);

    let balance_after = setup.client.get_balance();
    assert_eq!(balance_after.reserved, 0);
    assert_eq!(balance_after.available, 300_000);
    assert_eq!(balance_after.total_released, 0);
}

// ---------------------------------------------------------------------------
// E2E 5: Cancel from Active
// create_milestone → assign_contributor → cancel_milestone
// Verify: same accounting as Pending cancel, status is Cancelled.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_cancel_from_active() {
    let setup = setup_e2e_env(400_000);
    setup.client.try_deposit_funds(&400_000).unwrap().unwrap();

    setup
        .client
        .try_create_milestone(
            &40,
            &String::from_str(&setup.env, "Add rate limiting"),
            &150_000,
        )
        .unwrap()
        .unwrap();

    setup
        .client
        .try_assign_contributor(
            &40,
            &PayoutTarget {
                stellar_address: Some(setup.contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();

    let ms = setup.client.get_milestone(&40);
    assert_eq!(ms.status, MilestoneStatus::Active);

    // Cancel active milestone.
    setup.client.try_cancel_milestone(&40).unwrap().unwrap();

    let ms = setup.client.get_milestone(&40);
    assert_eq!(ms.status, MilestoneStatus::Cancelled);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 400_000);
    assert_eq!(balance.total_released, 0);
}

// ---------------------------------------------------------------------------
// E2E 6: Multiple milestones — accounting integrity
// Create 3 milestones, release 2, cancel 1.
// Verify: total_released, reserved, and available stay consistent throughout.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_multi_milestone_accounting() {
    let setup = setup_e2e_env(1_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);

    setup.client.try_deposit_funds(&900_000).unwrap().unwrap();

    let contrib_a = Address::generate(&setup.env);
    let contrib_b = Address::generate(&setup.env);

    // Create three milestones (total reserved = 600_000).
    setup
        .client
        .try_create_milestone(
            &100,
            &String::from_str(&setup.env, "Feature A"),
            &200_000,
        )
        .unwrap()
        .unwrap();
    setup
        .client
        .try_create_milestone(
            &101,
            &String::from_str(&setup.env, "Feature B"),
            &250_000,
        )
        .unwrap()
        .unwrap();
    setup
        .client
        .try_create_milestone(
            &102,
            &String::from_str(&setup.env, "Feature C"),
            &150_000,
        )
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 600_000);
    assert_eq!(balance.available, 300_000); // 900_000 - 600_000
    assert_eq!(balance.total_released, 0);

    // Assign contributors to A and B; leave C pending.
    setup
        .client
        .try_assign_contributor(
            &100,
            &PayoutTarget {
                stellar_address: Some(contrib_a.clone()),
            },
        )
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(
            &101,
            &PayoutTarget {
                stellar_address: Some(contrib_b.clone()),
            },
        )
        .unwrap()
        .unwrap();

    // Release milestone A (200_000 to contrib_a).
    setup.client.try_release_funds(&100).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 400_000); // B(250k) + C(150k)
    assert_eq!(balance.total_released, 200_000);
    assert_eq!(balance.available, 300_000); // 900k - 400k - 200k

    // Release milestone B (250_000 to contrib_b).
    setup.client.try_release_funds(&101).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 150_000); // C(150k) still pending
    assert_eq!(balance.total_released, 450_000);
    assert_eq!(balance.available, 300_000); // 900k - 150k - 450k

    // Cancel milestone C — its 150_000 returns to available.
    setup.client.try_cancel_milestone(&102).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.total_released, 450_000);
    assert_eq!(balance.available, 450_000); // 900k - 0 - 450k

    // Token balances reflect the correct per-contributor payouts.
    assert_eq!(token_client.balance(&contrib_a), 200_000);
    assert_eq!(token_client.balance(&contrib_b), 250_000);

    // Cross-check all milestones via list_milestones.
    let milestones = setup.client.list_milestones();
    assert_eq!(milestones.len(), 3);

    // Status assertions via get_milestone.
    assert_eq!(
        setup.client.get_milestone(&100).status,
        MilestoneStatus::Released
    );
    assert_eq!(
        setup.client.get_milestone(&101).status,
        MilestoneStatus::Released
    );
    assert_eq!(
        setup.client.get_milestone(&102).status,
        MilestoneStatus::Cancelled
    );
}

// ---------------------------------------------------------------------------
// E2E 7: Deposit after milestones are created
// Create milestone → deposit more funds → verify available increases but
// reserved is unchanged.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_deposit_after_milestone_created() {
    let setup = setup_e2e_env(500_000);
    let sac = token::StellarAssetClient::new(&setup.env, &setup.token);

    // Deposit first tranche and create a milestone.
    setup.client.try_deposit_funds(&200_000).unwrap().unwrap();

    setup
        .client
        .try_create_milestone(
            &50,
            &String::from_str(&setup.env, "Improve indexing"),
            &150_000,
        )
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 150_000);
    assert_eq!(balance.available, 50_000);

    // Mint extra tokens and make a second deposit.
    sac.mint(&setup.maintainer, &300_000);
    setup.client.try_deposit_funds(&300_000).unwrap().unwrap();

    // Available grew; reserved stayed the same.
    let balance = setup.client.get_balance();
    assert_eq!(balance.total_deposited, 500_000);
    assert_eq!(balance.reserved, 150_000); // unchanged
    assert_eq!(balance.available, 350_000); // 500k - 150k - 0
    assert_eq!(balance.total_released, 0);
}

// ---------------------------------------------------------------------------
// E2E 8: Withdraw up to (but not past) reserved boundary
// Deposit → create milestones → try withdraw more than available (expect
// error) → withdraw exactly available (expect success).
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_withdraw_boundary_respects_reserved() {
    let setup = setup_e2e_env(800_000);

    setup.client.try_deposit_funds(&800_000).unwrap().unwrap();

    // Reserve 500_000 across two milestones.
    setup
        .client
        .try_create_milestone(
            &60,
            &String::from_str(&setup.env, "Milestone X"),
            &300_000,
        )
        .unwrap()
        .unwrap();
    setup
        .client
        .try_create_milestone(
            &61,
            &String::from_str(&setup.env, "Milestone Y"),
            &200_000,
        )
        .unwrap()
        .unwrap();

    // Available = 800_000 - 500_000 = 300_000.
    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 300_000);

    // Attempting to withdraw 300_001 must fail.
    let err = setup
        .client
        .try_withdraw_funds(&300_001)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::WithdrawExceedsAvailable);

    // Withdrawing exactly 300_000 must succeed.
    setup
        .client
        .try_withdraw_funds(&300_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 0);
    assert_eq!(balance.reserved, 500_000);

    // One more withdrawal must fail.
    let err = setup
        .client
        .try_withdraw_funds(&1)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::WithdrawExceedsAvailable);
}

// ---------------------------------------------------------------------------
// E2E 9: Event sequence check
// Run a full lifecycle and assert events are emitted in the correct order
// with correct payloads.
// Expected sequence:
//   1. EscrowInitialized  — emitted by initialize (inside setup_e2e_env)
//   2. FundsDeposited
//   3. MilestoneCreated
//   4. ContributorAssigned
//   5. FundsReleased
//
// The Soroban SDK records events as (contract_id, topics_val, data_val).
// We assert the full event vec using `soroban_sdk::vec!` and `.into_val()` on
// the tuple of topics, which is the pattern documented in the SDK migration
// guide for `Events::publish`.
// ---------------------------------------------------------------------------

#[test]
fn test_debug_events_count() {
    use soroban_sdk::testutils::Events as _;
    let setup = setup_e2e_env(500_000);
    let after_setup = setup.env.events().all().len();
    setup.client.try_deposit_funds(&100_000).unwrap().unwrap();
    let after_deposit = setup.env.events().all().len();
    // should see at least EscrowInitialized + FundsDeposited
    assert!(
        after_deposit >= 2,
        "expected >=2 events after deposit, got setup={after_setup} deposit={after_deposit}"
    );
    setup
        .client
        .try_create_milestone(
            &99,
            &String::from_str(&setup.env, "Debug milestone"),
            &50_000,
        )
        .unwrap()
        .unwrap();
    let after_create = setup.env.events().all().len();
    assert!(
        after_create >= 3,
        "expected >=3 events after create_milestone, got {after_create}"
    );
    setup
        .client
        .try_assign_contributor(
            &99,
            &PayoutTarget {
                stellar_address: Some(setup.contributor.clone()),
            },
        )
        .unwrap()
        .unwrap();
    let after_assign = setup.env.events().all().len();
    assert!(
        after_assign >= 4,
        "expected >=4 events after assign_contributor, got {after_assign}"
    );
    setup.client.try_release_funds(&99).unwrap().unwrap();
    let after_release = setup.env.events().all().len();
    assert!(
        after_release >= 5,
        "expected >=5 events after release_funds, got {after_release}"
    );
}

#[test]
fn test_e2e_event_sequence_full_release() {
    use crate::events::EventKey;
    use soroban_sdk::IntoVal;

    let setup = setup_e2e_env(1_000_000);

    let deposit_amount: i128 = 500_000;
    let reward: i128 = 300_000;
    let issue_id: u64 = 77;

    setup
        .client
        .try_deposit_funds(&deposit_amount)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_create_milestone(
            &issue_id,
            &String::from_str(&setup.env, "E2E event milestone"),
            &reward,
        )
        .unwrap()
        .unwrap();

    let contributor_payout = PayoutTarget {
        stellar_address: Some(setup.contributor.clone()),
    };
    setup
        .client
        .try_assign_contributor(&issue_id, &contributor_payout)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_release_funds(&issue_id)
        .unwrap()
        .unwrap();

    // Retrieve the stored contributor from the milestone so we can reconstruct
    // the exact PayoutTarget value that was serialised into the FundsReleased
    // topic (it must be identical to what the contract stored).
    let stored_contributor = setup.client.get_milestone(&issue_id).contributor;

    // All events emitted during this test.  Each entry is a
    // (contract_id, topics_as_Val, data_as_Val) triple inside a soroban Vec.
    let events = setup.env.events().all();

    // Five contract events expected:
    //   0 – EscrowInitialized  (repo_id=1, maintainer)
    //   1 – FundsDeposited     (amount, new_total)
    //   2 – MilestoneCreated   (issue_id, reward)
    //   3 – ContributorAssigned(issue_id, contributor)
    //   4 – FundsReleased      (issue_id, contributor, reward)
    assert_eq!(
        events.len(),
        5,
        "expected 5 lifecycle events, got {}",
        events.len()
    );

    // Build the expected full event list using the same topic-tuple form that
    // emit_* functions use, then convert with `.into_val()` so the comparison
    // is against `Val` in the same way the SDK docs prescribe.

    let expected = soroban_sdk::vec![
        &setup.env,
        // 0: EscrowInitialized
        (
            setup.contract_id.clone(),
            (
                EventKey::EscrowInitialized,
                1_u64,                    // repo_id passed to initialize
                setup.maintainer.clone(), // maintainer
            )
                .into_val(&setup.env),
            ().into_val(&setup.env),
        ),
        // 1: FundsDeposited
        (
            setup.contract_id.clone(),
            (
                EventKey::FundsDeposited,
                deposit_amount,
                deposit_amount, // new_total == deposit_amount (first deposit)
            )
                .into_val(&setup.env),
            ().into_val(&setup.env),
        ),
        // 2: MilestoneCreated
        (
            setup.contract_id.clone(),
            (EventKey::MilestoneCreated, issue_id, reward).into_val(&setup.env),
            ().into_val(&setup.env),
        ),
        // 3: ContributorAssigned
        (
            setup.contract_id.clone(),
            (
                EventKey::ContributorAssigned,
                issue_id,
                contributor_payout.clone(),
            )
                .into_val(&setup.env),
            ().into_val(&setup.env),
        ),
        // 4: FundsReleased
        (
            setup.contract_id.clone(),
            (
                EventKey::FundsReleased,
                issue_id,
                stored_contributor,
                reward,
            )
                .into_val(&setup.env),
            ().into_val(&setup.env),
        ),
    ];

    assert_eq!(events, expected);
}
