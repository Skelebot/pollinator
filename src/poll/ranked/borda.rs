use anyhow::{anyhow, Context};
use askama::Template;
use bincode::{Decode, Encode};

use super::templates::*;
use crate::poll::{PollData, PollFormat};
use crate::util;

#[derive(Encode, Decode)]
pub struct BordaPoll {
    pub options: Vec<(String, u64)>,
}

impl PollFormat for BordaPoll {
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
        Ok(Box::new(BordaPoll { options }))
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
        RankedResultsTemplate {
            poll: data,
            options_sorted: &options,
        }
        .render()
    }

    /// Format:
    /// {0}={p}&{1}={p}&...,{n-1}={p}&{n}={p}
    /// 0,1...n - the option index. Must be in exact order.
    /// p - rank assigned to the option
    fn register_votes(&mut self, query: &str) -> Result<(), anyhow::Error> {
        let opts = util::parse_poll_opts(query, self.options.len())?;
        for (index, (opt_index, opt_place)) in opts.iter().enumerate() {
            // TODO: Check if place values are unique (wait until .is_sorted is stabilized?)
            // Check the argument order. Avoids malicious requests that do not vote
            // on some options or vote twice on one
            if *opt_index as usize != index {
                return Err(anyhow!(
                    "unexpected index: {}, expected {}",
                    opt_index,
                    index
                ));
            }
            let n = self.options.len() as u64;
            let option = self
                .options
                .get_mut(index)
                .context("Option number out of range")?;
            option.1 += n - (*opt_place as u64 + 1);
        }
        Ok(())
    }

    fn save_state(&self) -> Result<Vec<u8>, anyhow::Error> {
        bincode::encode_to_vec(self, bincode::config::standard()).context("Failed to encode state")
    }

    fn reset(&mut self) {
        self.options.iter_mut().for_each(|(_, c)| *c = 0);
    }
}
