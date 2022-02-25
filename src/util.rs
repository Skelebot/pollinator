use rand::Rng;

use crate::error::ParseError;

pub const BASE64_CONFIG: base64::Config = base64::Config::new(base64::CharacterSet::UrlSafe, false);

pub fn read_base64_u64(value: &str) -> Result<u64, ParseError> {
    let bytes = base64::decode_config(value, BASE64_CONFIG)
        .map_err(ParseError::InvalidBase64)?;

    let arr: [u8; 8] = bytes
        .try_into()
        .map_err(|_| ParseError::Base64TooShort)?;

    Ok(u64::from_le_bytes(arr))
}

pub fn encode_base64_u64(value: u64) -> String {
    base64::encode_config(value.to_le_bytes(), BASE64_CONFIG)
}

pub fn random_base64_u64() -> String {
    let mut rng = rand::thread_rng();
    encode_base64_u64(rng.gen())
}

#[test]
fn test_encode() {
    let a = encode_base64_u64(1);
    let b = encode_base64_u64(2);
    let c = encode_base64_u64(3);
    assert!(a != b);
    assert!(b != c);
}
