use soroban_sdk::{contractevent, contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PayoutTarget {
    None,
    Stellar(Address),
    Cctp(u32, BytesN<32>),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MilestoneStatus {
    Pending,   
    Active,    
    Released,  
    Cancelled, 
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub issue_id: u64,
    pub title: String,
    pub reward: i128, // in stroops (1 USDC = 10_000_000)
    pub contributor: PayoutTarget,
    pub status: MilestoneStatus,
    pub created_at: u64, // ledger timestamp
    pub released_at: Option<u64>,
    pub actual_released: i128, // 0 unless partial_release was used
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowState {
    pub repo_id: u64,
    pub maintainer: Address,
    pub platform: Address,
    pub token: Address, // USDC SAC address
    pub total_deposited: i128,
    pub reserved: i128,       // sum of rewards for Pending + Active milestones
    pub total_released: i128, // cumulative released to contributors
    pub created_at: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BalanceInfo {
    pub total_deposited: i128,
    pub reserved: i128,
    pub available: i128, // total_deposited - reserved - total_released
    pub total_released: i128,
}

// ---------------------------------------------------------------------------
// Contract events
// ---------------------------------------------------------------------------

#[contractevent]
pub struct EscrowInitialized {
    #[topic]
    pub repo_id: u64,
    pub maintainer: Address,
}

#[contractevent]
pub struct FundsDeposited {
    pub amount: i128,
    pub new_total: i128,
}

#[contractevent]
pub struct FundsWithdrawn {
    pub amount: i128,
    pub new_available: i128,
}

#[contractevent]
pub struct MilestoneCreated {
    #[topic]
    pub issue_id: u64,
    pub reward: i128,
}

#[contractevent]
pub struct MilestoneUpdated {
    #[topic]
    pub issue_id: u64,
    pub old_reward: i128,
    pub new_reward: i128,
}

#[contractevent]
pub struct ContributorAssigned {
    #[topic]
    pub issue_id: u64,
    pub contributor: PayoutTarget,
}

#[contractevent]
pub struct ContributorReassigned {
    #[topic]
    pub issue_id: u64,
    pub new_contributor: PayoutTarget,
}

#[contractevent]
pub struct FundsReleased {
    #[topic]
    pub issue_id: u64,
    pub contributor: PayoutTarget,
    pub amount: i128,
}

#[contractevent]
pub struct PartialRelease {
    #[topic]
    pub issue_id: u64,
    pub contributor: PayoutTarget,
    pub released: i128,
    pub returned_to_pool: i128,
}

#[contractevent]
pub struct MilestoneCancelled {
    #[topic]
    pub issue_id: u64,
}

#[contractevent]
pub struct AdminTransferred {
    #[topic]
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contractevent]
pub struct PlatformUpdated {
    #[topic]
    pub old_platform: Address,
    pub new_platform: Address,
}

#[contractevent]
pub struct MaintainerUpdated {
    #[topic]
    pub old_maintainer: Address,
    pub new_maintainer: Address,
}
