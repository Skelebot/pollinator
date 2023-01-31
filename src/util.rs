use crate::error::ParseError;
use anyhow::{anyhow, Context};
use base64::alphabet::URL_SAFE;
use base64::engine::fast_portable::NO_PAD;
use rand::Rng;
use std::time::Duration;

macro_rules! return_html {
    ($html:expr) => {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body($html))
    };
}

/// Tries to retrieve a Duration in seconds from an environmental variable or returns a default.
pub fn get_env_duration_or(name: &str, default: Duration) -> anyhow::Result<Duration> {
    std::env::var(name)
        .ok()
        .map(|n| n.parse().context(anyhow!("{}, must be a number", name)))
        .map(|s| s.map(Duration::from_secs))
        .unwrap_or(Ok(default))
}

/// Parses poll options in format:
/// {0}={p}&{1}={p}&...,{n-1}={p}&{n}={p}
/// 0,1...n - the option index
/// p - points assigned to the option
/// Checks whether the resulting number of options equals the expected number.
/// Returns a Vec of tuples (index, points)
pub fn parse_poll_opts(query: &str, num_opts: usize) -> anyhow::Result<Vec<(u32, u32)>> {
    if query.chars().filter(|c| *c == '&').count() + 1 != num_opts {
        return Err(anyhow!(
            "The number of query elements must be equal to the number of poll options"
        ));
    }
    let mut opts = Vec::with_capacity(num_opts);
    for opt in query.split('&') {
        let eq = opt.find('=').context("Expected '=' in option")?;
        let idx: u32 = opt[..eq]
            .parse()
            .context(anyhow!("option index not a number: {}", &opt[..eq]))?;
        let points: u32 = opt[eq + 1..]
            .parse()
            .context(anyhow!("option value not a number: {}", &opt[eq + 1..]))?;
        opts.push((idx, points));
    }

    Ok(opts)
}

pub const BASE64_ENGINE: base64::engine::fast_portable::FastPortable =
    base64::engine::fast_portable::FastPortable::from(&URL_SAFE, NO_PAD);

pub fn read_base64_u64(value: &str) -> Result<u64, ParseError> {
    let mut bytes: [u8; 8] = [0; 8];
    base64::decode_engine_slice(value, &mut bytes, &BASE64_ENGINE)
        .map_err(ParseError::InvalidBase64)?;

    Ok(u64::from_be_bytes(bytes))
}

pub fn encode_base64_u64(value: u64) -> String {
    base64::encode_engine(value.to_be_bytes(), &BASE64_ENGINE)
}

pub fn random_base64_u64() -> String {
    let mut rng = rand::thread_rng();
    encode_base64_u64(rng.gen())
}
