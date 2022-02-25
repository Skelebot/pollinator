use actix_web::rt::{self, time};
use askama::Template;
use db::DbPool;
use poll::{Poll, PollData, PollID, PollType};
use qstring::QString;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use anyhow::{bail, Context};
use templates::ReturnTemplate;

mod db;
mod error;
use error::*;
mod poll;
mod rate;
mod templates;
mod util;

// TODO: Move somewhere
macro_rules! return_html {
    ($html:expr) => {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body($html))
    };
}

// TODO: Environmental var instead
const GENERAL_ADMIN_TOKEN: &str = "szSnAkFwtQH";

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info,polls=info");
    std::env::set_var("RUST_LOG", "info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        bail!("Database path not specified. Set the database path in the first argument");
    }
    let db_path = std::path::Path::new(&args[1]);
    if !db_path.exists() {
        bail!(
            "Database file {:?} does not exist or could not be read.",
            db_path
        );
    }

    log::info!("Connecting to database: {} ...", db_path.to_str().unwrap());

    // SQLite database connection
    let manager = SqliteConnectionManager::file(&db_path);
    let pool = db::DbPool::new(manager)?;

    log::info!("Connected to database!");

    let limits = web::Data::new(rate::LimitStore::default());
    let l = limits.clone();
    rt::spawn(async move {
        let limits = l;
        let mut interval = time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            limits.cleanup();
            log::debug!("limits cleaned up");
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(limits.clone())
            .app_data(web::Data::new(pool.clone()))
            .configure(app_config)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
    .context("An error occured when running HttpServer")
}

fn app_config(config: &mut web::ServiceConfig) {
    config
        .service(
            actix_files::Files::new("/static", "static/")
                .prefer_utf8(true)
                .show_files_listing(),
        )
        .service(
            // TODO: Convert stuff from GET requests to POST requests
            web::scope("")
                // Allow only urls with queries shorter than 560 characters
                // TODO: Review the limit
                .guard(actix_web::guard::fn_guard(|req| {
                    req.uri.query().map(|p| p.len() < 560).unwrap_or(true)
                }))
                .service(web::resource("/").name("index").to(index))
                // Poll creation screen
                .service(web::resource("/create").to(handle_create))
                // Poll creation callback
                .service(web::resource("/create/poll").to(handle_create_desc))
                // Poll voting screen
                .service(
                    web::resource("/vote/{poll_id}")
                        .name("vote")
                        .to(handle_vote),
                )
                // Poll voting callback
                .service(web::resource("/vote/{poll_id}/response").to(handle_vote_desc))
                // Poll results screen
                .service(
                    web::resource("/results/{poll_id}")
                        .name("results")
                        .to(handle_results),
                )
                // General management callback
                .service(
                    web::resource("/admin")
                        .route(web::get().to(handle_admin))
                        .route(web::post().to(handle_admin_action)),
                )
                // Poll management callback
                .service(
                    web::resource("/admin/{poll_id}")
                        .route(web::get().to(handle_poll_admin))
                        .route(web::post().to(handle_poll_admin_action)),
                )
                // 404 screen
                .default_service(web::to(handle_default)),
        );
}

/// Handes the starting webpage
async fn index() -> Result<HttpResponse> {
    return_html!(include_str!("../static/index.html"))
}

/// Handles requests that don't match anything, returns error 404
async fn handle_default() -> Result<HttpResponse> {
    return_html!(include_str!("../static/404.html"))
}

/// Handles poll creation
/// Returns a website for creating a poll.
/// If provided a poll type, returns a website for creating a poll of that type,
/// if no arguments are given returns a website for choosing a poll type.
/// Params:
///  - poll_type: PollType enum variant, determines the poll creation website,
///    see PollType::try_parse for parsing format
async fn handle_create(req: HttpRequest) -> Result<HttpResponse> {
    let query = QString::from(req.query_string());

    // TODO: choose between poll_type and ptype names for poll type param
    // If there is a poll type specified
    if let Some(ptype) = query.get("poll_type") {
        let poll_type = PollType::try_parse(ptype)?;
        let content = templates::CreateTemplate { poll_type }
            .render()
            .map_err(|e| UserError::InternalError(e.into()))?;

        return_html!(content)
    } else {
        return_html!(include_str!("../static/create.html"))
    }
}

/// Handles complete poll creation requests
/// Params: TODO: write params
async fn handle_create_desc(req: HttpRequest, db: web::Data<DbPool>) -> Result<HttpResponse> {
    // Check for rate limiting of poll creation for given IP
    if rate::limit_create(&req) {
        return Err(UserError::TooManyRequests.into());
    }
    // TODO: Remove QString dependency
    let query = QString::from(req.query_string());

    let name = query
        .get("name")
        .ok_or_else(|| UserError::MissingParam("name".to_string()))?;

    // Parse poll type
    let ptype = query
        .get("type")
        .ok_or_else(|| UserError::MissingParam("type".to_string()))?;
    let ptype = PollType::try_parse(ptype)?;

    let format = poll::create_poll_format_from_query(ptype, &query)?;

    // Generate poll ID
    let id = db::last_id(&db).await? + 1;
    let id = PollID::generate(id as u64);

    // Generate poll's admin token used to manage the poll
    let admin_token = util::random_base64_u64();
    let poll = Poll {
        data: PollData {
            id,
            ptype,
            name: name.to_string(),
            date_created: chrono::Utc::now(),
            admin_link: admin_token.clone(),
            voters: 0,
        },
        format,
    };

    log::info!("Inserting poll id: {} to database...", id);
    db::insert_poll(&db, poll).await?;

    let content = ReturnTemplate {
        heading: &format!("Poll created: {}", name),
        links: &[
            (
                "Voting link",
                req.url_for("vote", &[&id.to_string()]).unwrap().as_str(),
            ),
            (
                "Results link",
                req.url_for("results", &[&id.to_string()]).unwrap().as_str(),
            ),
            ("Admin token", admin_token.as_str()),
        ],
    }
    .render()
    .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

async fn handle_vote(
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;

    let poll = db::get_poll(&db, poll_id).await?;
    let content = poll
        .format
        .voting_site(&poll.data)
        .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

async fn handle_vote_desc(
    req: HttpRequest,
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;
    if rate::limit_vote(&req, poll_id) {
        return Err(UserError::TooManyRequests.into());
    }

    let mut poll = db::get_poll(&db, poll_id).await?;

    let query = QString::from(req.query_string());

    poll.format
        .register_votes(&query)
        .map_err(UserError::InternalError)?;
    poll.data.voters += 1;

    db::update_poll(&db, &poll).await?;

    let content = ReturnTemplate {
        heading: "Voted.",
        links: &[(
            "See results",
            req.url_for("results", &[poll_id.to_string()])
                .unwrap()
                .as_str(),
        )],
    }
    .render()
    .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

/// Returns the results website for a given poll ID
async fn handle_results(
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;

    let poll = db::get_poll(&db, poll_id).await?;

    let content = poll
        .format
        .results_site(&poll.data)
        .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

// TODO: move somewhere
#[derive(Deserialize, Debug)]
pub enum AdminAction {
    PurgeDatabase,
    ResetLimits,
    ResetVotes,
    DeletePoll,
}

#[derive(Deserialize)]
struct AdminParams {
    token: String,
    action: AdminAction,
}

async fn handle_admin() -> Result<HttpResponse> {
    return_html!(include_str!("../static/admin.html"))
}

async fn handle_admin_action(
    db: web::Data<DbPool>,
    params: web::Form<AdminParams>,
    limits: web::Data<rate::LimitStore>,
) -> Result<HttpResponse> {
    if params.token != GENERAL_ADMIN_TOKEN {
        return Err(UserError::InvalidAdminToken.into());
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
        _ => (), // TODO
    }

    return_html!(format!("Action executed: {:?}", params.action))
}

async fn handle_poll_admin(web::Path(_poll_id): web::Path<String>) -> Result<HttpResponse> {
    return_html!(include_str!("../static/poll_admin.html"))
}

async fn handle_poll_admin_action(
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
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
        _ => (), // TODO
    }

    return_html!(format!("Action executed: {:?}", params.action))
}
