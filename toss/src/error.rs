use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    // Auth
    NotAdmin = 1,

    // Escrow
    EscrowNotFound = 10,
    EscrowAlreadyExists = 11,
    EscrowInactive = 12,
    EscrowAlreadyActive = 13,

    // Balance
    InsufficientBalance = 20, // deposit would leave pool underfunded
    WithdrawExceedsAvailable = 21,
    ZeroAmount = 22,
    BalanceInvariantBroken = 23,

    // Milestone
    MilestoneNotFound = 30,
    MilestoneNotPending = 31, // assign_contributor requires Pending
    MilestoneNotActive = 32,  // release/reassignment requires Active
    DuplicateIssueId = 33,
    ReleaseTooLarge = 34, // release_funds amount > milestone reward
    MilestoneNotCancellable = 35,

    // Contributor
    ContributorNotSet = 40,

    // CCTP
    InvalidDomain = 50,
    EmptyRecipient = 51,
    ZeroBurnAmount = 52,
    InvalidCctpRecipientPadding = 53,
    ZeroPageLimit = 60, // list_milestones rejects limit == 0
}
