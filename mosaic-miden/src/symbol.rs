use miden_client::Felt;
use miden_client::account::AccountId;

pub fn encode_symbol(input: &str, faucet: &AccountId) -> Result<[Felt; 4], &'static str> {
    if input.len() > 8 {
        return Err("Input must be at most 8 characters long");
    }
    // Only uppercase ASCII [A-Z]
    if !input.bytes().all(|b: u8| b.is_ascii_uppercase()) {
        return Err("Only uppercase ASCII letters [A-Z] are allowed");
    }

    // Pack into the first u64 (big-endian); rest zero.
    let mut words = [0u64; 4];
    let mut acc = 0u64;
    for (i, b) in input.bytes().enumerate() {
        acc |= (b as u64) << (8 * (7 - i));
    }
    words[0] = acc;

    let prefix = faucet.prefix().as_u64();
    let suffix = faucet.suffix().as_int();

    words[2] = prefix;
    words[3] = suffix;

    let felts: [Felt; 4] = words.map(Felt::new);

    Ok(felts)
}
