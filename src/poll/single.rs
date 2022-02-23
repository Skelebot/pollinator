use askama::Template;
use bincode::{Decode, Encode};

use crate::templates::{SimpleResultsTemplate, SimpleVoteTemplate};

use super::*;

#[derive(Encode, Decode)]
pub struct SingleChoicePoll {
    pub options: Vec<(String, u64)>,
}

impl PollFormat for SingleChoicePoll {
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
        Ok(Box::new(SingleChoicePoll { options }))
    }

    fn create_site_add_option_script() -> Result<String, &'static str>
    where
        Self: Sized,
    {
        Ok(String::new())
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

    fn register_votes(&mut self, query: &QString) -> Result<(), &'static str> {
        let resp = query
            .get("response")
            .ok_or("'response' query element not found")?;
        let opt: usize = resp.parse().map_err(|_| "'response' must be a number")?;
        self.options
            .get_mut(opt)
            .ok_or("'response' is outside of the range of options")?
            .1 += 1;
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
