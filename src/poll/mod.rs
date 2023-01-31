use crate::error::{ParseError, UserError};
use anyhow::Context;
use askama::Template;
use bincode::Decode;
use rand::Rng;
use serde::Deserialize;

mod ranked;
mod score;
mod simple;

use ranked::{BordaPoll, DowdallPoll};
use score::ScoredChoicePoll;
use simple::{MultipleChoicePoll, SingleChoicePoll};

use crate::util;

pub struct Poll {
    pub data: PollData,
    pub format: Box<dyn PollFormat>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum PollType {
    /// Simple single-choice poll
    Single,
    /// Simple multiple-choice poll
    Multiple,
    /// Score poll
    /// Each voter assigns a number of points from a set range
    /// to every option (for example from 0 to 5 points).
    /// Options are ranked by sum of points they received.
    Score,
    /// Ranked-choice poll - each voter assigns a unique rank to every option.
    /// The positional system determines the rules of assigning points and ranking
    /// options based on their ranks.
    Ranked(PositionalSystem),
}

impl std::fmt::Display for PollType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ranked(sys) => f.write_fmt(format_args!("Ranked{}", sys)),
            _ => f.write_fmt(format_args!("{:?}", self)),
        }
    }
}

impl PollType {
    pub fn try_parse(s: &str) -> Result<Self, ParseError> {
        match s {
            "Single" => Ok(PollType::Single),
            "Multiple" => Ok(PollType::Multiple),
            "Score" => Ok(PollType::Score),
            s if s.starts_with("Ranked") => {
                let desc = s
                    .get(6..)
                    .ok_or_else(|| ParseError::TypeIncomplete(s.to_string(), 6))?;
                let pos_system = PositionalSystem::try_parse(desc)?;
                Ok(PollType::Ranked(pos_system))
            }
            _ => Err(ParseError::InvalidPollType(s.into())),
        }
    }
    pub fn creation_site(&self) -> Result<String, askama::Error> {
        let poll_type = *self;
        match self {
            PollType::Single | PollType::Multiple => {
                simple::templates::SimpleCreateTemplate { poll_type }.render()
            }
            PollType::Ranked(_) => ranked::templates::RankedCreateTemplate { poll_type }.render(),
            PollType::Score => score::templates::ScoreCreateTemplate { poll_type }.render(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
/// Both Borda and Dowdall systems are vulnerable to tactical voting. Dowdall system may be more
/// resistant, but little research has been done thus far on this system.
pub enum PositionalSystem {
    /// Borda count, points assigned in form of {number of candidates}-{position on ballot}
    /// The lowest-ranked option gets 0 points.
    /// Particularly susceptible to distortion through the presence of options which are
    /// mostly irrelevant (rarely come into consideration).
    Borda,
    /// Dowdall / Nauru system. The first-ranked option gets 1 point, the second gets 1/2 of a point,
    /// the third 1/3 of a point, etc.
    Dowdall,
}

impl std::fmt::Display for PositionalSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl PositionalSystem {
    pub fn try_parse(s: &str) -> Result<Self, ParseError> {
        match s {
            "Borda" => Ok(PositionalSystem::Borda),
            "Dowdall" => Ok(PositionalSystem::Dowdall),
            _ => Err(ParseError::InvalidPositionalSystem(s.into())),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct PollID(pub u64, u64);

impl PollID {
    /// NOTE: This does not guarantee that the poll exists. It should be used only when reading a
    /// poll id from the database.
    pub fn new(id: u64, randpart: u64) -> PollID {
        PollID(id, randpart)
    }
    pub fn generate(id: u64) -> PollID {
        let mut rng = rand::thread_rng();
        let randpart: u64 = rng.gen();
        PollID(id, randpart)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }

    pub fn randpart(&self) -> u64 {
        self.1
    }
}

impl std::fmt::Display for PollID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id_encoded = self.0.to_string();
        let randpart_encoded = util::encode_base64_u64(self.1);
        f.write_fmt(format_args!("{}+{}", id_encoded, randpart_encoded))
    }
}

impl TryFrom<&str> for PollID {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let sep = value.find('+').ok_or(ParseError::PlusNotFound)?;

        let id = (value[..sep])
            .parse::<u64>()
            .map_err(ParseError::InvalidNumber)?;
        let randpart = util::read_base64_u64(&value[sep + 1..])?;

        Ok(PollID(id, randpart))
    }
}

#[derive(Debug)]
pub struct PollData {
    /// ID and randpart
    pub id: PollID,
    pub ptype: PollType,
    pub name: String,
    pub date_created: chrono::DateTime<chrono::Utc>,
    pub admin_link: String,
    pub voters: u64,
}

pub trait PollFormat: Send + Sync + 'static {
    /// Extract poll data from POST query for poll creation
    /// the query data can be anything, the data is created in
    /// templates/create.html onSubmit() function.
    fn from_data(data: &str) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized;

    /// Return HTML of the website for voting
    fn voting_site(&self, data: &PollData) -> Result<String, askama::Error>;
    /// Return HTML of the poll's results
    fn results_site(&self, data: &PollData) -> Result<String, askama::Error>;
    /// Register a new voting request: for example add points to the options
    /// the user voted for.
    fn register_votes(&mut self, query: &str) -> Result<(), anyhow::Error>;

    /// Save the poll's data into bytes for storing inside a database
    fn save_state(&self) -> Result<Vec<u8>, anyhow::Error>;

    /// Restore the poll's data from bytes (from a database) and create
    /// a PollFormat trait object that can be operated on
    /// Default
    fn from_bytes(bytes: Vec<u8>) -> Result<Box<dyn PollFormat>, anyhow::Error>
    where
        Self: Sized + Decode,
    {
        let (dec, _): (Self, _) = bincode::decode_from_slice(&bytes, bincode::config::standard())
            .context("Failed to decode state")?;
        Ok(Box::new(dec))
    }

    /// Reset the poll's state to as if it was just created (used by admin options)
    fn reset(&mut self);
}

pub fn create_poll_format_from_data(
    ptype: PollType,
    data: &str,
) -> Result<Box<dyn PollFormat>, UserError> {
    Ok(match ptype {
        PollType::Single => SingleChoicePoll::from_data(data).map_err(UserError::PollCreation)?,
        PollType::Multiple => {
            MultipleChoicePoll::from_data(data).map_err(UserError::PollCreation)?
        }
        PollType::Score => ScoredChoicePoll::from_data(data).map_err(UserError::PollCreation)?,
        PollType::Ranked(sys) => match sys {
            PositionalSystem::Borda => {
                BordaPoll::from_data(data).map_err(UserError::PollCreation)?
            }
            PositionalSystem::Dowdall => {
                DowdallPoll::from_data(data).map_err(UserError::PollCreation)?
            }
        },
    })
}

pub fn create_poll_format_from_bytes(
    ptype: PollType,
    data: Vec<u8>,
) -> Result<Box<dyn PollFormat>, anyhow::Error> {
    match ptype {
        PollType::Single => SingleChoicePoll::from_bytes(data),
        PollType::Multiple => MultipleChoicePoll::from_bytes(data),
        PollType::Score => ScoredChoicePoll::from_bytes(data),
        PollType::Ranked(sys) => match sys {
            PositionalSystem::Borda => BordaPoll::from_bytes(data),
            PositionalSystem::Dowdall => DowdallPoll::from_bytes(data),
        },
    }
}

#[test]
fn test_poll_id() {
    let poll_id = PollID(12, 5732390254647088000);
    let encoded = poll_id.to_string();
    assert_eq!(&encoded, "12+gJMmqpCKjU8");
    let decoded = PollID::try_from(encoded.as_str());
    assert_eq!(decoded, Ok(poll_id));
}
