use askama::Template;
use bincode::{Decode, Encode};

use crate::templates::{BordaResultsTemplate, RankedVoteTemplate};

use super::*;

#[derive(Encode, Decode)]
pub struct BordaPoll {
    pub options: Vec<(String, u64)>,
}

impl PollFormat for BordaPoll {
    fn from_query(query: &QString) -> Result<Box<Self>, &'static str>
    where
        Self: Sized,
    {
        let options_string = query
            .get("options")
            .ok_or("'options' query element not found")?;
        let options = options_string
            .split(',')
            .map(|s| (s.to_string(), 0u64))
            .collect();
        Ok(Box::new(BordaPoll { options }))
    }

    fn create_site_add_option_script() -> Result<String, &'static str>
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

    fn register_votes(&mut self, query: &QString) -> Result<(), &'static str> {
        let n = self.options.len() as u64;
        for i in 0..self.options.len() {
            let resp = query.get(&i.to_string()).ok_or("query invalid")?;
            let place: u64 = resp.parse().map_err(|_| "'response' must be a number")?;
            self.options
                .get_mut(i)
                .ok_or("option outside of the range of options")?
                .1 += n - (place + 1);
        }
        Ok(())
    }

    fn save_state(&self) -> Vec<u8> {
        // TODO: handle errors differently
        match bincode::encode_to_vec(self, bincode::config::standard()) {
            Ok(b) => b,
            Err(e) => {
                log::error!("Error while saving state for poll: {}", e);
                Vec::new()
            }
        }
    }

    fn from_bytes(bytes: Vec<u8>) -> Box<Self>
    where
        Self: Sized,
    {
        // TODO: handle errors differently
        match bincode::decode_from_slice(&bytes, bincode::config::standard()) {
            Ok((s, _)) => Box::new(s),
            Err(e) => {
                log::error!("Error while reading poll format data: {}", e);
                Box::new(Self {
                    options: Vec::new(),
                })
            }
        }
    }
}
