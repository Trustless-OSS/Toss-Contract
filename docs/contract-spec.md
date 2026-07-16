# Contract specification

This is the detailed reference for the current `trustless-oss` contract. Source code is authoritative if this document and the implementation ever differ.

## Entry points

The contract exposes the following public methods:

| Method | Authorization | Purpose |
| --- | --- | --- |
| `initialize(repo_id, maintainer, platform, token)` | First call: maintainer; later calls: stored admin | Creates the single escrow and stores the admin. |
| `deposit_funds(amount)` | Maintainer | Transfers USDC from the maintainer into the contract. |
| `withdraw_funds(amount)` | Maintainer | Returns only available USDC to the maintainer. |
| `create_milestone(issue_id, title, reward)` | Maintainer | Creates a pending milestone and reserves its reward. |
| `assign_contributor(issue_id, contributor)` | Maintainer | Sets the payout target and moves a pending milestone to active. |
| `reassign_contributor(issue_id, contributor)` | Maintainer | Changes the payout target of an active milestone. |
| `release_funds(issue_id)` | Platform | Pays the full reward and marks the milestone released. |
| `partial_release(issue_id, release_amount)` | Platform | Pays part of the reward and returns the remainder to the available pool. |
| `cancel_milestone(issue_id)` | Maintainer | Cancels a pending or active milestone and un-reserves its reward. |
| `get_escrow()` | None | Reads the escrow state. |
| `get_milestone(issue_id)` | None | Reads one milestone. |
| `get_balance()` | None | Reads deposited, reserved, released, and available amounts. |
| `list_milestones()` | None | Returns all indexed milestones. |

Every state-changing method requires an active escrow. Amounts are integer token base units; for a 7-decimal USDC token, `10_000_000` base units equals 1 USDC.

## Data model

### `PayoutTarget`

```text
payout_type = 0: stellar_address must be set
payout_type = 1: destination_domain and non-zero recipient are used for CCTP
payout_type = 2: unset contributor; releases are rejected
```

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

Milestone creation must fit within `available`, withdrawals cannot exceed it, and releasing or cancelling a milestone reduces `reserved`. The tests exercise funding, reservation, withdrawal, release, partial release, and CCTP paths.

## Storage and events

Persistent storage keys are:

| Key | Value |
| --- | --- |
| `Escrow` | `EscrowState` |
| `Milestone(issue_id)` | `Milestone` |
| `EscrowIssueIds` | `Vec<u64>` index used for listing |
| `Admin` | Stored initializer/admin address |

State-changing methods emit typed events for initialization, deposits, withdrawals, milestone creation, contributor assignment/reassignment, releases, partial releases, and cancellation. The event topic payloads are defined in `trustless-oss/src/events.rs`.

## Error groups

| Codes | Errors |
| --- | --- |
| 1–3 | `NotAdmin`, `NotPlatform`, `NotMaintainer` |
| 10–12 | `EscrowNotFound`, `EscrowAlreadyExists`, `EscrowInactive` |
| 20–22 | `InsufficientBalance`, `WithdrawExceedsAvailable`, `ZeroAmount` |
| 30–34 | `MilestoneNotFound`, `MilestoneNotPending`, `MilestoneNotActive`, `DuplicateIssueId`, `ReleaseTooLarge` |
| 40 | `ContributorNotSet` |
| 50–52 | `InvalidCctpDomain`, `InvalidCctpRecipient`, `CctpAmountPrecisionLoss` |

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

## Known limitations

1. `partial_release` relies on the platform wallet following maintainer instructions; a future version could require a direct maintainer authorization.
2. There is no milestone timeout or expiry-based cancellation.
3. A single platform wallet is trusted for all releases in this contract instance.
4. There is no neutral dispute-arbitration role; the maintainer controls milestone setup and cancellation.
5. Initialization is protected by the stored admin and the single-escrow guard.

See [the architecture guide](arch.md) for diagrams and module boundaries.
