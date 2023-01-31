use anyhow::{anyhow, Context};
use askama::Template;
use bincode::{Decode, Encode};

use crate::poll::{PollData, PollFormat, PollType};

use crate::util;
use templates::*;

pub mod templates {
    use super::*;

    #[derive(Template)]
    #[template(path = "score/create.html")]
    pub struct ScoreCreateTemplate {
        /// Gets passed on to the handle_create_desc in a POST request
        pub poll_type: PollType,
    }

    #[derive(Template)]
    #[template(path = "score/vote.html")]
    pub struct ScoreVoteTemplate<'a> {
        pub poll: &'a PollData,
        pub points_min: u32,
        pub points_max: u32,
        pub options: &'a [(&'a str, u64)],
    }

    #[derive(Template)]
    #[template(path = "score/results.html")]
    pub struct ScoreResultsTemplate<'a> {
        pub poll: &'a PollData,
        pub options_sorted: &'a [(&'a str, u64)],
        pub points_total: u64,
        pub points_max: u64,
    }
}

#[derive(Encode, Decode)]
pub struct ScoredChoicePoll {
    points_min: u32,
    points_max: u32,
    pub options: Vec<(String, u64)>,
}

impl PollFormat for ScoredChoicePoll {
    /// Format:
    /// `{option1},{option2},...,{optionN},points_min,points_max`
    /// `{option}` - option name (string)
    /// points_min - minimum assignable amount of points (integer)
    /// points_max - maximum assignable amount of points (integer, larger than points_min) (inclusive)
    fn from_data(data: &str) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized,
    {
        let mut options: Vec<(String, u64)> =
            data.split(',').map(|s| (s.to_string(), 0u64)).collect();

        if options.len() < 4 {
            return Err(anyhow!("Too few options specified"));
        }

        // Last two elements are points_min and points_max
        // 0 and 5 are the default values
        let points_max = options.pop().map(|o| o.0.parse::<u32>()).unwrap_or(Ok(0))?;
        let points_min = options.pop().map(|o| o.0.parse::<u32>()).unwrap_or(Ok(5))?;

        if points_min >= points_max {
            return Err(anyhow!("points_min must be lower than points_max"));
        }

        Ok(Box::new(ScoredChoicePoll {
            options,
            points_min,
            points_max,
        }))
    }

    fn voting_site(&self, data: &PollData) -> Result<String, askama::Error> {
        let options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| (opt.as_str(), *n))
            .collect();

        ScoreVoteTemplate {
            poll: data,
            points_min: self.points_min,
            points_max: self.points_max,
            options: &options,
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
        let points_total = options.iter().map(|(_, p)| p).sum();
        ScoreResultsTemplate {
            poll: data,
            options_sorted: &options,
            points_total,
            points_max: data.voters * self.points_max as u64,
        }
        .render()
    }

    /// Format:
    /// {0}={p}&{1}={p}&...,{n-1}={p}&{n}={p}
    /// 0,1...n - the option index. Must be in exact order.
    /// p - points assigned to the option
    fn register_votes(&mut self, query: &str) -> Result<(), anyhow::Error> {
        let opts = util::parse_poll_opts(query, self.options.len())?;
        for (index, (opt_index, opt_points)) in opts.iter().enumerate() {
            // Check the argument order. Avoids malicious requests that do not vote
            // on some options or vote twice on one
            if *opt_index as usize != index {
                return Err(anyhow!(
                    "unexpected index: {}, expected {}",
                    opt_index,
                    index
                ));
            }
            if !(self.points_min..=self.points_max).contains(opt_points) {
                return Err(anyhow!("points value outside poll's assignable range"));
            }
            let option = self
                .options
                .get_mut(index)
                .context("Option number out of range")?;
            option.1 += *opt_points as u64;
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
