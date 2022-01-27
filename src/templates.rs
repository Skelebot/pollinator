use askama::Template;

use crate::poll::{PollType, PositionalSystem};

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate<'a> {
    pub title: &'a str,
}

#[derive(Template)]
#[template(path = "vote.html")]
pub struct VoteTemplate<'a> {
    pub poll_name: &'a str,
    pub poll_id: usize,
    pub poll_type: PollType,
    pub options: &'a [(&'a str, u64)]
}

#[derive(Template)]
#[template(path = "results.html")]
pub struct ResultsTemplate<'a> {
    pub poll_name: &'a str,
    pub poll_id: usize,
    pub poll_type: PollType,
    pub voters: u64,
    pub options_sorted: &'a [(&'a str, u64)],
}

#[derive(Template)]
#[template(path = "create.html")]
pub struct CreateTemplate {
    pub poll_type: PollType,
}