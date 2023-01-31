mod multiple;
mod single;
pub use multiple::MultipleChoicePoll;
pub use single::SingleChoicePoll;

pub mod templates {
    use crate::poll::{PollData, PollType};
    use askama::Template;

    #[derive(Template)]
    #[template(path = "simple/create.html")]
    pub struct SimpleCreateTemplate {
        /// Determines whether to display radio buttons (single choice)
        /// or checkboxes (multiple choice)
        pub poll_type: PollType,
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
}
