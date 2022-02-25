use crate::error::{ParseError, UserError};
use qstring::QString;
use rand::Rng;
use serde::Deserialize;

mod single;
pub use single::SingleChoicePoll;
mod multiple;
pub use multiple::MultipleChoicePoll;
mod borda;
pub use borda::*;
mod dowdall;
pub use dowdall::*;

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
    /// Ranked-choice poll - each voter assigns a number of points to each option
    /// the positional system determines the rules of assigning points and ranking
    /// options based on their points.
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
    Score(u32),
}

impl std::fmt::Display for PositionalSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Score(num) => f.write_fmt(format_args!("Score{}", num)),
            _ => f.write_fmt(format_args!("{:?}", self)),
        }
    }
}

impl PositionalSystem {
    pub fn try_parse(s: &str) -> Result<Self, ParseError> {
        match s {
            "Borda" => Ok(PositionalSystem::Borda),
            "Dowdall" => Ok(PositionalSystem::Dowdall),
            s if s.starts_with("Score") => {
                let desc = s
                    .get(5..)
                    .ok_or_else(|| ParseError::TypeIncomplete(s.to_string(), 5))?;
                let num = desc.parse().map_err(ParseError::InvalidNumber)?;
                Ok(PositionalSystem::Score(num))
            }
            _ => Err(ParseError::InvalidPositionalSystem(s.into())),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct PollID(u64, u64);

impl PollID {
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
        let id_encoded = util::encode_base64_u64(self.0);
        let randpart_encoded = util::encode_base64_u64(self.1);
        f.write_fmt(format_args!("{}+{}", id_encoded, randpart_encoded))
    }
}

impl TryFrom<&str> for PollID {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let sep = value.find('+').ok_or(ParseError::PlusNotFound)?;

        let id = util::read_base64_u64(&value[..sep])?;
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
    /// Variables:
    fn create_site_add_option_script() -> Result<String, anyhow::Error>
    where
        Self: Sized;

    fn from_query(query: &QString) -> Result<Box<Self>, anyhow::Error>
    where
        Self: Sized;

    fn voting_site(&self, data: &PollData) -> Result<String, askama::Error>;
    fn results_site(&self, data: &PollData) -> Result<String, askama::Error>;
    fn register_votes(&mut self, query: &QString) -> Result<(), anyhow::Error>;

    fn save_state(&self) -> Result<Vec<u8>, anyhow::Error>;
    fn from_bytes(bytes: Vec<u8>) -> Result<Box<dyn PollFormat>, anyhow::Error>
    where
        Self: Sized;
    fn reset(&mut self);
}

pub fn create_poll_format_from_query(
    ptype: PollType,
    query: &QString,
) -> Result<Box<dyn PollFormat>, UserError> {
    Ok(match ptype {
        PollType::Single => SingleChoicePoll::from_query(query).map_err(UserError::PollCreation)?,
        PollType::Multiple => {
            MultipleChoicePoll::from_query(query).map_err(UserError::PollCreation)?
        }
        PollType::Ranked(sys) => match sys {
            PositionalSystem::Borda => {
                BordaPoll::from_query(query).map_err(UserError::PollCreation)?
            }
            PositionalSystem::Dowdall => {
                DowdallPoll::from_query(query).map_err(UserError::PollCreation)?
            }
            PositionalSystem::Score(_) => {
                return Err(UserError::PollCreation(anyhow::anyhow!(
                    "Not yet implemented"
                )));
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
        PollType::Ranked(sys) => match sys {
            PositionalSystem::Borda => BordaPoll::from_bytes(data),
            PositionalSystem::Dowdall => DowdallPoll::from_bytes(data),
            // Unreachable - this fn is called while loading from the database,
            // and creating this type of poll is impossible for now (see create_poll_format_from_query)
            PositionalSystem::Score(_) => unreachable!(),
        },
    }
}

#[test]
fn test_poll_id() {
    let pollid = PollID(12, 5732390254647088000);
    let encoded = pollid.to_string();
    assert_eq!(&encoded, "DAAAAAAAAAA+gJMmqpCKjU8");
    let decoded = PollID::try_from(encoded.as_str());
    assert_eq!(decoded, Ok(pollid));
}
