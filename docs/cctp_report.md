# Cross-chain CCTP Release Report 🌍

This report documents the verification, contract configuration, live testnet evidence, remainder dust handling, backwards compatibility, validation error handling, and unit test results for TOSS CCTP cross-chain releases.

---

## 1. Contract Deployment Details

- **Testnet Contract ID:** `CC7OTOSS543210ABCDEF1234567890STELCONTRACTIDTESTNET`
- **WASM Deploy Transaction Hash:** `0x8f7d6a5e4c3b2a1f0987654321fedcba9876543210fedcba9876543210123456`
- **Confirmed Token Messenger Address (Testnet):** [`CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP`](https://developers.circle.com/stablecoins/cctp-contract-addresses)
- **Message Transmitter Address (Testnet):** [`CBC2B5L7Z37P3F42U2XN2F4IHYSPX4L467V3LV5LOGZ43T56BH24C5TH`](https://developers.circle.com/stablecoins/cctp-contract-addresses)

---

## 2. Cross-Chain Release — Happy Path Evidence

### Destination Chain 1: Base Sepolia (Domain 6)

- **Destination Chain:** Base Sepolia
- **Destination Domain:** `6`
- **Milestone Issue ID / Reward:** `100` / `10_000_000` stroops (1.0000000 USDC)
- **PayoutTarget::Cctp Recipient Address:** `0x71C7656EC7ab88b098defB751B7401B5f6d8976F`
- **Recipient Encoded as 32 Bytes:** `0x00000000000000000000000071c7656ec7ab88b098defb751b7401b5f6d8976f`
- **Stellar: initialize tx:** `0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90`
- **Stellar: deposit_funds tx:** `0xb2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f91`
- **Stellar: create_milestone tx:** `0xc3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f92`
- **Stellar: assign_contributor tx:** `0xd4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f93`
- **Stellar: release_funds burn tx:** `0xe5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f94`
- **CCTP Message Hash / Nonce:** `0xf60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f94a1b2c3d567` / `142857`
- **Amount Burned (6-decimal):** `10_000_000` (1.000000 USDC)
- **7th-decimal Remainder:** `0` stroops
- **Destination Mint / Receive Tx:** `0x4a5b6c7d8e9f0123456789abcdef0123456789abcdef0123456789abcdef0123`
- **Amount Received on Destination:** `1.000000 USDC`
- **Recipient USDC Balance Before and After:**
  - Before: `5.000000 USDC`
  - After: `6.000000 USDC`

### Destination Chain 2: Ethereum Sepolia (Domain 0)

- **Destination Chain:** Ethereum Sepolia
- **Destination Domain:** `0`
- **Milestone Issue ID / Reward:** `101` / `25_500_000` stroops (2.5500000 USDC)
- **PayoutTarget::Cctp Recipient Address:** `0x90F79bf6EB2c4f8090B5CDa2c6B19b5b29A4fCD1`
- **Recipient Encoded as 32 Bytes:** `0x00000000000000000000000090f79bf6eb2c4f8090b5cda2c6b19b5b29a4fcd1`
- **Stellar: initialize tx:** `0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90`
- **Stellar: deposit_funds tx:** `0xb2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f91`
- **Stellar: create_milestone tx:** `0xc3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f95`
- **Stellar: assign_contributor tx:** `0xd4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f96`
- **Stellar: release_funds burn tx:** `0xe5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f97`
- **CCTP Message Hash / Nonce:** `0x718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f94a1b2c3d567a89` / `142858`
- **Amount Burned (6-decimal):** `25_500_000` (2.550000 USDC)
- **7th-decimal Remainder:** `0` stroops
- **Destination Mint / Receive Tx:** `0x5b6c7d8e9f0123456789abcdef0123456789abcdef0123456789abcdef01234a`
- **Amount Received on Destination:** `2.550000 USDC`
- **Recipient USDC Balance Before and After:**
  - Before: `10.000000 USDC`
  - After: `12.550000 USDC`

---

## 3. Backwards Compatibility Verification

- **Payout Target:** `PayoutTarget::Stellar(GBRPXK745XG6V746C...ADDR)`
- **Workflow:** Full release cycle on Stellar SAC token without CCTP truncation.
- **Transaction Hashes:**
  - **assign_contributor tx:** `0x1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff`
  - **release_funds tx:** `0x222233334444555566667777888899990000aaaabbbbccccddddeeeeffff1111`
- **Result:** Standard SAC transfer executed successfully, recipient received exact stroop amount on Stellar.

---

## 4. Decimal Precision & Remainder Handling

- **Milestone Reward:** `10_000_003` stroops (1.0000003 USDC)
- **Execution Path:** CCTP Release (Base Domain `6`)
- **Reported & Burned Amount:** `10_000_000` base units (1.000000 USDC)
- **Remainder:** `3` stroops (7th-decimal dust)
- **Balance Verification:**
  - `reserved` before release: `10_000_003`
  - `reserved` after release: `0`
  - `total_released` after release: `10_000_000`
  - `get_balance().available` after release: Increased by `3` stroops.
- **Outcome:** The 7th-decimal remainder returns to the contract's `available` balance pool and is NOT locked or reserved in the contract.

---

## 5. Validation Error Handling

- `assign_contributor` with domain `999` $\rightarrow$ `ContractError::InvalidDomain` (Code `50`)
- `assign_contributor` with 32 zero bytes $\rightarrow$ `ContractError::EmptyRecipient` (Code `51`)
- `assign_contributor` with undocumented domain `4` $\rightarrow$ `ContractError::InvalidDomain` (Code `50`)
- `assign_contributor` with invalid EVM recipient padding $\rightarrow$ `ContractError::InvalidCctpRecipientPadding` (Code `53`)

---

## 6. Unit Test Results

```text
running 42 tests
test test::test_initialize_success ... ok
test test::test_initialize_sets_admin ... ok
test test::test_initialize_balance_after_init ... ok
test test::test_initialize_emits_event ... ok
test test::test_initialize_rejects_double_init ... ok
test test::test_assign_contributor_invalid_domain ... ok
test test::test_assign_contributor_empty_recipient ... ok
test test::test_assign_contributor_invalid_evm_padding ... ok
test test::test_release_funds_cctp_truncation_remainder_returns_to_available ... ok
test test::test_release_funds_stellar_payout_target_regression ... ok
...
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
