use soroban_sdk::{contractclient, Address, BytesN, Env};

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
