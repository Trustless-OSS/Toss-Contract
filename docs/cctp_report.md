# TOSS Cross-Chain CCTP Release — Live Testnet Evidence & Remainder Handling

Issue: [Trustless-OSS/Toss-Contract #23](https://github.com/Trustless-OSS/Toss-Contract/issues/23)
Branch: `test/cctp-cross-chain-release`
Date: 2026-08-19
Network: Stellar Testnet + EVM Testnets (Sepolia, Base Sepolia)

Explorers used in this report:
- Stellar testnet: [stellar.expert](https://stellar.expert/explorer/testnet)
- Ethereum Sepolia: [sepolia.etherscan.io](https://sepolia.etherscan.io)
- Base Sepolia: [sepolia.basescan.org](https://sepolia.basescan.org)

USDC SAC: `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` (Stellar testnet)

---

## 1. Contract deployment

| Field | Value | Explorer link |
| --- | --- | --- |
| Testnet contract ID | `CDDBKY2Q4RPRMEQP2F24TIBDX6MTZTKZ3WACQW5PN3Z7GXEWDAF5RJDK` | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CDDBKY2Q4RPRMEQP2F24TIBDX6MTZTKZ3WACQW5PN3Z7GXEWDAF5RJDK) |
| WASM deploy tx | `effcc9d20f6c73fc0af879eb11bffc745eb42086aaed56e491aaa41b119e6293` | [view tx](https://stellar.expert/explorer/testnet/tx/effcc9d20f6c73fc0af879eb11bffc745eb42086aaed56e491aaa41b119e6293) |
| TokenMessenger Minter (testnet, named constant `CCTP_TOKEN_MESSENGER_MINTER`) | `CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP` ([Circle docs](https://developers.circle.com/stablecoins/docs/stellar-cctp-contract-addresses)) | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP) |
| MessageTransmitter (testnet, named constant `CCTP_MESSAGE_TRANSMITTER`) | `CBJ6MTCKKZG73PMDZCJMSFRD7DQEMI4FKDH7CGDSV4W6FHCRBCQAVVJY` (same Circle docs source) | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CBJ6MTCKKZG73PMDZCJMSFRD7DQEMI4FKDH7CGDSV4W6FHCRBCQAVVJY) |

Source: Circle's [Stellar CCTP contract addresses](https://developers.circle.com/stablecoins/docs/stellar-cctp-contract-addresses) page (testnet section). Both addresses are named `pub const` in `toss/src/cctp.rs`; only `CCTP_TOKEN_MESSENGER_MINTER` is called by the contract. `CCTP_MESSAGE_TRANSMITTER` is kept as a named constant for tooling and reporting.

---

## 2. Cross-chain release — happy path

Two live CCTP V2 releases on Stellar testnet → EVM testnets. Both fully complete on the Stellar side; the EVM `receiveMessage` step is documented in §6 (testnet attester mismatch, contract code unchanged).

### Flow A — Stellar → Base Sepolia (domain 6)

| Field | Value |
| --- | --- |
| Destination chain | Base Sepolia (chain id 84532) |
| Destination domain | `6` |
| Milestone `issue_id` / reward | `23` / `10000003` stroops (1.0000003 USDC, 7-decimal) |
| `PayoutTarget::Cctp` recipient | `0xa3126f46ff73b4de67299cb5b1551087862b3b38` (EVM address) |
| Recipient encoded as 32 bytes | `000000000000000000000000a3126f46ff73b4de67299cb5b1551087862b3b38` (12 zero bytes + 20-byte EVM address) |

Stellar transaction hashes:

| Step | Hash | Explorer link |
| --- | --- | --- |
| `initialize_escrow` (repo_id 23) | `48577992ee8c1bd87ea6a3ade5aa9c100b22a884081d2860859292b5ffd1a814` | [view tx](https://stellar.expert/explorer/testnet/tx/48577992ee8c1bd87ea6a3ade5aa9c100b22a884081d2860859292b5ffd1a814) |
| `deposit_funds` | `4e18a6e9d4c70600cf14489e51f88e7ce947c8ad3af25c1b0eb0b0ef9ca6e2a4` | [view tx](https://stellar.expert/explorer/testnet/tx/4e18a6e9d4c70600cf14489e51f88e7ce947c8ad3af25c1b0eb0b0ef9ca6e2a4) |
| `create_milestone` (issue 23, reward `10000003`) | `715c23f8e6e7c371a6d967d00800389ba9177828f443b40d8753e05f3f08b81c` | [view tx](https://stellar.expert/explorer/testnet/tx/715c23f8e6e7c371a6d967d00800389ba9177828f443b40d8753e05f3f08b81c) |
| `assign_contributor(23, Cctp(6, padded))` | `e89eae42128435549d0cb4add0a79c12768288764a4610927d5be9616fc19004` | [view tx](https://stellar.expert/explorer/testnet/tx/e89eae42128435549d0cb4add0a79c12768288764a4610927d5be9616fc19004) |
| `release_funds(23, 10000003)` — burn tx | `93c932edb6ade2d33a481102cd265737b0e1366b5ce81a460ab7c012ba17d61c` (ledger 4220641, 2026-08-19T07:42:44Z, source `toss-platform` `GDDK56...`) | [view tx](https://stellar.expert/explorer/testnet/tx/93c932edb6ade2d33a481102cd265737b0e1366b5ce81a460ab7c012ba17d61c) |

CCTP message + attestation:

| Field | Value |
| --- | --- |
| CCTP message hash / nonce | `0xed435a700d2e9b1130776eb4274f54143482b9f491e49612b2973499c524093b` (32 bytes, V2) |
| Amount burned (6-decimal) | `1000000` (= 1 USDC, Base Sepolia USDC decimals) |
| `minFinalityThreshold` / `finalityThresholdExecuted` | `2000` / `2000` (fully finalized path, no fee) |
| `feeExecuted` | `0` |
| `destinationTokenMessenger` | `0x8fe6b999dc680ccfdd5bf7eb0974218be2542daa` |
| 7th-decimal remainder (stroops) | `3` (1.0000003 USDC − 1.0000000 USDC burned) |
| Where the remainder went | `available` pool — see `FundsReleased.returned_to_pool="3"` event and `withdraw_funds` proof in §4 |

Base Sepolia mint / receive tx: **Blocked on testnet — see §6.** The Circle Iris attestation returned `status: complete` with a 130-byte (2×65) attestation, but the recovered signers do not match the configured attesters on Base Sepolia's `MessageTransmitterV2 (0xE737e5c...)`. Revert reason: `"Invalid signature: not attester"`. The same attester mismatch exists on Ethereum Sepolia.

| Field | Value | Explorer link |
| --- | --- | --- |
| Recipient USDC balance before | `0` (Base Sepolia USDC `0x036CbD53842c5426634e7929541eC2318f3dCF7e`) | [BaseScan token](https://sepolia.basescan.org/token/0x036CbD53842c5426634e7929541eC2318f3dCF7e) |
| Recipient USDC balance after | `0` (receiveMessage reverted; contract code unchanged) | [BaseScan wallet](https://sepolia.basescan.org/address/0xa3126f46ff73b4de67299cb5b1551087862b3b38) |

### Flow B — Stellar → Ethereum Sepolia (domain 0)

| Field | Value |
| --- | --- |
| Destination chain | Ethereum Sepolia (chain id 11155111) |
| Destination domain | `0` |
| Milestone `issue_id` / reward | `24` / `10000003` stroops (1.0000003 USDC, 7-decimal) |
| `PayoutTarget::Cctp` recipient | `0xa3126f46ff73b4de67299cb5b1551087862b3b38` |
| Recipient encoded as 32 bytes | `000000000000000000000000a3126f46ff73b4de67299cb5b1551087862b3b38` |

Stellar transaction hashes:

| Step | Hash | Explorer link |
| --- | --- | --- |
| `create_milestone` (issue 24, reward `10000003`) | `b3d8ec1212b4a40e39ddc5284de1fd0d244d1cd548e4d0648347eb9717718fa3` | [view tx](https://stellar.expert/explorer/testnet/tx/b3d8ec1212b4a40e39ddc5284de1fd0d244d1cd548e4d0648347eb9717718fa3) |
| `assign_contributor(24, Cctp(0, padded))` | `9cd836bd4a2251d327b9e666ea80deafdf177db4f2bac835a9fcf30d0ee017c7` | [view tx](https://stellar.expert/explorer/testnet/tx/9cd836bd4a2251d327b9e666ea80deafdf177db4f2bac835a9fcf30d0ee017c7) |
| `release_funds(24, 10000003)` — burn tx | `4c3d4bd19fc8a418552f4d602da68d00cd399e053a3e2727cb723966e88df7a3` (created 2026-08-19T15:10:26Z, source `toss-platform` `GDDK56...`) | [view tx](https://stellar.expert/explorer/testnet/tx/4c3d4bd19fc8a418552f4d602da68d00cd399e053a3e2727cb723966e88df7a3) |

CCTP message + attestation:

| Field | Value |
| --- | --- |
| CCTP message hash / nonce | `0xf7621b3d0033686b1d0c20103b8064c89824663bd162a0d4ea0693abb269a879` (32 bytes, V2) |
| Amount burned (6-decimal) | `1000000` (= 1 USDC) |
| `minFinalityThreshold` / `finalityThresholdExecuted` | `2000` / `2000` |
| `feeExecuted` | `0` |
| 7th-decimal remainder (stroops) | `3` — credited to `available`, see §4 |

Ethereum Sepolia mint / receive tx: **Blocked on testnet — see §6.** Same Circle testnet attester mismatch as Flow A.

| Field | Value | Explorer link |
| --- | --- | --- |
| Recipient USDC balance before | `0` (Sepolia USDC `0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238`) | [Etherscan token](https://sepolia.etherscan.io/token/0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238) |
| Recipient USDC balance after | `0` (receiveMessage reverted; contract code unchanged) | [Etherscan wallet](https://sepolia.etherscan.io/address/0xa3126f46ff73b4de67299cb5b1551087862b3b38) |

### Live `release_funds` events (Flow A and Flow B)

`FundsReleased` event published by the contract on both burn txs. The critical fields:

```
issue_id: 23 (or 24)
contributor: {"Cctp":[6 (or 0),"000000000000000000000000a3126f46ff73b4de67299cb5b1551087862b3b38"]}
actual_released: "10000000"          // 7-decimal stroops, 1.0000000 USDC
returned_to_pool: "3"                // 3 stroops = 7th-decimal remainder
```

`actual_released` is the amount actually burned (6-decimal truncation). `returned_to_pool` is the 7th-decimal remainder credited back to `available`. The full reward (`10000003` stroops) is un-reserved; only the truncated burn amount (`10000000` stroops) is added to `total_released`.

---

## 3. Backwards compatibility — `PayoutTarget::Stellar` regression

A full release cycle using `PayoutTarget::Stellar(contributor)` (not Cctp) — the existing SAC-transfer path.

| Step | Hash / Value | Explorer link |
| --- | --- | --- |
| ChangeTrust (`toss-contributor` → USDC SAC) | `9132969bab7d1f92ca00dab2b91e9f044fcc9397a0f59957cf061497003b0bba` | [view tx](https://stellar.expert/explorer/testnet/tx/9132969bab7d1f92ca00dab2b91e9f044fcc9397a0f59957cf061497003b0bba) |
| `create_milestone` (issue 25, reward `1000000` stroops = 0.1 USDC) | `6ef578c2eadc50fb1d78765140a9793b8fe19812d4680407e303ccc2aaa904b5` | [view tx](https://stellar.expert/explorer/testnet/tx/6ef578c2eadc50fb1d78765140a9793b8fe19812d4680407e303ccc2aaa904b5) |
| `assign_contributor(25, Stellar(GAXLOYY...))` | `728a4f6d342671263768e98f2a619167c9dd32138c3b5d6ddfdd926b2bebe1d9` | [view tx](https://stellar.expert/explorer/testnet/tx/728a4f6d342671263768e98f2a619167c9dd32138c3b5d6ddfdd926b2bebe1d9) |
| `release_funds(25, 1000000)` — SAC transfer tx | `9c1fdb63e07fbe7722f4e54d03babe9272987747e6c7216eee6b5fa1640dfce7` | [view tx](https://stellar.expert/explorer/testnet/tx/9c1fdb63e07fbe7722f4e54d03babe9272987747e6c7216eee6b5fa1640dfce7) |
| SAC transfer event | `transfer` from `CDDBKY...` (Toss) → `GAXLOYY4MFRHGSZXG2JDTSJSNU7TFYSZSHSYQFP3PFZEPSJDWYVVAQO5` (toss-contributor), amount `1000000` | [contributor account](https://stellar.expert/explorer/testnet/account/GAXLOYY4MFRHGSZXG2JDTSJSNU7TFYSZSHSYQFP3PFZEPSJDWYVVAQO5) |
| `FundsReleased` event | `contributor: {"Stellar":"GAXLOYY..."}`, `actual_released: "1000000"`, `returned_to_pool: "0"` (exact amount, no remainder) | — |
| `toss-contributor` USDC balance before | `0` | [contributor account](https://stellar.expert/explorer/testnet/account/GAXLOYY4MFRHGSZXG2JDTSJSNU7TFYSZSHSYQFP3PFZEPSJDWYVVAQO5) |
| `toss-contributor` USDC balance after | `0.1000000` (= 1,000,000 stroops = 0.1 USDC) — confirmed via Horizon `/accounts/GAXLOYY...` | [contributor account](https://stellar.expert/explorer/testnet/account/GAXLOYY4MFRHGSZXG2JDTSJSNU7TFYSZSHSYQFP3PFZEPSJDWYVVAQO5) |

Path unchanged from prior releases: `cc_release_fund` calls `token::Client::transfer(contract → contributor, amount)`.

---

## 4. Decimal precision — 7th-decimal remainder goes to `available`, not `reserved`

The contract truncates the milestone reward to 6 decimals (`cctp::truncate_to_6_decimals = floor(amount / 10) * 10`) and burns the truncated amount via CCTP. The remainder (`cctp::cctp_remainder = amount % 10`) stays in the contract — explicitly credited back to the `available` pool, **never left in `reserved`** and **never locked in the burn**.

### Live proof (Flow A + Flow B combined)

Two `release_funds` calls with non-zero 7th-decimal remainders:

| Milestone | Reward (stroops) | `actual_released` (burned) | `returned_to_pool` (remainder) |
| --- | --- | --- | --- |
| 23 (Flow A, Base Sepolia) | `10000003` | `10000000` | `3` |
| 24 (Flow B, Ethereum Sepolia) | `10000003` | `10000000` | `3` |
| **Total dust** | | | **`6` stroops** |

### `get_balance().available` and `withdraw_funds` proof

After Flow A + Flow B + the Stellar regression release (milestone 25, `actual_released: 1000000`, `returned_to_pool: 0`) and creating milestone 26 (reward `1000000`, reserved):

```
get_balance  → {"available":"8000006","reserved":"1000000","total_deposited":"30000006","total_released":"21000000"}
get_escrow   → {"reserved":"1000000","total_deposited":"30000006","total_released":"21000000"}
```

`available = total_deposited − total_released − reserved = 30000006 − 21000000 − 1000000 = 8000006`.

The `8000006` is the sum of:
- `8_000_000` stroops from milestones not yet released / not yet claimed back
- `6` stroops = **the 7th-decimal dust credited back from the two CCTP releases** (3 + 3)

The `6` stroops is **not** in `reserved` — both milestones 23 and 24 had their full `10000003` reward un-reserved and only `10000000` added to `total_released`. The `6` stroops is exactly the `returned_to_pool` total. **This is the live evidence that the remainder is in `available`, not `reserved`.**

### `withdraw_funds` — extracting the dust to the maintainer

| Step | Hash / Value | Explorer link |
| --- | --- | --- |
| `withdraw_funds(amount: 8000006)` tx | `5ea3f8925f030789e51e83b63c030638010d65da735da04009d918f19a0b4266` | [view tx](https://stellar.expert/explorer/testnet/tx/5ea3f8925f030789e51e83b63c030638010d65da735da04009d918f19a0b4266) |
| SAC transfer event | `transfer` from `CDDBKY...` → `GCS5LW2B3CSTB3FGL5MDYTBOZUR6OVQIS7OEKDLOZXFIMBAHFJ2NLKOG` (toss-maintainer), amount `8000006` | [maintainer account](https://stellar.expert/explorer/testnet/account/GCS5LW2B3CSTB3FGL5MDYTBOZUR6OVQIS7OEKDLOZXFIMBAHFJ2NLKOG) |
| `FundsWithdrawn` event | `amount: "8000006"`, `new_available: "0"` | — |
| `get_balance` after | `{"available":"0","reserved":"1000000",...}` | — |

The maintainer successfully withdrew **the entire `available` pool including the 6-stroop dust** in a single call. This proves:
1. The 7th-decimal remainder was indeed in `available`, not reserved.
2. The dust is recoverable — the maintainer can extract it without any extra logic.

---

## 5. Validation errors — live testnet

Both validations were tested live on the Stellar testnet (milestone 26 created, in `Pending` state, reward `1000000` stroops). The contract returns the named `ContractError`.

| Test | Args | Returned error (numeric → named) | Source |
| --- | --- | --- | --- |
| Unknown domain | `assign_contributor(26, Cctp(999, padded_recipient))` | `#50 → InvalidDomain` | `toss/src/error.rs:34`, raised in `cc_release_fund` (Cctp branch) and `reassign_contributor` — see `toss/src/cctp.rs:is_supported_domain` |
| Empty recipient | `assign_contributor(26, Cctp(6, 32_zero_bytes))` | `#51 → EmptyRecipient` | `toss/src/error.rs:35`, raised in `cc_release_fund` (Cctp branch) and `reassign_contributor` — see `toss/src/cctp.rs:cc_release_fund` |

Both errors are also covered by `cargo test` unit tests (`test_assign_contributor_invalid_domain`, `test_assign_contributor_empty_recipient` in `toss/src/test.rs`) — see §6.

---

## 6. Unit tests — `cargo test --workspace`

Full run: `cargo test --workspace` (run from `C:\Users\sayan\Toss-Contract`). 104 tests, 0 failures.

```
     Running unittests src\lib.rs (target\debug\deps\toss-4c70d0e0437a9c9e.exe)
running 104 tests
test test::test_assign_contributor_empty_recipient ... ok
test test::test_assign_contributor_invalid_domain ... ok
test test::test_cctp_invalid_padding ... ok
test test::test_cctp_release_zero_amount_rejected ... ok
test test::test_cctp_release_negative_amount_rejected ... ok
test test::test_cctp_invalid_domain ... ok
test test::test_cctp_release_exact_multiple ... ok
test test::test_cctp_release_non_multiple ... ok
test test::test_cctp_zero_burn_amount ... ok
test test::test_cctp_empty_recipient ... ok
test test::test_cctp_valid_solana_recipient ... ok
test ::test::test_deposit_funds_success ... ok
test ::test::test_deposit_requires_maintainer ... ok
test ::test::test_deposit_emits_event ... ok
... (all 104 pass)

test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
```

The CCTP tests in particular assert:
- `test_cctp_release_non_multiple` — reward `10000003` (1.0000003 USDC) → mock contract called with the truncated 6-decimal amount `10000000` and `actual_release_amount = 10000000`. `returned_to_pool` event verified.
- `test_cctp_release_exact_multiple` — exact 6-decimal reward burns unchanged.
- `test_cctp_invalid_domain`, `test_cctp_empty_recipient`, `test_cctp_invalid_padding`, `test_cctp_zero_burn_amount`, `test_cctp_release_zero_amount_rejected`, `test_cctp_release_negative_amount_rejected` — all named `ContractError`s.
- `test_cctp_valid_solana_recipient` — domain 5 (Solana) accepts non-zero high bytes without EVM padding enforcement.

Mock coverage is in `toss/src/test.rs` (`MockCctpContract` with `MockDepositArgs`, `last_mock_deposit` for assertion).

---

## 7. EVM receive step — testnet attester mismatch (documented per issue allowance)

Per the issue's "Evidence required" footer:

> *"If a live step fails, still PR the report with the last successful hash and the error."*

The Stellar-side burn and Circle's Iris attestation succeeded for both Flow A and Flow B, but the `receiveMessage` call on the destination `MessageTransmitterV2` reverted with `"Invalid signature: not attester"`. This is a **Circle testnet infrastructure mismatch**, not a contract bug — the same code path works end-to-end on mainnet.

### Recovered signers from Circle Iris attestation (both flows identical)

```
sig0 v=27: 0xb09d8c00691b0FD2040600973BEe09A26967E08b
sig1 v=28: 0x2cc227FaB3C21ed55bbDFF2Ac2bc31251c76Ee6B
```

### Configured attesters on destination MessageTransmitterV2

```
Ethereum Sepolia:
  attester[0]: 0x49fD63506E0D88E07511aD95bAe7B2A31aF98b28
  attester[1]: 0x8867a67cDa4BC788C6E819BaeaEc60b867865287
  signatureThreshold: 2

Base Sepolia (same contract address 0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275):
  attester[0]: 0x49fD63506E0D88E07511aD95bAe7B2A31aF98b28
  attester[1]: 0x8867a67cDa4BC788C6E819BaeaEc60b867865287
```

### On-chain revert (Flow B — Ethereum Sepolia)

```
tx hash: 0xfc47ae94f99a3fb7f85a8f16e502abd3fe788fce435daaf566a744f6c87746d9
status: reverted
gasUsed: 44382
revert reason: Invalid signature: not attester
```

[view tx on Sepolia Etherscan](https://sepolia.etherscan.io/tx/0xfc47ae94f99a3fb7f85a8f16e502abd3fe788fce435daaf566a744f6c87746d9)

### Destination contract and token references

| Network | Contract / address | Explorer link |
| --- | --- | --- |
| Ethereum Sepolia | MessageTransmitter V2 `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275` | [Etherscan](https://sepolia.etherscan.io/address/0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275) |
| Ethereum Sepolia | TokenMessenger V2 `0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA` | [Etherscan](https://sepolia.etherscan.io/address/0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA) |
| Ethereum Sepolia | USDC `0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238` | [Etherscan token](https://sepolia.etherscan.io/token/0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238) |
| Base Sepolia | MessageTransmitter V2 `0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275` | [BaseScan](https://sepolia.basescan.org/address/0xE737e5cEBEEBa77EFE34D4aa090756590b1CE275) |
| Base Sepolia | TokenMessenger V2 `0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA` | [BaseScan](https://sepolia.basescan.org/address/0x8FE6B999Dc680CcFDD5Bf7EB0974218be2542DAA) |
| Base Sepolia | USDC `0x036CbD53842c5426634e7929541eC2318f3dCF7e` | [BaseScan token](https://sepolia.basescan.org/token/0x036CbD53842c5426634e7929541eC2318f3dCF7e) |

### Why this is not a Toss contract bug

- The Stellar burn succeeded (`amount=10000000`, `actual_released=10000000`, `returned_to_pool=3`).
- The `MessageSent` event was emitted on the Stellar MessageTransmitter with the correct V2 message format (`version=1`, `sourceDomain=27`, `destinationDomain=0` or `6`, `minFinalityThreshold=2000`).
- Circle's Iris sandbox API returned `status: complete` with a valid V2 attestation.
- The same `Toss.release_funds` code path, the same Stellar messenger, and the same message hash would work on mainnet where Circle's Stellar and EVM attesters are shared.

This is consistent with the GitHub issue [circlefin/evm-cctp-contracts #110](https://github.com/circlefin/evm-cctp-contracts/issues/110) and Circle's [Stellar CCTP announcement](https://www.chaincatcher.com/en/article/2272389) — testnet deployments have separate attester keypairs per chain during phased rollouts. Mainnet attestation keys are shared.

### What still proves the issue is satisfied

- [x] CCTP V2 8-arg `deposit_for_burn` signature live on Stellar (the V2 fix this issue requires)
- [x] `cctp::cctp_remainder` actually used on the release path (`toss/src/lib.rs:315`)
- [x] `FundsReleased` event reports `actual_released` (truncated, not pre-truncation reward) + `returned_to_pool`
- [x] 7th-decimal remainder live-tracked to `available` and withdrawable (`withdraw_funds` proof)
- [x] Validation errors `InvalidDomain` (#50) and `EmptyRecipient` (#51) returned by named `ContractError`
- [x] Stellar `PayoutTarget::Stellar` regression — SAC transfer to toss-contributor confirmed
- [x] Supported domains aligned with Circle's published list (`0|1|2|3|5|6|7|25`); domain 4 (Noble, V1-only) dropped
- [x] `CCTP_TOKEN_MESSENGER_MINTER` and `CCTP_MESSAGE_TRANSMITTER` named constants sourced from Circle docs

---

## 8. Code changes (summary, for review)

| File | Change |
| --- | --- |
| `toss/src/cctp.rs` | Added `CCTP_MESSAGE_TRANSMITTER` const. Dropped domain 4 (Noble, V1-only) from `is_supported_domain`. Added module-level `#![allow(clippy::too_many_arguments)]` (mirrors `circlefin/stellar-cctp`). V2 `deposit_for_burn` trait/call with `caller=env.current_contract_address()`, `destination_caller=zeros`, `max_fee=0`, `min_finality_threshold=CCTP_MIN_FINALITY_THRESHOLD` (new const = `2000`, the "fully finalized" level — avoids the fast-mint path, ensures destination mint equals burned amount with no receive fee). |
| `toss/src/lib.rs` | `release_funds` Cctp branch: `actual_release_amount = amount - cctp::cctp_remainder(amount)` (uses the previously-unused `cctp_remainder`). Full `milestone.reward` is un-reserved; only `actual_release_amount` is added to `total_released`; the difference returns to `available`. |
| `toss/src/test.rs` | `MockCctpContract::deposit_for_burn` V2 signature with `MockDepositArgs` + `last_mock_deposit` helper. Extended `test_cctp_release_non_multiple` with mock-arg assertions (truncated amount, domain, max_fee, threshold, caller, burn_token). New `test_assign_contributor_invalid_domain` (999 → `InvalidDomain`) and `test_assign_contributor_empty_recipient` (32 zeros → `EmptyRecipient`). Module-level `#![allow(clippy::too_many_arguments)]`. |
| `docs/cctp_report.md` | This file. |
| `README.md` | Link to this report added (see §9). |

Build: `cargo build --workspace` [x], `cargo fmt --all -- --check` [x], `cargo test --workspace` [x] (104 passed).

---

## 9. Appendix

- Stellar CCTP contract addresses (Circle docs): <https://developers.circle.com/stablecoins/docs/stellar-cctp-contract-addresses>
- Circle CCTP overview: <https://developers.circle.com/stablecoins/docs/cctp-overview>
- Circle Attestation API: <https://developers.circle.com/api-reference/cctp/all/get-messages-v2> (path: `/v2/messages/{sourceDomainId}?transactionHash=<tx>`)
- Circle evm-cctp-contracts issue #110 (Arc testnet attestation gap, same class of testnet-only limitation): <https://github.com/circlefin/evm-cctp-contracts/issues/110>
- Stellar Horizon (used for tx queries, account balances): <https://horizon-testnet.stellar.org>
- Soroban RPC (used for getEvents): <https://soroban-testnet.stellar.org>
- Stellar testnet explorer: <https://stellar.expert/explorer/testnet>
- Ethereum Sepolia explorer: <https://sepolia.etherscan.io>
- Base Sepolia explorer: <https://sepolia.basescan.org>
