use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("Internal server error. Try again later")]
    InternalError(#[source] anyhow::Error),
    #[error("Failed to create poll from data")]
    PollCreation(#[source] anyhow::Error),
    #[error("Missing query parameter: {0}")]
    MissingParam(String),
    #[error("Too many requests")]
    TooManyRequests,
    #[error("Invalid admin token specified")]
    InvalidAdminToken,
    #[error("Admin functions are disabled on this server")]
    AdminOff,
    #[error("Invalid admin action")]
    InvalidAdminAction,
}

impl ResponseError for UserError {
    fn status_code(&self) -> StatusCode {
        use UserError::*;
        match *self {
            InternalError(_) | PollCreation(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MissingParam(_) | AdminOff | InvalidAdminAction => StatusCode::BAD_REQUEST,
            InvalidAdminToken => StatusCode::UNAUTHORIZED,
            TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use UserError::*;
        let mut req = HttpResponse::build(self.status_code());
        req.content_type("text/plain; charset=utf-8");

        match self {
            TooManyRequests => req.body(include_str!("../static/limit.html")),
            other => req.body(format!("{}", other)),
        }
    }
}

#[derive(Debug, Error, PartialEq)]
/// Error when parsing query parameters
pub enum ParseError {
    #[error("Poll type incomplete: {0}; expected more after byte {1}")]
    TypeIncomplete(String, usize),
    #[error("Invalid poll type: {0}")]
    InvalidPollType(String),
    #[error("Invalid positional system: {0}")]
    InvalidPositionalSystem(String),
    #[error("Error while parsing integer")]
    InvalidNumber(#[source] std::num::ParseIntError),
    #[error("Invalid base 64 number")]
    InvalidBase64(#[source] base64::DecodeError),
    #[error("Base 64 number too short")]
    Base64TooShort,
    #[error("Poll ID has to contain a '+'")]
    PlusNotFound,
}

/// Any query parsing error is considered a bad request
impl ResponseError for ParseError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
