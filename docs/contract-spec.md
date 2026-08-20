# Contract specification

This is the detailed reference for the current `trustless-oss` contract. Source code is authoritative if this document and the implementation ever differ.

## Entry points

The contract exposes the following public methods:

| Method | Authorization | Purpose |
| --- | --- | --- |
| `initialize(repo_id, maintainer, platform, token)` | First call: maintainer; later calls: stored admin | Creates the single escrow and stores the admin. |
| `deposit_funds(amount)` | Maintainer | Transfers USDC from the maintainer into the contract. |
| `withdraw_funds(amount)` | Maintainer | Returns only available USDC to the maintainer. |
| `create_milestone(issue_id, reward)` | Maintainer | Creates a pending milestone and reserves its reward. Titles stay on GitHub; only the issue ID is stored. |
| `update_milestone(issue_id, reward)` | Maintainer | Edits the reward of a pending milestone, adjusting the reservation by the delta. |
| `assign_contributor(issue_id, contributor)` | Maintainer | Sets the payout target and moves a pending milestone to active. |
| `reassign_contributor(issue_id, contributor)` | Maintainer | Changes the payout target of an active milestone. |
| `release_funds(issue_id, amount)` | Platform | Pays up to the full reward and marks the milestone released; any unpaid remainder is returned to the available pool. An `amount` of zero or less is rejected with `ZeroAmount`. |
| `cancel_milestone(issue_id)` | Maintainer | Cancels a pending or active milestone and un-reserves its reward. |
| `get_escrow()` | None | Reads the escrow state. |
| `get_milestone(issue_id)` | None | Reads one milestone. |
| `get_balance()` | None | Reads deposited, reserved, released, and available amounts. |
| `list_milestones(offset, limit)` | None | Returns a paginated slice of milestones. A zero `limit` is rejected with `ZeroPageLimit`; limits above 50 are clamped. Past-the-end offsets return an empty vector. |
| `get_milestone_count()` | None | Returns the total number of milestones. |

Every state-changing method requires an active escrow. Amounts are integer token base units; for a 7-decimal USDC token, `10_000_000` base units equals 1 USDC.

## Failure policy

The contract uses two distinct failure channels:

- A missing or wrong signature fails through Soroban `require_auth()` and is an authorization panic. Signature failures are never translated into `NotPlatform` or `NotMaintainer` contract errors.
- Missing state and business-rule failures return `Err(ContractError::...)`. After initialization, an inactive escrow consistently returns `EscrowInactive` from every state-changing method except `resume_escrow`, which is the operation that restores activity. A repeat `initialize` call stops at its existence guard and returns `EscrowAlreadyExists` before authentication or activity checks.

`NotAdmin` is reserved for a missing admin storage key. If the key exists but its address did not authorize the call, `require_auth()` panics instead.

| Method | Authorization panic | Returned `ContractError` values |
| --- | --- | --- |
| `initialize` | First call: maintainer; repeat calls stop at the existence guard before authentication | `EscrowAlreadyExists` |
| `deposit_funds` | Maintainer | `EscrowNotFound`, `EscrowInactive`, `ZeroAmount`, `BalanceInvariantBroken` |
| `withdraw_funds` | Maintainer | `EscrowNotFound`, `EscrowInactive`, `ZeroAmount`, `WithdrawExceedsAvailable`, `BalanceInvariantBroken` |
| `create_milestone` | Maintainer | `EscrowNotFound`, `EscrowInactive`, `ZeroAmount`, `DuplicateIssueId`, `InsufficientBalance`, `BalanceInvariantBroken` |
| `update_milestone` | Maintainer | `EscrowNotFound`, `EscrowInactive`, `ZeroAmount`, `MilestoneNotFound`, `MilestoneNotPending`, `InsufficientBalance`, `BalanceInvariantBroken` |
| `assign_contributor` | Maintainer | `InvalidDomain`, `EmptyRecipient`, `InvalidCctpRecipientPadding`, `EscrowNotFound`, `EscrowInactive`, `MilestoneNotFound`, `MilestoneNotPending` |
| `reassign_contributor` | Maintainer | `InvalidDomain`, `EmptyRecipient`, `InvalidCctpRecipientPadding`, `EscrowNotFound`, `EscrowInactive`, `MilestoneNotFound`, `MilestoneNotActive` |
| `release_funds` | Platform | `EscrowNotFound`, `EscrowInactive`, `MilestoneNotFound`, `MilestoneNotActive`, `ZeroAmount`, `ReleaseTooLarge`, `ContributorNotSet`, `InvalidDomain`, `EmptyRecipient`, `ZeroBurnAmount`, `InvalidCctpRecipientPadding`, `BalanceInvariantBroken` |
| `cancel_milestone` | Maintainer | `EscrowNotFound`, `EscrowInactive`, `MilestoneNotFound`, `MilestoneNotCancellable`, `BalanceInvariantBroken` |
| `transfer_admin` | Stored admin | `NotAdmin`, `EscrowNotFound`, `EscrowInactive` |
| `update_platform` | Stored admin | `NotAdmin`, `EscrowNotFound`, `EscrowInactive` |
| `update_maintainer` | Stored admin | `NotAdmin`, `EscrowNotFound`, `EscrowInactive` |
| `pause_escrow` | Stored admin | `NotAdmin`, `EscrowNotFound`, `EscrowInactive` |
| `resume_escrow` | Stored admin | `NotAdmin`, `EscrowNotFound`, `EscrowAlreadyActive` |
| `get_escrow` | None | `EscrowNotFound` |
| `get_milestone` | None | `MilestoneNotFound` |
| `get_balance` | None | `EscrowNotFound`, `BalanceInvariantBroken` |
| `list_milestones` | None | `ZeroPageLimit`, `MilestoneNotFound` if the stored issue index is inconsistent |
| `get_milestone_count` | None | None |

## Data model

### `PayoutTarget`

```text
stellar_address: set for an assigned contributor; unset contributors cannot be paid
```

### `Milestone`

Stores `issue_id`, `reward`, `contributor`, `status`, `created_at`, `released_at`, and `actual_released`. The milestone **title is intentionally not stored on-chain** — it already lives on GitHub, so `create_milestone(issue_id, reward)` stores only the issue ID and reward, and `update_milestone(issue_id, reward)` changes only the reward.

### `MilestoneStatus`

- `Pending`: created but no contributor has been assigned.
- `Active`: contributor assigned and reward reserved.
- `Released`: payout completed; `actual_released` records what was paid.
- `Cancelled`: reward returned to the available pool.

### Balance invariant

The contract maintains this derived balance:

```text
available = total_deposited - reserved - total_released
```

`total_deposited` is the current principal accounted by the escrow, not lifetime deposit volume. Deposits increase it and withdrawals decrement it; indexers that need cumulative deposit volume must compute that from deposit events instead of this field.

The three accounting fields are locked together with checked arithmetic. `reserved` is the sum of rewards for milestones in `Pending` or `Active`, `total_released` is the sum of `actual_released` for milestones in `Released`, and cancelled milestones contribute zero to both totals. Milestone creation and reward increases must fit within `available`, withdrawals cannot exceed it, and releasing or cancelling a milestone reduces `reserved`. A partial release un-reserves the full reward but adds only the actual payout to `total_released`, so any unpaid or CCTP-truncated remainder stays in `available`.

## Storage and events

Persistent storage keys are:

| Key | Value |
| --- | --- |
| `Escrow` | `EscrowState` |
| `Milestone(issue_id)` | `Milestone` |
| `EscrowIssueIds` | `Vec<u64>` index used for listing |
| `Admin` | Stored initializer/admin address |

State-changing methods emit typed events for initialization, deposits, withdrawals, milestone creation and updates, contributor assignment/reassignment, releases, and cancellation. The event topic payloads are defined in `toss/src/events.rs`.

## Error groups

| Codes | Errors |
| --- | --- |
| 1 | `NotAdmin` (missing admin storage key only) |
| 10–13 | `EscrowNotFound`, `EscrowAlreadyExists`, `EscrowInactive`, `EscrowAlreadyActive` |
| 20–23 | `InsufficientBalance`, `WithdrawExceedsAvailable`, `ZeroAmount`, `BalanceInvariantBroken` |
| 30–35 | `MilestoneNotFound`, `MilestoneNotPending`, `MilestoneNotActive`, `DuplicateIssueId`, `ReleaseTooLarge`, `MilestoneNotCancellable` |
| 40 | `ContributorNotSet` |
| 50–53 | `InvalidDomain`, `EmptyRecipient`, `ZeroBurnAmount`, `InvalidCctpRecipientPadding` |
| 60 | `ZeroPageLimit` |

## Deployment and integration

The backend integration uses these values conceptually:

```bash
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443
SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
CONTRACT_ID=<deployed_contract_id>
PLATFORM_SECRET_KEY=<keep-secret>
PLATFORM_PUBLIC_KEY=<platform_address>
USDC_TOKEN_ADDRESS=<testnet_usdc_sac>
```

The contract does not load environment variables itself. These values belong to the caller or deployment environment and must never be committed.

> **Breaking storage change — fresh deployment required.** This version removed the on-chain `Milestone.title` field, which changes the storage encoding of milestone records. Records written by an older binary cannot be decoded by this version, and the contract provides no in-place migration. Deploy a fresh contract instance with this wasm: it starts with empty storage, so the escrow state and all existing milestone data are reset.

## Known limitations

1. `release_funds` relies on the platform wallet following maintainer instructions; a future version could require a direct maintainer authorization.
2. There is no milestone timeout or expiry-based cancellation.
3. A single platform wallet is trusted for all releases in this contract instance.
4. There is no neutral dispute-arbitration role; the maintainer controls milestone setup and cancellation.
5. Initialization is protected by the stored admin and the single-escrow guard.

See [the architecture guide](arch.md) for diagrams and module boundaries.
