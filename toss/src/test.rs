#![cfg(test)]

use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::Events as _;

use super::*;
use crate::error::ContractError;
use crate::types::MilestoneStatus;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::TryIntoVal;
use soroban_sdk::{token, Address, Env, Map, Symbol, Val, Vec};

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
    let contract_id = env.register_contract(None, TOSSContract);

    // Register mock CCTP contract
    let cctp_address = soroban_sdk::Address::from_string(&soroban_sdk::String::from_str(
        &env,
        cctp::CCTP_TOKEN_MESSENGER_MINTER,
    ));
    env.register_contract(Some(&cctp_address), MockCctpContract);

    (env, contract_id)
}

fn client(env: &Env, contract_id: &soroban_sdk::Address) -> TOSSContractClient<'static> {
    TOSSContractClient::new(env, contract_id)
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

/// Funds the escrow and creates `count` milestones (`issue_id`s 1..=count),
/// each reserving `reward`. Returns the ready-to-use setup.
fn setup_milestones(count: u32, reward: i128) -> FundingSetup {
    let total = reward * count as i128;
    let setup = setup_funding_env(total);
    setup.client.try_deposit_funds(&total).unwrap().unwrap();
    for i in 1..=count {
        setup
            .client
            .try_create_milestone(&(i as u64), &reward)
            .unwrap()
            .unwrap();
    }
    setup
}

#[test]
fn test_list_milestones_empty_after_init() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    let result = c.try_initialize(&1, &maintainer, &platform, &token);
    assert!(result.is_ok());

    let milestones = c.list_milestones(&0, &50);
    assert_eq!(milestones.len(), 0);
}

#[test]
fn test_list_milestones_first_page() {
    let setup = setup_milestones(5, 100);

    let milestones = setup.client.list_milestones(&0, &2);
    assert_eq!(milestones.len(), 2);
    assert_eq!(milestones.get(0).unwrap().issue_id, 1);
    assert_eq!(milestones.get(1).unwrap().issue_id, 2);
}

#[test]
fn test_list_milestones_pages_cover_all() {
    let setup = setup_milestones(5, 100);

    // Contiguous pages walk the whole list in creation order.
    let page1 = setup.client.list_milestones(&0, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().issue_id, 1);
    assert_eq!(page1.get(1).unwrap().issue_id, 2);

    let page2 = setup.client.list_milestones(&2, &2);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().issue_id, 3);
    assert_eq!(page2.get(1).unwrap().issue_id, 4);

    // A short final page returns just what remains.
    let page3 = setup.client.list_milestones(&4, &2);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().issue_id, 5);
}

#[test]
fn test_list_milestones_past_end_offset() {
    let setup = setup_milestones(3, 100);

    // Offset exactly at the end, and far past it, both return empty.
    assert_eq!(setup.client.list_milestones(&3, &10).len(), 0);
    assert_eq!(setup.client.list_milestones(&100, &10).len(), 0);
}

#[test]
fn test_list_milestones_zero_limit_rejected() {
    let setup = setup_milestones(3, 100);

    let result = setup.client.try_list_milestones(&0, &0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroPageLimit);
}

#[test]
fn test_list_milestones_limit_capped() {
    let setup = setup_milestones(60, 10);

    // A limit above the cap is clamped to the public 50-item page cap.
    let page = setup.client.list_milestones(&0, &100);
    assert_eq!(page.len(), 50);
    assert_eq!(page.get(0).unwrap().issue_id, 1);

    // The remainder is reachable past the first page.
    let rest = setup.client.list_milestones(&50, &100);
    assert_eq!(rest.len(), 10);
    assert_eq!(rest.get(0).unwrap().issue_id, 51);
}

// ---------------------------------------------------------------------------
// get_milestone_count
// ---------------------------------------------------------------------------

#[test]
fn test_get_milestone_count_after_init() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    assert_eq!(c.get_milestone_count(), 0);
}

#[test]
fn test_get_milestone_count_matches_list() {
    let setup = setup_milestones(4, 100);

    assert_eq!(setup.client.get_milestone_count(), 4);
    assert_eq!(setup.client.list_milestones(&0, &50).len(), 4);
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

    let result = c.try_release_funds(&1, &100);
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

    let result = c.try_release_funds(&2, &100);
    assert!(result.is_err());
}

#[test]
fn test_release_funds_too_large() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 3,
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

    let result = c.try_release_funds(&3, &150);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ReleaseTooLarge);
}

#[test]
fn test_release_funds_zero_amount_panics() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 4,
        reward: 100,
        contributor: PayoutTarget::Stellar(Address::generate(&env)),
        status: MilestoneStatus::Active,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };
    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 4, &milestone);
    });

    let result = c.try_release_funds(&4, &0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_release_funds_negative_amount_panics() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let milestone = Milestone {
        issue_id: 5,
        reward: 100,
        contributor: PayoutTarget::Stellar(Address::generate(&env)),
        status: MilestoneStatus::Active,
        created_at: 100,
        released_at: None,
        actual_released: 0,
    };
    env.as_contract(&contract_id, || {
        storage::set_milestone(&env, 5, &milestone);
    });

    let result = c.try_release_funds(&5, &-50);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

// ---------------------------------------------------------------------------
// Funding mechanics – helpers
// ---------------------------------------------------------------------------

struct FundingSetup {
    env: Env,
    contract_id: Address,
    client: TOSSContractClient<'static>,
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

fn assert_accounting_invariants(setup: &FundingSetup) {
    let balance = setup.client.get_balance();
    let escrow = setup.client.get_escrow();
    assert_eq!(balance.total_deposited, escrow.total_deposited);
    assert_eq!(balance.reserved, escrow.reserved);
    assert_eq!(balance.total_released, escrow.total_released);

    let mut expected_reserved = 0;
    let mut expected_released = 0;
    let milestones = setup.client.list_milestones(&0, &50);
    for milestone in milestones.iter() {
        match milestone.status {
            MilestoneStatus::Pending | MilestoneStatus::Active => {
                expected_reserved += milestone.reward;
                assert_eq!(milestone.actual_released, 0);
            }
            MilestoneStatus::Released => {
                expected_released += milestone.actual_released;
            }
            MilestoneStatus::Cancelled => {
                assert_eq!(milestone.actual_released, 0);
            }
        }
    }

    let expected_available = balance
        .total_deposited
        .checked_sub(expected_reserved)
        .unwrap()
        .checked_sub(expected_released)
        .unwrap();

    assert_eq!(balance.reserved, expected_reserved);
    assert_eq!(balance.total_released, expected_released);
    assert_eq!(balance.available, expected_available);
    assert!(balance.available >= 0);
}

fn assert_same_balance(before: &BalanceInfo, after: &BalanceInfo) {
    assert_eq!(before.total_deposited, after.total_deposited);
    assert_eq!(before.reserved, after.reserved);
    assert_eq!(before.available, after.available);
    assert_eq!(before.total_released, after.total_released);
}

fn valid_evm_recipient(env: &Env) -> soroban_sdk::BytesN<32> {
    let mut recipient = [0u8; 32];
    recipient[31] = 1;
    soroban_sdk::BytesN::from_array(env, &recipient)
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

/// Decodes the `FundsReleased` event emitted by the latest contract invocation,
/// returning `(actual_released, returned_to_pool)`.
fn funds_released_event(env: &Env, contract_id: &Address) -> (i128, i128) {
    let event = env
        .events()
        .all()
        .iter()
        .find(|e| &e.0 == contract_id)
        .expect("no FundsReleased event emitted");
    let data: Map<Symbol, Val> = event.2.try_into_val(env).unwrap();
    let actual_released: i128 = data
        .get_unchecked(Symbol::new(env, "actual_released"))
        .try_into_val(env)
        .unwrap();
    let returned_to_pool: i128 = data
        .get_unchecked(Symbol::new(env, "returned_to_pool"))
        .try_into_val(env)
        .unwrap();
    (actual_released, returned_to_pool)
}

#[test]
fn test_release_funds_transfers_to_stellar_contributor() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
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

    setup.client.try_release_funds(&1, &500).unwrap().unwrap();

    // Event assertions must run before any other invocation: the test host
    // clears the event log at the start of every top-level call.
    let (actual_released, returned_to_pool) = funds_released_event(&setup.env, &setup.contract_id);
    assert_eq!(actual_released, 500);
    assert_eq!(returned_to_pool, 0);

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
fn test_release_funds_partial_transfers_to_stellar_contributor() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
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

    setup.client.try_release_funds(&1, &400).unwrap().unwrap();

    // Event assertions must run before any other invocation: the test host
    // clears the event log at the start of every top-level call.
    let (actual_released, returned_to_pool) = funds_released_event(&setup.env, &setup.contract_id);
    assert_eq!(actual_released, 400);
    assert_eq!(returned_to_pool, 100);

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

    let result = setup.client.try_release_funds(&2, &500);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidDomain);
}

#[test]
fn test_cctp_empty_recipient() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 3,
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

    let result = setup.client.try_release_funds(&3, &500);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EmptyRecipient);
}

#[test]
fn test_cctp_zero_burn_amount() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 4,
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

    let result = setup.client.try_release_funds(&4, &5);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroBurnAmount);
}

#[test]
fn test_cctp_release_zero_amount_rejected() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 7,
            reward: 500,
            contributor: PayoutTarget::Cctp(
                5,
                soroban_sdk::BytesN::from_array(&setup.env, &[1; 32]),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 7, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    // Zero is rejected by the amount guard before truncation or burn checks.
    let result = setup.client.try_release_funds(&7, &0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_cctp_release_negative_amount_rejected() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 8,
            reward: 500,
            contributor: PayoutTarget::Cctp(
                5,
                soroban_sdk::BytesN::from_array(&setup.env, &[1; 32]),
            ),
            status: MilestoneStatus::Active,
            created_at: 100,
            released_at: None,
            actual_released: 0,
        };
        storage::set_milestone(&setup.env, 8, &milestone);

        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved += 500;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_release_funds(&8, &-10);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
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

    setup.client.try_release_funds(&5, &500).unwrap().unwrap();

    // Event assertions must run before any other invocation: the test host
    // clears the event log at the start of every top-level call.
    let (actual_released, returned_to_pool) = funds_released_event(&setup.env, &setup.contract_id);
    assert_eq!(actual_released, 500);
    assert_eq!(returned_to_pool, 0);

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

    setup.client.try_release_funds(&6, &507).unwrap().unwrap();

    // Event assertions must run before any other invocation: the test host
    // clears the event log at the start of every top-level call. The event
    // reports what actually left the contract and the dust credited back to
    // the pool.
    let (actual_released, returned_to_pool) = funds_released_event(&setup.env, &setup.contract_id);
    assert_eq!(actual_released, 500);
    assert_eq!(returned_to_pool, 7);

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

// ---------------------------------------------------------------------------
// Accounting invariants
// ---------------------------------------------------------------------------

#[test]
fn test_accounting_invariants_after_realistic_mixed_sequence() {
    let setup = setup_funding_env(10_000);
    setup.client.try_deposit_funds(&10_000).unwrap().unwrap();
    assert_accounting_invariants(&setup);

    setup
        .client
        .try_create_milestone(&1, &1_000)
        .unwrap()
        .unwrap();
    assert_accounting_invariants(&setup);

    setup
        .client
        .try_update_milestone(&1, &1_300)
        .unwrap()
        .unwrap();
    assert_accounting_invariants(&setup);

    setup
        .client
        .try_update_milestone(&1, &900)
        .unwrap()
        .unwrap();
    assert_accounting_invariants(&setup);

    let active_contributor = Address::generate(&setup.env);
    setup
        .client
        .try_create_milestone(&2, &1_000)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(&2, &PayoutTarget::Stellar(active_contributor))
        .unwrap()
        .unwrap();
    setup.client.try_cancel_milestone(&2).unwrap().unwrap();
    assert_eq!(
        setup.client.get_milestone(&2).status,
        MilestoneStatus::Cancelled
    );
    assert_accounting_invariants(&setup);

    let full_release_contributor = Address::generate(&setup.env);
    setup
        .client
        .try_create_milestone(&3, &700)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(&3, &PayoutTarget::Stellar(full_release_contributor))
        .unwrap()
        .unwrap();
    setup.client.try_release_funds(&3, &700).unwrap().unwrap();
    assert_accounting_invariants(&setup);

    let partial_release_contributor = Address::generate(&setup.env);
    setup
        .client
        .try_create_milestone(&4, &800)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(&4, &PayoutTarget::Stellar(partial_release_contributor))
        .unwrap()
        .unwrap();
    setup.client.try_release_funds(&4, &500).unwrap().unwrap();
    assert_accounting_invariants(&setup);

    setup
        .client
        .try_create_milestone(&5, &507)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(&5, &PayoutTarget::Cctp(0, valid_evm_recipient(&setup.env)))
        .unwrap()
        .unwrap();
    setup.client.try_release_funds(&5, &507).unwrap().unwrap();

    let cctp_milestone = setup.client.get_milestone(&5);
    assert_eq!(cctp_milestone.status, MilestoneStatus::Released);
    assert_eq!(cctp_milestone.actual_released, 500);

    let balance_before_withdraw = setup.client.get_balance();
    assert_eq!(balance_before_withdraw.reserved, 900);
    assert_eq!(balance_before_withdraw.total_released, 1_700);
    assert_eq!(balance_before_withdraw.available, 7_400);
    assert_accounting_invariants(&setup);

    setup
        .client
        .try_withdraw_funds(&balance_before_withdraw.available)
        .unwrap()
        .unwrap();

    let balance_after_withdraw = setup.client.get_balance();
    assert_eq!(balance_after_withdraw.total_deposited, 2_600);
    assert_eq!(balance_after_withdraw.available, 0);
    assert_accounting_invariants(&setup);
}

#[derive(Clone, Copy)]
enum AccountingOp {
    Create(u64, i128),
    Update(u64, i128),
    Assign(u64),
    Cancel(u64),
    Release(u64, i128),
    Withdraw(i128),
}

fn run_accounting_sequence(ops: &[AccountingOp]) {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    assert_accounting_invariants(&setup);

    for op in ops {
        match *op {
            AccountingOp::Create(issue_id, reward) => {
                setup
                    .client
                    .try_create_milestone(&issue_id, &reward)
                    .unwrap()
                    .unwrap();
            }
            AccountingOp::Update(issue_id, reward) => {
                setup
                    .client
                    .try_update_milestone(&issue_id, &reward)
                    .unwrap()
                    .unwrap();
            }
            AccountingOp::Assign(issue_id) => {
                let contributor = Address::generate(&setup.env);
                setup
                    .client
                    .try_assign_contributor(&issue_id, &PayoutTarget::Stellar(contributor))
                    .unwrap()
                    .unwrap();
            }
            AccountingOp::Cancel(issue_id) => {
                setup
                    .client
                    .try_cancel_milestone(&issue_id)
                    .unwrap()
                    .unwrap();
            }
            AccountingOp::Release(issue_id, amount) => {
                setup
                    .client
                    .try_release_funds(&issue_id, &amount)
                    .unwrap()
                    .unwrap();
            }
            AccountingOp::Withdraw(amount) => {
                setup.client.try_withdraw_funds(&amount).unwrap().unwrap();
            }
        }

        assert_accounting_invariants(&setup);
    }
}

#[test]
fn test_accounting_invariants_after_looped_operation_sequences() {
    let sequences: [&[AccountingOp]; 2] = [
        &[
            AccountingOp::Create(1, 200),
            AccountingOp::Create(2, 150),
            AccountingOp::Update(1, 250),
            AccountingOp::Assign(1),
            AccountingOp::Release(1, 200),
            AccountingOp::Cancel(2),
            AccountingOp::Withdraw(800),
        ],
        &[
            AccountingOp::Create(1, 400),
            AccountingOp::Update(1, 300),
            AccountingOp::Assign(1),
            AccountingOp::Cancel(1),
            AccountingOp::Create(2, 250),
            AccountingOp::Assign(2),
            AccountingOp::Release(2, 250),
            AccountingOp::Withdraw(750),
        ],
    ];

    for sequence in sequences {
        run_accounting_sequence(sequence);
    }
}

#[test]
fn test_cctp_truncation_remainder_stays_available_not_reserved() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    setup
        .client
        .try_create_milestone(&1, &507)
        .unwrap()
        .unwrap();
    setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Cctp(0, valid_evm_recipient(&setup.env)))
        .unwrap()
        .unwrap();

    setup.client.try_release_funds(&1, &507).unwrap().unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 0);
    assert_eq!(balance.total_released, 500);
    assert_eq!(balance.available, 500);
    assert_accounting_invariants(&setup);
}

#[test]
fn test_paused_escrow_rejections_do_not_mutate_balances() {
    let setup = setup_pending_milestone(1_000, 300);
    let before = setup.client.get_balance();

    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    assert_eq!(
        setup.client.try_deposit_funds(&100).unwrap_err().unwrap(),
        ContractError::EscrowInactive
    );
    assert_eq!(
        setup.client.try_withdraw_funds(&100).unwrap_err().unwrap(),
        ContractError::EscrowInactive
    );
    assert_eq!(
        setup
            .client
            .try_create_milestone(&2, &100)
            .unwrap_err()
            .unwrap(),
        ContractError::EscrowInactive
    );
    assert_eq!(
        setup
            .client
            .try_update_milestone(&1, &400)
            .unwrap_err()
            .unwrap(),
        ContractError::EscrowInactive
    );
    assert_eq!(
        setup.client.try_cancel_milestone(&1).unwrap_err().unwrap(),
        ContractError::EscrowInactive
    );

    let after = setup.client.get_balance();
    assert_same_balance(&before, &after);
    assert_accounting_invariants(&setup);
}

#[test]
fn test_get_balance_rejects_negative_available_invariant() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&100).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved = 101;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_get_balance();
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::BalanceInvariantBroken
    );
}

#[test]
fn test_deposit_rejects_total_deposited_overflow() {
    let setup = setup_funding_env(1);

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.total_deposited = i128::MAX;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_deposit_funds(&1);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::BalanceInvariantBroken
    );
}

#[test]
fn test_release_rejects_reserved_underflow_without_mutating_milestone() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    setup
        .client
        .try_create_milestone(&1, &300)
        .unwrap()
        .unwrap();

    let contributor = Address::generate(&setup.env);
    setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor))
        .unwrap()
        .unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved = 100;
        storage::set_escrow(&setup.env, &escrow);
    });

    let result = setup.client.try_release_funds(&1, &300);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::BalanceInvariantBroken
    );

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.status, MilestoneStatus::Active);
    assert_eq!(milestone.actual_released, 0);
}

#[test]
fn test_cctp_invalid_padding() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut invalid_recipient = [0u8; 32];
    invalid_recipient[0] = 1; // Domain 0 (Ethereum), invalid padding

    let result = setup.client.try_assign_contributor(
        &1,
        &PayoutTarget::Cctp(
            0,
            soroban_sdk::BytesN::from_array(&setup.env, &invalid_recipient),
        ),
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InvalidCctpRecipientPadding
    );
}

#[test]
fn test_cctp_valid_solana_recipient() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();

    let mut solana_recipient = [1u8; 32]; // non-zero high bytes

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
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

    let result = setup.client.try_assign_contributor(
        &1,
        &PayoutTarget::Cctp(
            5, // Solana domain
            soroban_sdk::BytesN::from_array(&setup.env, &solana_recipient),
        ),
    );
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// admin rotation
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_admin_success() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let new_admin = Address::generate(&env);
    let result = c.try_transfer_admin(&new_admin);
    assert!(result.is_ok());

    env.as_contract(&contract_id, || {
        let stored = storage::get_admin(&env);
        assert_eq!(stored, Some(new_admin));
    });
}

#[test]
fn test_transfer_admin_emits_event() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap()
        .unwrap();

    let new_admin = Address::generate(&env);
    c.try_transfer_admin(&new_admin).unwrap().unwrap();

    let events = env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_update_platform_success() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let new_platform = Address::generate(&env);
    let result = c.try_update_platform(&new_platform);
    assert!(result.is_ok());

    let escrow = c.get_escrow();
    assert_eq!(escrow.platform, new_platform);
}

#[test]
fn test_update_platform_emits_event() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap()
        .unwrap();

    let new_platform = Address::generate(&env);
    c.try_update_platform(&new_platform).unwrap().unwrap();

    let events = env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_update_maintainer_success() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let new_maintainer = Address::generate(&env);
    let result = c.try_update_maintainer(&new_maintainer);
    assert!(result.is_ok());

    let escrow = c.get_escrow();
    assert_eq!(escrow.maintainer, new_maintainer);
}

#[test]
fn test_update_maintainer_emits_event() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap()
        .unwrap();

    let new_maintainer = Address::generate(&env);
    c.try_update_maintainer(&new_maintainer).unwrap().unwrap();

    let events = env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_old_platform_rejected_after_update() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let new_platform = Address::generate(&env);
    c.try_update_platform(&new_platform).unwrap();

    let escrow = c.get_escrow();
    assert_eq!(escrow.platform, new_platform);
    assert_ne!(escrow.platform, platform);
}

#[test]
fn test_old_maintainer_rejected_after_update() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    let new_maintainer = Address::generate(&env);
    c.try_update_maintainer(&new_maintainer).unwrap();

    let escrow = c.get_escrow();
    assert_eq!(escrow.maintainer, new_maintainer);
    assert_ne!(escrow.maintainer, maintainer);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_transfer_admin_rejects_non_admin() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    env.set_auths(&[]);
    let new_admin = Address::generate(&env);
    c.transfer_admin(&new_admin);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_update_platform_rejects_non_admin() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    env.set_auths(&[]);
    let new_platform = Address::generate(&env);
    c.update_platform(&new_platform);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_update_maintainer_rejects_non_admin() {
    let (env, contract_id) = setup_env();
    let c = client(&env, &contract_id);
    env.mock_all_auths();

    let (maintainer, platform, token) = addresses(&env);
    c.try_initialize(&1, &maintainer, &platform, &token)
        .unwrap();

    env.set_auths(&[]);
    let new_maintainer = Address::generate(&env);
    c.update_maintainer(&new_maintainer);
}

// ---------------------------------------------------------------------------
// update_milestone
// ---------------------------------------------------------------------------

/// Funds the escrow with `deposit` and creates a single pending milestone
/// (`issue_id = 1`) reserving `reward`, returning the ready-to-use setup.
fn setup_pending_milestone(deposit: i128, reward: i128) -> FundingSetup {
    let setup = setup_funding_env(deposit);
    setup.client.try_deposit_funds(&deposit).unwrap().unwrap();
    setup
        .client
        .try_create_milestone(&1, &reward)
        .unwrap()
        .unwrap();
    setup
}

#[test]
fn test_update_milestone_same_reward_keeps_reservation() {
    let setup = setup_pending_milestone(1_000, 300);

    setup
        .client
        .try_update_milestone(&1, &300)
        .unwrap()
        .unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.reward, 300);
    assert_eq!(milestone.status, MilestoneStatus::Pending);

    // Reward unchanged, so the reservation is untouched.
    let escrow = setup.client.get_escrow();
    assert_eq!(escrow.reserved, 300);
    assert_eq!(setup.client.get_balance().available, 700);
}

#[test]
fn test_update_milestone_reward_increase_within_balance() {
    let setup = setup_pending_milestone(1_000, 300);

    setup
        .client
        .try_update_milestone(&1, &800)
        .unwrap()
        .unwrap();

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.reward, 800);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 800);
    assert_eq!(balance.available, 200); // 1000 - 800
}

#[test]
fn test_update_milestone_reward_increase_exceeding_balance() {
    let setup = setup_pending_milestone(1_000, 300);

    // Available is 700 on top of the 300 already reserved; asking for 1_100
    // (a delta of 800) can't fit.
    let result = setup.client.try_update_milestone(&1, &1_100);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::InsufficientBalance
    );

    // State is left untouched on rejection.
    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.reward, 300);
    assert_eq!(setup.client.get_escrow().reserved, 300);
}

#[test]
fn test_update_milestone_reward_decrease_frees_available() {
    let setup = setup_pending_milestone(1_000, 800);
    assert_eq!(setup.client.get_balance().available, 200);

    setup
        .client
        .try_update_milestone(&1, &500)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 500);
    assert_eq!(balance.available, 500); // 300 freed back into the pool
}

#[test]
fn test_update_milestone_reward_increase_to_full_available() {
    let setup = setup_pending_milestone(1_000, 300);

    // Delta of 700 exactly consumes the remaining available balance.
    setup
        .client
        .try_update_milestone(&1, &1_000)
        .unwrap()
        .unwrap();

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 1_000);
    assert_eq!(balance.available, 0);
}

#[test]
fn test_update_milestone_zero_reward_rejected() {
    let setup = setup_pending_milestone(1_000, 300);
    let result = setup.client.try_update_milestone(&1, &0);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_update_milestone_negative_reward_rejected() {
    let setup = setup_pending_milestone(1_000, 300);
    let result = setup.client.try_update_milestone(&1, &-50);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ZeroAmount);
}

#[test]
fn test_update_milestone_missing_milestone_rejected() {
    let setup = setup_pending_milestone(1_000, 300);
    let result = setup.client.try_update_milestone(&99, &100);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneNotFound
    );
}

#[test]
fn test_update_milestone_on_active_rejected() {
    let setup = setup_pending_milestone(1_000, 300);
    let contributor = Address::generate(&setup.env);
    setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor))
        .unwrap()
        .unwrap();

    let result = setup.client.try_update_milestone(&1, &400);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneNotPending
    );
}

#[test]
fn test_update_milestone_on_cancelled_rejected() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.client.try_cancel_milestone(&1).unwrap().unwrap();

    let result = setup.client.try_update_milestone(&1, &400);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneNotPending
    );
}

#[test]
fn test_update_milestone_on_released_rejected() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    // Seed a released milestone directly.
    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 7,
            reward: 500,
            contributor: PayoutTarget::Stellar(contributor),
            status: MilestoneStatus::Released,
            created_at: 100,
            released_at: Some(200),
            actual_released: 500,
        };
        storage::set_milestone(&setup.env, 7, &milestone);
    });

    let result = setup.client.try_update_milestone(&7, &400);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::MilestoneNotPending
    );
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_update_milestone_requires_maintainer() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.env.set_auths(&[]);
    setup.client.update_milestone(&1, &400);
}

#[test]
fn test_update_milestone_emits_event() {
    let setup = setup_pending_milestone(1_000, 300);

    setup
        .client
        .try_update_milestone(&1, &450)
        .unwrap()
        .unwrap();

    // `events().all()` reports the most recent invocation's events; the update
    // publishes exactly one, the MilestoneUpdated event.
    let events = setup.env.events().all();
    assert_eq!(events.len(), 1);
}

// pause_escrow and resume_escrow

#[test]
fn test_pause_escrow_success() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&100).unwrap().unwrap();

    let result = setup.client.try_pause_escrow(&None);
    assert!(result.is_ok());

    let escrow = setup.client.get_escrow();
    assert!(!escrow.is_active);
}

#[test]
fn test_resume_escrow_success() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_resume_escrow();
    assert!(result.is_ok());

    let escrow = setup.client.get_escrow();
    assert!(escrow.is_active);
}

#[test]
fn test_pause_escrow_emits_event() {
    let setup = setup_funding_env(1_000);

    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let events = setup.env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_resume_escrow_emits_event() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    setup.client.try_resume_escrow().unwrap().unwrap();

    let events = setup.env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_double_pause_rejected() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_pause_escrow(&None);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_double_resume_rejected() {
    let setup = setup_funding_env(1_000);

    let result = setup.client.try_resume_escrow();
    assert_eq!(
        result.unwrap_err().unwrap(),
        ContractError::EscrowAlreadyActive
    );
}

#[test]
fn test_pause_escrow_requires_admin() {
    let setup = setup_funding_env(1_000);

    setup.env.set_auths(&[]);
    let result = setup.client.try_pause_escrow(&None);
    assert!(result.is_err());
}

#[test]
fn test_resume_escrow_requires_admin() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    setup.env.set_auths(&[]);
    let result = setup.client.try_resume_escrow();
    assert!(result.is_err());
}

#[test]
fn test_deposit_funds_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_deposit_funds(&100);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_withdraw_funds_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_withdraw_funds(&100);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_create_milestone_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_create_milestone(&1, &100);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_update_milestone_blocked_when_paused() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_update_milestone(&1, &400);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_assign_contributor_blocked_when_paused() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let contributor = Address::generate(&setup.env);
    let result = setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_reassign_contributor_blocked_when_paused() {
    let setup = setup_pending_milestone(1_000, 300);
    let contributor = Address::generate(&setup.env);
    setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor))
        .unwrap()
        .unwrap();
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let new_contributor = Address::generate(&setup.env);
    let result = setup
        .client
        .try_reassign_contributor(&1, &PayoutTarget::Stellar(new_contributor));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_release_funds_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 1,
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

    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_release_funds(&1, &500);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_release_funds_partial_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&1_000).unwrap().unwrap();
    let contributor = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        let milestone = Milestone {
            issue_id: 2,
            reward: 500,
            contributor: PayoutTarget::Stellar(contributor.clone()),
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

    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_release_funds(&2, &300);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_cancel_milestone_blocked_when_paused() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let result = setup.client.try_cancel_milestone(&1);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_transfer_admin_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let new_admin = Address::generate(&setup.env);
    let result = setup.client.try_transfer_admin(&new_admin);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_update_platform_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let new_platform = Address::generate(&setup.env);
    let result = setup.client.try_update_platform(&new_platform);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_update_maintainer_blocked_when_paused() {
    let setup = setup_funding_env(1_000);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    let new_maintainer = Address::generate(&setup.env);
    let result = setup.client.try_update_maintainer(&new_maintainer);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);
}

#[test]
fn test_query_methods_work_while_paused() {
    let setup = setup_pending_milestone(1_000, 300);
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    // All query methods should still work
    let escrow = setup.client.get_escrow();
    assert!(!escrow.is_active);

    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.reward, 300);

    let balance = setup.client.get_balance();
    assert_eq!(balance.reserved, 300);

    let milestones = setup.client.list_milestones(&0, &50);
    assert_eq!(milestones.len(), 1);

    let count = setup.client.get_milestone_count();
    assert_eq!(count, 1);
}

#[test]
fn test_pause_resume_cycle() {
    let setup = setup_funding_env(1_000);

    // Initially active
    let escrow = setup.client.get_escrow();
    assert!(escrow.is_active);

    // Pause
    setup.client.try_pause_escrow(&None).unwrap().unwrap();
    let escrow = setup.client.get_escrow();
    assert!(!escrow.is_active);

    // Try to deposit (should fail)
    let result = setup.client.try_deposit_funds(&100);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::EscrowInactive);

    // Resume
    setup.client.try_resume_escrow().unwrap().unwrap();
    let escrow = setup.client.get_escrow();
    assert!(escrow.is_active);

    // Now deposit should work
    let result = setup.client.try_deposit_funds(&100);
    assert!(result.is_ok());

    let balance = setup.client.get_balance();
    assert_eq!(balance.total_deposited, 100);
}

#[test]
fn test_pause_preserves_balances() {
    let setup = setup_funding_env(1_000);
    setup.client.try_deposit_funds(&500).unwrap().unwrap();

    setup.env.as_contract(&setup.contract_id, || {
        let mut escrow = storage::get_escrow(&setup.env).unwrap();
        escrow.reserved = 200;
        escrow.total_released = 50;
        storage::set_escrow(&setup.env, &escrow);
    });

    let balance_before = setup.client.get_balance();
    setup.client.try_pause_escrow(&None).unwrap().unwrap();
    let balance_after = setup.client.get_balance();

    // Balances should not change
    assert_eq!(
        balance_before.total_deposited,
        balance_after.total_deposited
    );
    assert_eq!(balance_before.reserved, balance_after.reserved);
    assert_eq!(balance_before.available, balance_after.available);
    assert_eq!(balance_before.total_released, balance_after.total_released);
}

#[test]
fn test_pause_resume_with_milestone_state() {
    let setup = setup_pending_milestone(1_000, 300);

    // Pause
    setup.client.try_pause_escrow(&None).unwrap().unwrap();

    // Verify milestone is still accessible and unchanged
    let milestone = setup.client.get_milestone(&1);
    assert_eq!(milestone.reward, 300);
    assert_eq!(milestone.status, MilestoneStatus::Pending);

    // Resume
    setup.client.try_resume_escrow().unwrap().unwrap();

    // Can now assign contributor
    let contributor = Address::generate(&setup.env);
    setup
        .client
        .try_assign_contributor(&1, &PayoutTarget::Stellar(contributor))
        .unwrap()
        .unwrap();

    let milestone_after = setup.client.get_milestone(&1);
    assert_eq!(milestone_after.status, MilestoneStatus::Active);
}
