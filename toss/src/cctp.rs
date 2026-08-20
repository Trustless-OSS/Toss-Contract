// The V2 TokenMessengerMinter `deposit_for_burn` interface carries 9
// parameters; circlefin/stellar-cctp applies the same allow on its trait.
#![allow(clippy::too_many_arguments)]

use crate::error::ContractError;
use crate::types::PayoutTarget;
use soroban_sdk::{contractclient, token, Address, BytesN, Env, String};

/// Stellar testnet TokenMessengerMinter (V2), per Circle's Stellar CCTP
/// contracts page (https://developers.circle.com/stablecoins/docs/stellar-cctp-contract-addresses).
pub const CCTP_TOKEN_MESSENGER_MINTER: &str =
    "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP";

/// Stellar testnet MessageTransmitter, from the same Circle source. Outbound
/// releases never call it (the recipient or a relayer executes the destination
/// receive), but it is kept as a named constant for documentation and tooling.
pub const CCTP_MESSAGE_TRANSMITTER: &str =
    "CBJ6MTCKKZG73PMDZCJMSFRD7DQEMI4FKDH7CGDSV4W6FHCRBCQAVVJY";

/// Minimum finality threshold requested for every burn. 2000 is CCTP's standard
/// "fully finalized" level: it avoids the fast-mint path, so the destination
/// mint equals the burned amount and no fee is charged on receive.
pub const CCTP_MIN_FINALITY_THRESHOLD: u32 = 2000;

pub fn is_supported_domain(domain: u32) -> bool {
    // Ethereum: 0, Avalanche: 1, OP Mainnet: 2, Arbitrum: 3, Solana: 5,
    // Base: 6, Polygon PoS: 7, Starknet: 25.
    // Domain 4 (Noble) is deliberately absent: it is CCTP V1-only and the
    // V2 TokenMessengerMinter on Stellar does not support it.
    matches!(domain, 0 | 1 | 2 | 3 | 5 | 6 | 7 | 25)
}

pub fn truncate_to_6_decimals(amount: i128) -> i128 {
    (amount / 10) * 10
}

pub fn cctp_remainder(amount: i128) -> i128 {
    amount % 10
}

pub fn has_valid_padding(domain: u32, recipient: &BytesN<32>) -> bool {
    // EVM domains require the first 12 bytes to be zero.
    // Ethereum: 0, Avalanche: 1, Arbitrum: 3, Base: 6, Polygon PoS: 7
    if matches!(domain, 0 | 1 | 3 | 6 | 7) {
        for i in 0..12 {
            if recipient.get(i).unwrap_or(0) != 0 {
                return false;
            }
        }
    }
    true
}

pub(crate) fn validate_cctp_target(target: &PayoutTarget) -> Result<(), ContractError> {
    if let PayoutTarget::Cctp(domain, recipient) = target {
        if !is_supported_domain(*domain) {
            return Err(ContractError::InvalidDomain);
        }
        if recipient.iter().all(|b| b == 0) {
            return Err(ContractError::EmptyRecipient);
        }
        if !has_valid_padding(*domain, recipient) {
            return Err(ContractError::InvalidCctpRecipientPadding);
        }
    }

    Ok(())
}

pub fn cc_release_fund(
    env: &Env,
    token: &Address,
    target: &PayoutTarget,
    amount: i128,
) -> Result<(), ContractError> {
    validate_cctp_target(target)?;

    match target {
        PayoutTarget::None => Err(ContractError::ContributorNotSet),
        PayoutTarget::Stellar(recipient_address) => {
            let token_client = token::Client::new(env, token);
            token_client.transfer(&env.current_contract_address(), recipient_address, &amount);
            Ok(())
        }
        PayoutTarget::Cctp(destination_domain, recipient) => {
            if amount == 0 {
                return Err(ContractError::ZeroBurnAmount);
            }

            let cctp_address =
                Address::from_string(&String::from_str(env, CCTP_TOKEN_MESSENGER_MINTER));

            let token_client = token::Client::new(env, token);
            token_client.approve(
                &env.current_contract_address(),
                &cctp_address,
                &amount,
                &(env.ledger().sequence() + 100),
            );

            let cctp_client = CctpClient::new(env, &cctp_address);
            cctp_client.deposit_for_burn(
                &env.current_contract_address(),
                &amount,
                destination_domain,
                recipient,
                token,
                &BytesN::from_array(env, &[0u8; 32]),
                &0,
                &CCTP_MIN_FINALITY_THRESHOLD,
            );

            Ok(())
        }
    }
}

#[contractclient(name = "CctpClient")]
pub trait CctpTokenMessengerMinter {
    /// V2 consolidated TokenMessengerMinter `deposit_for_burn`, matching
    /// circlefin/stellar-cctp (contracts/token-messenger-minter-v2). `caller`
    /// is the contract itself; `destination_caller` is zeroed so any address
    /// may broadcast the attestation on the destination chain.
    fn deposit_for_burn(
        env: Env,
        caller: Address,
        amount: i128,
        destination_domain: u32,
        mint_recipient: BytesN<32>,
        burn_token: Address,
        destination_caller: BytesN<32>,
        max_fee: i128,
        min_finality_threshold: u32,
    );
}
