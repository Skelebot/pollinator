use askama::Template;
use serde::Deserialize;

use crate::templates::*;

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
            _ => f.write_fmt(format_args!("{:?}", self))
        }
    }
}

impl PositionalSystem {
    pub fn try_parse(s: &str) -> Result<Self, ()> {
        if s.starts_with("Borda") {
            Ok(PositionalSystem::Borda)
        } else if s.starts_with("Dowdall") {
            Ok(PositionalSystem::Dowdall)
        } else if s.starts_with("Score") {
            let desc = s.get(5..).ok_or(())?;
            let num = desc.parse().map_err(|_| ())?;
            Ok(PositionalSystem::Score(num))
        } else {
            Err(())
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum PollType {
    Single,
    Multiple,
    Ranked(PositionalSystem),
}

impl std::fmt::Display for PollType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ranked(sys) => f.write_fmt(format_args!("Ranked{}", sys)),
            _ => f.write_fmt(format_args!("{:?}", self))
        }
    }
}

impl PollType {
    pub fn try_parse(s: &str) -> Result<Self, ()> {
        if s.starts_with("Single") {
            Ok(PollType::Single)
        } else if s.starts_with("Multiple") {
            Ok(PollType::Multiple)
        } else if s.starts_with("Ranked") {
            let desc = s.get(6..).ok_or(())?;
            log::info!("desc: {}", desc);
            let pos_system = PositionalSystem::try_parse(desc)?;
            Ok(PollType::Ranked(pos_system))
        } else {
            Err(())
        }
    }

    // ranked polls only
    pub fn can_unranked(&self) -> bool {
        if let PollType::Ranked(system) = self {
            match system {
                PositionalSystem::Borda => false,
                PositionalSystem::Dowdall => false,
                PositionalSystem::Score(_) => false,
            }
        } else {
            false // doesn't matter
        }
    }

    // ranked polls only
    // whether to include a script that forces the user to select
    // unique scores for every option
    pub fn unique_scores(&self) -> bool {
        if let PollType::Ranked(system) = self {
            match system {
                PositionalSystem::Borda => true,
                PositionalSystem::Dowdall => true,
                PositionalSystem::Score(_) => false,
            }
        } else {
            false // doesn't matter
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct PollOptionDesc {
    pub name: String,
}

impl<T> From<T> for PollOptionDesc
where
    T: AsRef<str>,
{
    fn from(s: T) -> Self {
        PollOptionDesc {
            name: s.as_ref().to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Poll {
    pub name: String,
    pub ptype: PollType,
    pub voters: u64,
    pub options: Vec<(PollOptionDesc, u64)>,
}

impl Poll {
    pub fn vote_template(&self, id: usize) -> Result<String, askama::Error> {
        let options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| (opt.name.as_str(), *n))
            .collect();
        VoteTemplate {
            options: &options,
            poll_name: &self.name,
            poll_id: id,
            poll_type: self.ptype,
        }
        .render()
    }
    pub fn results_template(&self, id: usize) -> Result<String, askama::Error> {
        let mut options: Vec<_> = self
            .options
            .iter()
            .map(|(opt, n)| {
                (
                    opt.name.as_str(),
                    *n,
                )
            })
            .collect();
        options.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        ResultsTemplate {
            options_sorted: &options,
            poll_name: &self.name,
            poll_type: self.ptype,
            voters: self.voters,
            poll_id: id,
        }
        .render()
    }
}

pub trait PollFormat {
    /// Variables:
    fn create_site_add_option_script() -> Result<String, &'static str>;
    fn voting_site(&self) -> Result<String, askama::Error>;
    fn results_site(&self) -> Result<String, askama::Error>;
    fn register_votes(&mut self, query: &str) -> Result<String, &'static str>;
}

pub struct SingleChoicePoll {
    votes: Vec<u64>,
}

impl PollFormat for SingleChoicePoll {
    fn create_site_add_option_script() -> Result<String, &'static str> {
        todo!()
    }

    fn voting_site(&self) -> Result<String, askama::Error> {
        todo!()
    }

    fn results_site(&self) -> Result<String, askama::Error> {
        todo!()
    }

    fn register_votes(&mut self, query: &str) -> Result<String, &'static str> {
        todo!()
    }
}