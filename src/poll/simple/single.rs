use anyhow::{anyhow, Context};
use askama::Template;
use bincode::{Decode, Encode};

use super::templates::*;
use crate::poll::*;

#[derive(Encode, Decode)]
pub struct SingleChoicePoll {
    /// Contains (Option name, points)
    pub options: Vec<(String, u64)>,
}

impl PollFormat for SingleChoicePoll {
    /// Format:
    /// `{option1},{option2},...,{optionN}
    /// `{option}` - option name (string)
    fn from_data(data: &str) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized,
    {
        let options: Vec<(String, u64)> = data.split(',').map(|s| (s.to_string(), 0u64)).collect();
        if options.len() < 2 {
            return Err(anyhow!("Too few options specified"));
        }
        Ok(Box::new(SingleChoicePoll { options }))
    }

    fn voting_site(&self, data: &PollData) -> Result<String, askama::Error> {
        let options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| (opt.as_str(), *n))
            .collect();
        SimpleVoteTemplate {
            poll: data,
            options: &options,
            multiple: false,
        }
        .render()
    }

    fn results_site(&self, data: &PollData) -> Result<String, askama::Error> {
        let mut options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| (opt.as_str(), *n))
            .collect();
        options.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        SimpleResultsTemplate {
            poll: data,
            options_sorted: &options,
        }
        .render()
    }

    /// Format:
    /// response={n}
    /// n - the index of the selected option
    fn register_votes(&mut self, query: &str) -> Result<(), anyhow::Error> {
        if !query.starts_with("response=") {
            return Err(anyhow!("Expected 'response' query element"));
        }
        let opt: usize = query[9..].parse().context("'response' must be a number")?;
        self.options
            .get_mut(opt)
            .context("'response' is outside of the range of options")?
            .1 += 1;
        Ok(())
    }

    fn save_state(&self) -> Result<Vec<u8>, anyhow::Error> {
        bincode::encode_to_vec(self, bincode::config::standard()).context("Failed to encode state")
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Box<dyn PollFormat>, anyhow::Error>
    where
        Self: Sized,
    {
        let (dec, _): (Self, _) = bincode::decode_from_slice(&bytes, bincode::config::standard())
            .context("Failed to decode state")?;
        Ok(Box::new(dec))
    }

    fn reset(&mut self) {
        self.options.iter_mut().for_each(|(_, c)| *c = 0);
    }
}
