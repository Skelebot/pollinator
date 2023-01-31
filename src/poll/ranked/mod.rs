mod borda;
mod dowdall;

pub use borda::BordaPoll;
pub use dowdall::DowdallPoll;

pub mod templates {
    use crate::poll::{PollData, PollType};
    use askama::Template;

    #[derive(Template)]
    #[template(path = "ranked/create.html")]
    pub struct RankedCreateTemplate {
        /// Gets passed on to the handle_create_desc in a POST request
        pub poll_type: PollType,
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
    #[template(path = "ranked/ranked_results.html")]
    pub struct RankedResultsTemplate<'a> {
        pub poll: &'a PollData,
        pub options_sorted: &'a [(&'a str, u64)],
    }
}
