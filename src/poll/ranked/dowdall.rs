use std::cmp::Ordering;

use anyhow::{anyhow, Context};
use askama::Template;
use bincode::{Decode, Encode};

use super::templates::*;
use crate::poll::{PollData, PollFormat};
use crate::util;

#[derive(Template)]
#[template(path = "ranked/dowdall_results.html")]
pub struct DowdallResultsTemplate<'a> {
    pub poll: &'a PollData,
    pub options_sorted: &'a [(&'a str, f32)],
    pub points_total: f32,
}

#[derive(Encode, Decode)]
pub struct DowdallPoll {
    pub options: Vec<(String, f32)>,
}

impl PollFormat for DowdallPoll {
    /// Format:
    /// `{option1},{option2},...,{optionN}
    /// `{option}` - option name (string)
    fn from_data(data: &str) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized,
    {
        let options: Vec<(String, f32)> = data.split(',').map(|s| (s.to_string(), 0.0)).collect();
        if options.len() < 2 {
            return Err(anyhow!("Too few options specified"));
        }
        Ok(Box::new(DowdallPoll { options }))
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

    /// Format:
    /// {p0},{p1},...,{pN-1},{pN}
    /// p{N} - place assigned to the option number N
    /// N - the number of poll options
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
            let option = self
                .options
                .get_mut(index)
                .context("Option number out of range")?;
            option.1 += 1.0 / (opt_place + 1) as f32;
        }
        Ok(())
    }

    fn save_state(&self) -> Result<Vec<u8>, anyhow::Error> {
        bincode::encode_to_vec(self, bincode::config::standard()).context("Failed to encode state")
    }

    fn reset(&mut self) {
        self.options.iter_mut().for_each(|(_, c)| *c = 0.0);
    }
}
