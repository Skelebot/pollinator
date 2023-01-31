use crate::db::DbPool;
use crate::poll::PollID;
use crate::*;
use askama::Template;
use serde::Deserialize;

use actix_web::{web, HttpResponse, Result};

pub struct AdminToken(pub Option<String>);

/// Admin actions that can be executed on the server through
/// the admin page provided that the user knows the POLL_ADMIN_TOKEN.
#[derive(Deserialize, Debug)]
enum AdminAction {
    /// Removes all entries currently in the database.
    PurgeDatabase,
    /// Empties the request limit store.
    ResetLimits,
    /// Lists all polls currently in the database.
    ListPolls,
    /// Resets all votes on a poll. Poll specific.
    ResetVotes,
    /// Removes a poll from the database. Poll specific.
    DeletePoll,
}

#[derive(Deserialize)]
pub struct AdminParams {
    token: String,
    action: AdminAction,
}

/// Handles the general administration webpage
pub async fn handle_admin() -> Result<HttpResponse> {
    return_html!(include_str!("../static/admin.html"))
}

/// Handles the general administration webpage callback
/// Params:
///  - token: The admin token, should match the POLL_ADMIN_TOKEN environmental variable
///  - action: an AdminAction enum member, specifies the action to be executed.
/// Only the non-poll-specific administration actions can be executed from here.
pub async fn handle_admin_action(
    db: web::Data<DbPool>,
    params: web::Form<AdminParams>,
    limits: web::Data<rate::LimitStore>,
    admin_token: web::Data<AdminToken>,
) -> Result<HttpResponse> {
    match admin_token.0.as_ref() {
        None => return Err(UserError::AdminOff.into()),
        Some(t) if t != params.token.as_str() => return Err(UserError::InvalidAdminToken.into()),
        _ => (),
    }

    match params.action {
        AdminAction::PurgeDatabase => {
            let r = db::purge(&db).await?;
            log::warn!("Database purged: {} rows deleted!", r);
        }
        AdminAction::ResetLimits => {
            limits.reset();
            log::warn!("Limits reset!");
        }
        AdminAction::ListPolls => {
            log::warn!("Listing polls...");
            let polls = db::list_polls(&db).await?;
            let content = templates::PollListTemplate { polls }
                .render()
                .map_err(|e| UserError::InternalError(e.into()))?;
            return return_html!(content);
        }
        _ => return Err(UserError::InvalidAdminAction.into()),
    }

    return_html!(format!("Action executed: {:?}", params.action))
}

/// Handles the poll-specific administration webpage
pub async fn handle_poll_admin(_poll_id: web::Path<String>) -> Result<HttpResponse> {
    return_html!(include_str!("../static/poll_admin.html"))
}

/// Handles the poll-specific administration webpage callback
/// Params:
///  - token: The admin token, should match the POLL_ADMIN_TOKEN environmental variable
///  - action: an AdminAction enum member, specifies the action to be executed.
/// Only the poll-specific administration actions can be executed from here.
pub async fn handle_poll_admin_action(
    db: web::Data<DbPool>,
    poll_id: web::Path<String>,
    params: web::Form<AdminParams>,
) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;
    let mut poll = db::get_poll(&db, poll_id).await?;

    if params.token != poll.data.admin_link.as_str() {
        log::warn!(
            "Invalid poll token: {}, {}",
            params.token,
            poll.data.admin_link.as_str()
        );
        return Err(UserError::InvalidAdminToken.into());
    }

    match params.action {
        AdminAction::ResetVotes => {
            poll.data.voters = 0;
            poll.format.reset();
            db::update_poll(&db, &poll).await?;
        }
        AdminAction::DeletePoll => {
            db::delete_poll(&db, poll.data.id).await?;
        }
        _ => return Err(UserError::InvalidAdminAction.into()),
    }

    return_html!(format!("Action executed: {:?}", params.action))
}
