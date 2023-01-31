use crate::poll::PollID;
use askama::Template;

#[derive(Template)]
#[template(path = "base.html")]
/// Contains the html overall template (header etc)
pub struct BaseTemplate<'a> {
    pub title: &'a str,
}

#[derive(Template)]
#[template(path = "poll_created.html")]
/// Returned when a poll was successfully created.
pub struct PollCreatedTemplate<'a> {
    pub name: &'a str,
    pub voting_link: &'a str,
    pub results_link: &'a str,
    pub admin_link: &'a str,
    pub admin_token: &'a str,
}

#[derive(Template)]
#[template(path = "voted.html")]
/// Returned when a vote was successfully registered.
pub struct VotedTemplate<'a> {
    pub results_link: &'a str,
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
/// Returned when a list of complete poll data is requested
pub struct PollListTemplate {
    pub polls: Vec<PollInfo>,
}
