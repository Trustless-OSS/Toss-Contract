#![cfg(test)]

use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::Events as _;

use super::*;
use crate::error::ContractError;
use crate::types::MilestoneStatus;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{token, Address, Env, String, Vec};

#[soroban_sdk::contract]
pub struct MockCctpContract;

#[soroban_sdk::contractimpl]
impl MockCctpContract {
    pub fn deposit_for_burn(
        env: Env,
        amount: i128,
        destination_domain: u32,
        mint_recipient: soroban_sdk::BytesN<32>,
        mint_token: Address,
    ) -> u64 {
        // Just return a dummy value
        1
    }
}

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
    
    // Register mock CCTP contract
    let cctp_address = soroban_sdk::Address::from_string(&soroban_sdk::String::from_str(&env, cctp::CCTP_TOKEN_MESSENGER_MINTER));
    env.register_contract(Some(&cctp_address), MockCctpContract);

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
        contributor: PayoutTarget::None,
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
        assert_eq!(loaded.contributor, PayoutTarget::None);
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
        // `get_ttl` excludes the current ledger, so an `extend_to` value of
        // 200,000 is observed as 199,999 at sequence number 1.
        assert_eq!(ttl, 199_999);
    });
}

#[test]
fn test_ttl_extended_on_milestone_write() {
    let (env, contract_id) = setup_env();

    let milestone = Milestone {
        issue_id: 1,
        title: String::from_str(&env, "Test"),
        reward: 100_000_000,
        contributor: PayoutTarget::None,
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
        assert_eq!(ttl, 199_999);
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
        assert_eq!(ttl, 199_999);
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
        assert_eq!(ttl, 199_999);
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
        contributor: PayoutTarget::Stellar(Address::generate(&env)),
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
        contributor: PayoutTarget::None,
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
        contributor: PayoutTarget::Stellar(Address::generate(&env)),
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
            contributor: PayoutTarget::Stellar(Address::generate(&setup.env)),
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
            contributor: PayoutTarget::Stellar(contributor.clone()),
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
            contributor: PayoutTarget::Stellar(contributor.clone()),
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
// CCTP payouts
// ---------------------------------------------------------------------------

#[test]
fn test_cctp_invalid_domain() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 2,
            title: String::from_str(&setup.env, "CCTP invalid domain"),
            reward: 500,
            contributor: PayoutTarget::Cctp(
                999,
                soroban_sdk::BytesN::from_array(&setup.env, &[1; 32]),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 2, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_release_funds(&2);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidDomain);
}

#[test]
fn test_cctp_empty_recipient() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 3,
            title: String::from_str(&setup.env, "CCTP empty recipient"),
            reward: 500,
            contributor: PayoutTarget::Cctp(
                0,
                soroban_sdk::BytesN::from_array(&setup.env, &[0; 32]),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 3, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_release_funds(&3);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EmptyRecipient);
}

#[test]
fn test_cctp_zero_burn_amount() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 4,
            title: String::from_str(&setup.env, "CCTP zero burn amount"),
            reward: 5, // < 10 stroops, normalizes to 0
            contributor: PayoutTarget::Cctp(
                5,
                soroban_sdk::BytesN::from_array(&setup.env, &[1; 32]),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 4, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 5;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_release_funds(&4);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroBurnAmount);
}

#[test]
fn test_cctp_release_exact_multiple() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut valid_recipient = [0u8; 32];
    valid_recipient[31] = 1; // Domain 0 (Ethereum), valid padding

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 5,
            title: String::from_str(&setup.env, "CCTP exact multiple"),
            reward: 500, // exact multiple of 10
            contributor: PayoutTarget::Cctp(
                0,
                soroban_sdk::BytesN::from_array(&setup.env, &valid_recipient),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 5, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    setup.client.try_release_funds(&5).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&5);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 500);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 500);
}

#[test]
fn test_cctp_release_non_multiple() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut valid_recipient = [0u8; 32];
    valid_recipient[31] = 1;

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 6,
            title: String::from_str(&setup.env, "CCTP non multiple"),
            reward: 507, // non multiple of 10
            contributor: PayoutTarget::Cctp(
                0,
                soroban_sdk::BytesN::from_array(&setup.env, &valid_recipient),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 6, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 507;
        storage::set_escrow(&setup.env, &escrow);
    });

    setup.client.try_release_funds(&6).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&6);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 500);

    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 0);
    assert_eq!(escrow.total_released, 500);
    
    // Remaining 7 stroops stays in the available balance.
    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 500); // 1000 total deposited - 0 reserved - 500 total_released = 500

}

#[test]
fn test_cctp_invalid_padding() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut invalid_recipient = [0u8; 32];
    invalid_recipient[0] = 1; // Domain 0 (Ethereum), invalid padding

    let result = setup.client.try_assign_contributor(&1, &PayoutTarget::Cctp(
        0,
        soroban_sdk::BytesN::from_array(&setup.env, &invalid_recipient),
    ));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidCctpRecipientPadding);
}

#[test]
fn test_cctp_valid_solana_recipient() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut solana_recipient = [1u8; 32]; // non-zero high bytes

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
            title: String::from_str(&setup.env, "Solana recipient"),
            reward: 500,
            contributor: PayoutTarget::None,
            status: MilestoneStatus::Pending,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 1, &milestone);
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_assign_contributor(&1, &PayoutTarget::Cctp(
        5, // Solana domain
        soroban_sdk::BytesN::from_array(&setup.env, &solana_recipient),
    ));
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// End-to-End Milestone Lifecycle Tests
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_full_release_flow() {
    let setup = setup_funding_env(10_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);
    let contributor = Address::generate(&setup.env);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();
    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 4_000_000);
    assert_eq!(balance.available, 6_000_000);

    setup.client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor.clone()))
        .unwrap()
        .unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Active);

    setup.client.try_release_funds(&1).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 4_000_000);
    assert!(milestone.released_at.is_some());

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 6_000_000);
    assert_eq!(balance.total_released, 4_000_000);

    assert_eq!(token_client.balance(&contributor), 4_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), 6_000_000);
}

#[test]
fn test_e2e_partial_release_flow() {
    let setup = setup_funding_env(10_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);
    let contributor = Address::generate(&setup.env);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();
    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();

    setup.client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor.clone()))
        .unwrap()
        .unwrap();

    setup.client.try_partial_release(&1, &3_000_000).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Released);
    assert_eq!(milestone.actual_released, 3_000_000);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 7_000_000);
    assert_eq!(balance.total_released, 3_000_000);

    assert_eq!(token_client.balance(&contributor), 3_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), 7_000_000);
}

#[test]
fn test_e2e_reassign_then_release() {
    let setup = setup_funding_env(10_000_000);
    let token_client = token::Client::new(&setup.env, &setup.token);
    let contributor1 = Address::generate(&setup.env);
    let contributor2 = Address::generate(&setup.env);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();
    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();

    setup.client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor1.clone()))
        .unwrap()
        .unwrap();

    setup.client
        .try_reassign_contributor(&1, &PayoutTarget::Stellar(contributor2.clone()))
        .unwrap()
        .unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.contributor, PayoutTarget::Stellar(contributor2.clone()));

    setup.client.try_release_funds(&1).unwrap().unwrap();

    assert_eq!(token_client.balance(&contributor1), 0);
    assert_eq!(token_client.balance(&contributor2), 4_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), 6_000_000);
}

#[test]
fn test_e2e_cancel_from_pending_flow() {
    let setup = setup_funding_env(10_000_000);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();
    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 4_000_000);
    assert_eq!(balance.available, 6_000_000);

    setup.client.try_cancel_milestone(&1).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Cancelled);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 10_000_000);
}

#[test]
fn test_e2e_cancel_from_active_flow() {
    let setup = setup_funding_env(10_000_000);
    let contributor = Address::generate(&setup.env);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();
    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();
    setup.client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor))
        .unwrap()
        .unwrap();

    setup.client.try_cancel_milestone(&1).unwrap().unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Cancelled);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.available, 10_000_000);
}

#[test]
fn test_e2e_multiple_milestones_accounting() {
    let setup = setup_funding_env(10_000_000);
    let contributor1 = Address::generate(&setup.env);
    let contributor2 = Address::generate(&setup.env);

    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();

    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "M1"), &2_000_000)
        .unwrap()
        .unwrap();
    setup.client
        .try_create_milestone(&2, &String::from_str(&setup.env, "M2"), &3_000_000)
        .unwrap()
        .unwrap();
    setup.client
        .try_create_milestone(&3, &String::from_str(&setup.env, "M3"), &4_000_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 9_000_000);
    assert_eq!(balance.available, 1_000_000);

    setup.client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor1))
        .unwrap()
        .unwrap();
    setup.client.try_release_funds(&1).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 7_000_000);
    assert_eq!(balance.total_released, 2_000_000);
    assert_eq!(balance.available, 1_000_000);

    setup.client
        .try_assign_contributor(&2, &PayoutTarget::Stellar(contributor2))
        .unwrap()
        .unwrap();
    setup.client.try_partial_release(&2, &1_500_000).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 4_000_000);
    assert_eq!(balance.total_released, 3_500_000);
    assert_eq!(balance.available, 2_500_000);

    setup.client.try_cancel_milestone(&3).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.total_released, 3_500_000);
    assert_eq!(balance.available, 6_500_000);
}

#[test]
fn test_e2e_deposit_after_milestones_created() {
    let setup = setup_funding_env(10_000_000);
    setup.client.try_deposit_funds(&5_000_000).unwrap().unwrap();

    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "M1"), &3_000_000)
        .unwrap()
        .unwrap();

    let balance_before = setup.client.get_balance();
    assert_eq!(balance_before.reserved, 3_000_000);
    assert_eq!(balance_before.available, 2_000_000);

    let sac = token::StellarAssetClient::new(&setup.env, &setup.token);
    sac.mint(&setup.maintainer, &5_000_000);
    setup.client.try_deposit_funds(&5_000_000).unwrap().unwrap();

    let balance_after = setup.client.get_balance();
    assert_eq!(balance_after.reserved, 3_000_000);
    assert_eq!(balance_after.available, 7_000_000);
}

#[test]
fn test_e2e_withdraw_up_to_reserved_boundary() {
    let setup = setup_funding_env(10_000_000);
    setup.client.try_deposit_funds(&10_000_000).unwrap().unwrap();

    setup.client
        .try_create_milestone(&1, &String::from_str(&setup.env, "M1"), &6_000_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 4_000_000);

    let result = setup.client.try_withdraw_funds(&4_000_001);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::WithdrawExceedsAvailable);

    let result_ok = setup.client.try_withdraw_funds(&4_000_000);
    assert!(result_ok.is_ok());

    let balance = setup.client.get_balance();
    assert_eq!(balance.available, 0);
    assert_eq!(balance.total_deposited, 6_000_000);
}

#[test]
fn test_e2e_event_sequence_check() {
    use soroban_sdk::TryFromVal;
    
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let platform = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_contract.address();
    let contributor = Address::generate(&env);

    // 1. EscrowInitialized
    c.try_initialize(&1, &maintainer, &platform, &token).unwrap().unwrap();
    {
        let init_events = env.events().all();
        let mut escrow_init_event = None;
        for event in init_events {
            if event.0 == contract_id {
                escrow_init_event = Some((event.1, event.2));
            }
        }
        let (topics, payload) = escrow_init_event.unwrap();
        let event_type = events::DataKey::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        let repo_id = u64::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
        let maintainer_val = Address::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
        
        assert_eq!(event_type, events::DataKey::EscrowInitialized);
        assert_eq!(repo_id, 1);
        assert_eq!(maintainer_val, maintainer);
        assert_eq!(<()>::try_from_val(&env, &payload).unwrap(), ());
    }

    // Mint token for maintainer
    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&maintainer, &10_000_000);

    // 2. FundsDeposited
    c.try_deposit_funds(&10_000_000).unwrap().unwrap();
    {
        let deposit_events = env.events().all();
        let mut deposit_event = None;
        for event in deposit_events {
            if event.0 == contract_id {
                deposit_event = Some((event.1, event.2));
            }
        }
        let (topics, payload) = deposit_event.unwrap();
        let event_type = events::DataKey::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        let amount = i128::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
        let new_total = i128::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
        
        assert_eq!(event_type, events::DataKey::FundsDeposited);
        assert_eq!(amount, 10_000_000);
        assert_eq!(new_total, 10_000_000);
        assert_eq!(<()>::try_from_val(&env, &payload).unwrap(), ());
    }

    // 3. MilestoneCreated
    c.try_create_milestone(&1, &String::from_str(&env, "Milestone 1"), &4_000_000)
        .unwrap()
        .unwrap();
    {
        let create_events = env.events().all();
        let mut create_event = None;
        for event in create_events {
            if event.0 == contract_id {
                create_event = Some((event.1, event.2));
            }
        }
        let (topics, payload) = create_event.unwrap();
        let event_type = events::DataKey::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        let issue_id = u64::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
        let reward = i128::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
        
        assert_eq!(event_type, events::DataKey::MilestoneCreated);
        assert_eq!(issue_id, 1);
        assert_eq!(reward, 4_000_000);
        assert_eq!(<()>::try_from_val(&env, &payload).unwrap(), ());
    }

    // 4. ContributorAssigned
    c.try_assign_contributor(&1, &PayoutTarget::Stellar(contributor.clone()))
        .unwrap()
        .unwrap();
    {
        let assign_events = env.events().all();
        let mut assign_event = None;
        for event in assign_events {
            if event.0 == contract_id {
                assign_event = Some((event.1, event.2));
            }
        }
        let (topics, payload) = assign_event.unwrap();
        let event_type = events::DataKey::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        let issue_id = u64::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
        let contributor_val = PayoutTarget::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
        
        assert_eq!(event_type, events::DataKey::ContributorAssigned);
        assert_eq!(issue_id, 1);
        assert_eq!(contributor_val, PayoutTarget::Stellar(contributor.clone()));
        assert_eq!(<()>::try_from_val(&env, &payload).unwrap(), ());
    }

    // 5. FundsReleased
    c.try_release_funds(&1).unwrap().unwrap();
    {
        let release_events = env.events().all();
        let mut release_event = None;
        for event in release_events {
            if event.0 == contract_id {
                release_event = Some((event.1, event.2));
            }
        }
        let (topics, payload) = release_event.unwrap();
        let event_type = events::DataKey::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        let issue_id = u64::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
        let contributor_val = PayoutTarget::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
        let amount = i128::try_from_val(&env, &topics.get(3).unwrap()).unwrap();
        
        assert_eq!(event_type, events::DataKey::FundsReleased);
        assert_eq!(issue_id, 1);
        assert_eq!(contributor_val, PayoutTarget::Stellar(contributor));
        assert_eq!(amount, 4_000_000);
        assert_eq!(<()>::try_from_val(&env, &payload).unwrap(), ());
    }
}

