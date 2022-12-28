use crate::error::ParseError;
use rand::Rng;

macro_rules! return_html {
    ($html:expr) => {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body($html))
    };
}

pub const BASE64_CONFIG: base64::Config = base64::Config::new(base64::CharacterSet::UrlSafe, false);

pub fn read_base64_u64(value: &str) -> Result<u64, ParseError> {
    let bytes = base64::decode_config(value, BASE64_CONFIG).map_err(ParseError::InvalidBase64)?;

    let arr: [u8; 8] = bytes.try_into().map_err(|_| ParseError::Base64TooShort)?;

    Ok(u64::from_be_bytes(arr))
}

pub fn encode_base64_u64(value: u64) -> String {
    base64::encode_config(value.to_be_bytes(), BASE64_CONFIG)
}

pub fn random_base64_u64() -> String {
    let mut rng = rand::thread_rng();
    encode_base64_u64(rng.gen())
}
