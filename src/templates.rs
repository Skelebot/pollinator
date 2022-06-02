use askama::Template;

use crate::poll::{PollData, PollID, PollType};

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate<'a> {
    pub title: &'a str,
}
#[derive(Template)]
#[template(path = "create.html")]
pub struct CreateTemplate {
    pub poll_type: PollType,
}

#[derive(Template)]
#[template(path = "poll_created.html")]
pub struct PollCreatedTemplate<'a> {
    pub name: &'a str,
    pub voting_link: &'a str,
    pub results_link: &'a str,
    pub admin_link: &'a str,
    pub admin_token: &'a str,
}

#[derive(Template)]
#[template(path = "voted.html")]
pub struct VotedTemplate<'a> {
    pub results_link: &'a str,
}

#[derive(Template)]
#[template(path = "simple/vote.html")]
pub struct SimpleVoteTemplate<'a> {
    pub poll: &'a PollData,
    pub multiple: bool,
    pub options: &'a [(&'a str, u64)],
}

#[derive(Template)]
#[template(path = "simple/results.html")]
pub struct SimpleResultsTemplate<'a> {
    pub poll: &'a PollData,
    pub options_sorted: &'a [(&'a str, u64)],
}

#[derive(Template)]
#[template(path = "ranked/vote.html")]
pub struct RankedVoteTemplate<'a> {
    pub poll: &'a PollData,
    pub can_unranked: bool,
    pub unique_scores: bool,
    pub options: &'a [&'a str],
}

#[derive(Template)]
#[template(path = "ranked/borda_results.html")]
pub struct BordaResultsTemplate<'a> {
    pub poll: &'a PollData,
    pub options_sorted: &'a [(&'a str, u64)],
}

#[derive(Template)]
#[template(path = "ranked/dowdall_results.html")]
pub struct DowdallResultsTemplate<'a> {
    pub poll: &'a PollData,
    pub options_sorted: &'a [(&'a str, f32)],
    pub points_total: f32,
}

/// All essential poll information - to be displayed in a poll list
pub struct PollInfo {
    pub id: PollID,
    pub name: String,
    pub poll_type: String,
    pub date_created: String,
    pub admin_token: String,
    pub voters: u64,
}

#[derive(Template)]
#[template(path = "poll_list.html")]
pub struct PollListTemplate {
    pub polls: Vec<PollInfo>,
}
