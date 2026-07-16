# Trustless-OSS Contract Architecture

This document describes the current implementation in `trustless-oss/src`. The contract stores one repository escrow per deployed contract instance. A backend can deploy separate instances when it needs separate isolation boundaries.

## System context

```mermaid
flowchart TB
    Maintainer[Maintainer]
    GitHub[GitHub issues and pull requests]
    Backend[Trustless-OSS backend]
    Soroban[Soroban contract]
    USDC[USDC Stellar Asset Contract]
    Platform[Platform wallet]
    Stellar[Stellar contributor address]
    CCTP[CCTP Token Messenger / Minter]
    Chain[Destination chain recipient]

    GitHub -->|webhooks| Backend
    Backend -->|initialize, milestones, queries| Soroban
    Maintainer -->|authorize funding and milestone changes| Soroban
    Soroban -->|transfer and approve| USDC
    Backend -->|confirm completion| Platform
    Platform -->|authorize release| Soroban
    Soroban -->|direct transfer| Stellar
    Soroban -->|deposit_for_burn| CCTP
    CCTP -->|mint on destination chain| Chain

    classDef human fill:#fff3bf,stroke:#f08c46,color:#5f370e,stroke-width:2px;
    classDef external fill:#ffe3e3,stroke:#c92a2a,color:#641414,stroke-width:2px;
    classDef app fill:#d0ebff,stroke:#1971c2,color:#0b2e4f,stroke-width:2px;
    classDef contract fill:#d3f9d8,stroke:#2b8a3e,color:#123b1a,stroke-width:2px;
    classDef asset fill:#e5dbff,stroke:#7048e8,color:#30156e,stroke-width:2px;
    class Maintainer,Platform human;
    class GitHub,USDC,CCTP,Chain external;
    class Backend app;
    class Soroban contract;
    class Stellar asset;
```

## Module responsibilities

| Module | Responsibility |
| --- | --- |
| `lib.rs` | Contract entry points, escrow/milestone transitions, token transfers, and CCTP payout dispatch. |
| `types.rs` | `EscrowState`, `Milestone`, `MilestoneStatus`, `PayoutTarget`, and `BalanceInfo` contract types. |
| `storage.rs` | Persistent keys, reads/writes, issue-ID indexing, and storage TTL extension. |
| `auth.rs` | Soroban authorization checks for the maintainer, platform, and active escrow state. |
| `events.rs` | Typed event topics emitted after state-changing operations. |
| `error.rs` | Stable numeric `ContractError` values returned by entry points. |
| `test.rs` | In-memory Soroban environment, token mocks, authorization tests, and CCTP payout tests. |

## State and storage

```mermaid
flowchart LR
    Init[initialize] --> Escrow[(Escrow)]
    Init --> IDs[(EscrowIssueIds)]
    Init --> Admin[(Admin)]
    Create[create_milestone] --> Escrow
    Create --> Milestone[(Milestone issue_id)]
    Create --> IDs
    Assign[assign / reassign contributor] --> Milestone
    Cancel[cancel_milestone] --> Escrow
    Cancel --> Milestone
    Release[release / partial_release] --> Escrow
    Release --> Milestone
    Queries[get_escrow / get_balance / list_milestones] -. read .-> Escrow
    Queries -. read .-> Milestone

    classDef entry fill:#d0ebff,stroke:#1971c2,color:#0b2e4f,stroke-width:2px;
    classDef store fill:#d3f9d8,stroke:#2b8a3e,color:#123b1a,stroke-width:2px;
    classDef query fill:#e5dbff,stroke:#7048e8,color:#30156e,stroke-width:2px;
    class Init,Create,Assign,Cancel,Release entry;
    class Escrow,IDs,Admin,Milestone store;
    class Queries query;
```

All application keys are stored in Soroban persistent storage and writes extend their TTL to the configured `100_000` minimum and maximum values. `EscrowIssueIds` is the index used by `list_milestones`; each milestone is stored separately under its issue ID.

The deployed contract has one `EscrowState`, not a map of escrow IDs. Its state tracks:

- `total_deposited`: cumulative funds added to the contract balance.
- `reserved`: rewards belonging to pending or active milestones.
- `total_released`: cumulative amount paid to contributors.
- `available`: derived as `total_deposited - reserved - total_released`.

## Milestone lifecycle

```mermaid
stateDiagram-v2
    [*] --> Pending: create_milestone
    Pending --> Active: assign_contributor
    Active --> Active: reassign_contributor
    Active --> Released: release_funds
    Active --> Released: partial_release
    Pending --> Cancelled: cancel_milestone
    Active --> Cancelled: cancel_milestone
    Released --> [*]
    Cancelled --> [*]

    classDef pending fill:#fff3bf,stroke:#f08c46,color:#5f370e,stroke-width:2px;
    classDef active fill:#d0ebff,stroke:#1971c2,color:#0b2e4f,stroke-width:2px;
    classDef released fill:#d3f9d8,stroke:#2b8a3e,color:#123b1a,stroke-width:2px;
    classDef cancelled fill:#ffe3e3,stroke:#c92a2a,color:#641414,stroke-width:2px;
    class Pending pending;
    class Active active;
    class Released released;
    class Cancelled cancelled;
```

Creating a milestone reserves its full reward. Assignment changes the payout target and status but does not change the reserved amount. A full release pays the full reward; a partial release pays the requested amount and makes the remainder available again. Cancellation un-reserves the full reward without transferring funds.

## Payout sequence

```mermaid
sequenceDiagram
    participant W as Platform wallet
    participant C as Soroban contract
    participant T as USDC SAC
    participant S as Stellar contributor
    participant X as CCTP minter

    W->>C: release_funds(issue_id)
    C->>C: require platform auth
    C->>C: validate Active milestone
    alt Stellar payout (payout_type = 0)
        C->>T: transfer(contract, contributor, amount)
        T-->>S: credit USDC
    else CCTP payout (payout_type = 1)
        C->>T: approve(contract, CCTP minter, amount)
        C->>X: deposit_for_burn(amount, domain, recipient, token)
        X-->>X: burn and emit cross-chain message
    end
    C->>C: persist state and emit event
```

`PayoutTarget.payout_type` is `0` for a Stellar address, `1` for CCTP, and `2` for an unset contributor. CCTP releases validate the destination domain, reject a zero recipient, and require an amount divisible by 10 to avoid precision loss.

## Authorization boundaries

| Operation | Required authorization |
| --- | --- |
| First `initialize` | The maintainer authorizes and becomes the stored admin. |
| Later `initialize` calls | The stored admin authorizes; the existing escrow still prevents reinitialization. |
| `deposit_funds`, `withdraw_funds` | Maintainer. |
| `create_milestone`, `assign_contributor`, `reassign_contributor`, `cancel_milestone` | Maintainer. |
| `release_funds`, `partial_release` | Platform wallet. |
| Query entry points | No explicit caller authorization. |

## Related documentation

- [Repository README](../README.md) — installation, build, test, deployment, and contribution workflow.
- [Contract specification](contract-spec.md) — complete entry-point reference, error codes, events, and known limitations.
- [Contributing guide](contributing.md) — branch names, commit messages, and pull request expectations.
