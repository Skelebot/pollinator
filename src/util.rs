use rand::Rng;

pub const BASE64_CONFIG: base64::Config = base64::Config::new(base64::CharacterSet::UrlSafe, false);

pub fn read_base64_u64(value: &str) -> Result<u64, ()> {
    let decoded = base64::decode_config(value, BASE64_CONFIG).map_err(|_| ())?;
    if decoded.len() != std::mem::size_of::<u64>() {
        return Err(());
    }
    Ok(u64::from_le_bytes(decoded.try_into().unwrap()))
}

pub fn encode_base64_u64(value: u64) -> String {
    base64::encode_config(value.to_le_bytes(), BASE64_CONFIG)
}

pub fn random_base64_u64() -> String {
    let mut rng = rand::thread_rng();
    encode_base64_u64(rng.gen())
}
