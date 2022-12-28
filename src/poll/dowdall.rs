use std::cmp::Ordering;

use anyhow::Context;
use askama::Template;
use bincode::{Decode, Encode};

use crate::templates::{DowdallResultsTemplate, RankedVoteTemplate};

use super::*;

#[derive(Encode, Decode)]
pub struct DowdallPoll {
    pub options: Vec<(String, f32)>,
}

impl PollFormat for DowdallPoll {
    fn from_data(data: &str) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized,
    {
        let options = data
            .split(',')
            .map(|s| (s.to_string(), 0.0))
            .collect();
        Ok(Box::new(DowdallPoll { options }))
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
        let points_per_round = {
            let mut sum: f32 = 0.0;
            for i in 1..self.options.len() + 1 {
                sum += 1.0 / i as f32;
            }
            sum
        };
        let mut options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| (opt.as_str(), *n))
            .collect();
        options.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        DowdallResultsTemplate {
            poll: data,
            options_sorted: &options,
            points_total: data.voters as f32 * points_per_round,
        }
        .render()
    }

    fn register_votes(&mut self, query: &QString) -> Result<(), anyhow::Error> {
        for i in 0..self.options.len() {
            let resp = query
                .get(&i.to_string())
                .context("Expected number arguments 0..{number of options}")?;
            let place: u64 = resp.parse().context("'response' must be a number")?;
            self.options
                .get_mut(i)
                .context("option outside of the range of options")?
                .1 += 1.0 / (place + 1) as f32;
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
        self.options.iter_mut().for_each(|(_, c)| *c = 0.0);
    }
}
