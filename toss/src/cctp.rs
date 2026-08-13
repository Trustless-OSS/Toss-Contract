use crate::error::ContractError;
use crate::types::PayoutTarget;
use soroban_sdk::{contractclient, token, Address, BytesN, Env, String};

pub const CCTP_TOKEN_MESSENGER_MINTER: &str =
    "CDNG7HXAPBWICI2E3AUBP3YZWZELJLYSB6F5CC7WLDTLTHVM74SLRTHP";

pub fn is_supported_domain(domain: u32) -> bool {
    // Ethereum: 0, Avalanche: 1, Arbitrum: 3, Solana: 5, Base: 6, Polygon PoS: 7, Starknet: 25
    matches!(domain, 0 | 1 | 3 | 5 | 6 | 7 | 25)
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

pub fn cc_release_fund(
    env: &Env,
    token: &Address,
    target: &PayoutTarget,
    amount: i128,
) -> Result<(), ContractError> {
    match target {
        PayoutTarget::None => Err(ContractError::ContributorNotSet),
        PayoutTarget::Stellar(recipient_address) => {
            let token_client = token::Client::new(env, token);
            token_client.transfer(&env.current_contract_address(), recipient_address, &amount);
            Ok(())
        }
        PayoutTarget::Cctp(destination_domain, recipient) => {
            if !is_supported_domain(*destination_domain) {
                return Err(ContractError::InvalidDomain);
            }
            if recipient.iter().all(|b| b == 0) {
                return Err(ContractError::EmptyRecipient);
            }
            if !has_valid_padding(*destination_domain, recipient) {
                return Err(ContractError::InvalidCctpRecipientPadding);
            }

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
            cctp_client.deposit_for_burn(&amount, destination_domain, recipient, token);

            Ok(())
        }
    }
}

#[contractclient(name = "CctpClient")]
pub trait CctpTokenMessengerMinter {
    fn deposit_for_burn(
        env: Env,
        amount: i128,
        destination_domain: u32,
        mint_recipient: BytesN<32>,
        mint_token: Address,
    ) -> u64;
}
