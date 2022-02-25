use anyhow::{anyhow, Context};
use askama::Template;
use bincode::{Decode, Encode};

use crate::templates::{BordaResultsTemplate, RankedVoteTemplate};

use super::*;

#[derive(Encode, Decode)]
pub struct BordaPoll {
    pub options: Vec<(String, u64)>,
}

impl PollFormat for BordaPoll {
    fn from_query(query: &QString) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized,
    {
        let options_string = query
            .get("options")
            .context("'options' query element not found")?;
        let options = options_string
            .split(',')
            .map(|s| (s.to_string(), 0u64))
            .collect();
        Ok(Box::new(BordaPoll { options }))
    }

    fn create_site_add_option_script() -> Result<String, anyhow::Error>
    where
        Self: Sized,
    {
        Ok(String::new())
    }

    fn voting_site(&self, data: &PollData) -> Result<String, askama::Error> {
        let options: Vec<_> = self.options.iter().map(|(opt, _)| opt.as_str()).collect();
        RankedVoteTemplate {
            poll: data,
            options: &options,
            can_unranked: false,
            unique_scores: true,
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
        BordaResultsTemplate {
            poll: data,
            options_sorted: &options,
        }
        .render()
    }

    fn register_votes(&mut self, query: &QString) -> Result<(), anyhow::Error> {
        let n = self.options.len() as u64;
        for (i, (_, votes)) in self.options.iter_mut().enumerate() {
            let resp = query
                .get(&i.to_string())
                .context("Expected number arguments 0..{number of options}")?;
            let place: u64 = resp
                .parse()
                .context(anyhow!("option value not a number: {}", resp))?;
            *votes += n - (place + 1);
        }
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
