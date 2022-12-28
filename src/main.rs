use actix_web::rt::{self, time};
use askama::Template;
use db::DbPool;
use poll::{Poll, PollData, PollID, PollType};
use qstring::QString;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use anyhow::{bail, Context};
use std::time::Duration;

mod db;
mod error;
use error::*;

use crate::templates::{PollCreatedTemplate, VotedTemplate};
mod poll;
mod rate;
mod templates;
#[macro_use]
mod util;

mod admin;

// TODO: cmd options
const RATE_LIMIT_CLEANUP_DURATION: Duration = Duration::from_secs(30);
const BIND_ADDRESS: &str = "0.0.0.0:8080";

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info");
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

    // Read the admin token from envars
    let admin_token = std::env::var("POLL_ADMIN_TOKEN").ok();
    if admin_token.is_none() {
        log::warn!("Environment variable POLL_ADMIN_TOKEN not set - admin functions off.");
    }

    // SQLite database connection
    log::info!("Connecting to database: {} ...", db_path.to_str().unwrap());
    let manager = SqliteConnectionManager::file(&db_path);
    let pool = db::DbPool::new(manager)?;

    log::info!("Connected to database!");

    // Create the rate limit store and a thread that periodically
    // checks and cleans up expired limits.
    let limits = web::Data::new(rate::LimitStore::default());
    let l = limits.clone();
    rt::spawn(async move {
        let limits = l;
        let mut interval = time::interval(RATE_LIMIT_CLEANUP_DURATION);
        loop {
            interval.tick().await;
            limits.cleanup();
            log::debug!("Rate limits cleaned up");
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(limits.clone())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(admin::AdminToken(admin_token.clone())))
            .configure(app_config)
    })
    .bind(BIND_ADDRESS)?
    .run()
    .await
    .context("An error occured when running HttpServer")
}

fn app_config(config: &mut web::ServiceConfig) {
    config
        .service(
            actix_files::Files::new("/static", "static/")
                .prefer_utf8(true)
                .index_file("index.html"),
        )
        .service(
            web::scope("")
                // Allow only urls with queries shorter than 560 characters
                .guard(actix_web::guard::fn_guard(|req| {
                    if let Some(query) = req.head().uri.query() {
                        query.len() < 560
                    } else {
                        true
                    }
                }))
                .service(web::resource("/").name("index").to(index))
                .service(
                    web::resource("/create")
                        // Poll creation screen
                        .route(web::get().to(handle_create))
                        // Poll creation callback
                        .route(web::post().to(handle_create_desc)),
                )
                .service(
                    web::resource("/vote/{poll_id}")
                        .name("vote")
                        // Poll voting screen
                        .route(web::get().to(handle_vote))
                        // Poll voting callback
                        .route(web::post().to(handle_vote_desc)),
                )
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
                        .route(web::get().to(admin::handle_admin))
                        .route(web::post().to(admin::handle_admin_action)),
                )
                // Poll management callback
                .service(
                    web::resource("/admin/{poll_id}")
                        .name("admin")
                        .route(web::get().to(admin::handle_poll_admin))
                        .route(web::post().to(admin::handle_poll_admin_action)),
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

#[derive(Deserialize)]
struct CreateParams {
    name: String,
    r#type: String,
    data: String,
}

/// Handles complete poll creation requests
/// Params:
///  - name: Poll name (String)
///  - type: PollType enum variant, see PollType::try_parse for parsing format
///  - data: To be parsed as a PollFormat trait object, see the corresponding
///    PollType::from_data function for the proper format
async fn handle_create_desc(
    req: HttpRequest,
    params: web::Form<CreateParams>,
    db: web::Data<DbPool>,
) -> Result<HttpResponse> {
    // Check for rate limiting of poll creation for given IP
    if rate::limit_create(&req) {
        return Err(UserError::TooManyRequests.into());
    }

    let name = &params.name;
    let ptype = PollType::try_parse(&params.r#type)?;
    let data = params.data.as_str();

    let format = poll::create_poll_format_from_data(ptype, data)?;

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

    let content = PollCreatedTemplate {
        name,
        voting_link: req.url_for("vote", &[&id.to_string()]).unwrap().as_str(),
        results_link: req.url_for("results", &[&id.to_string()]).unwrap().as_str(),
        admin_link: req.url_for("admin", &[&id.to_string()]).unwrap().as_str(),
        admin_token: admin_token.as_str(),
    }
    .render()
    .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

async fn handle_vote(db: web::Data<DbPool>, poll_id: web::Path<String>) -> Result<HttpResponse> {
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
    poll_id: web::Path<String>,
    params: String,
) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;
    if rate::limit_vote(&req, poll_id) {
        return Err(UserError::TooManyRequests.into());
    }

    let mut poll = db::get_poll(&db, poll_id).await?;

    //let query = QString::from(req.query_string());
    let query = QString::from(QString::from(params.as_str()));

    poll.format
        .register_votes(&query)
        .map_err(UserError::InternalError)?;
    poll.data.voters += 1;

    db::update_poll(&db, &poll).await?;

    let content = VotedTemplate {
        results_link: req
            .url_for("results", &[poll_id.to_string()])
            .unwrap()
            .as_str(),
    }
    .render()
    .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}

/// Returns the results website for a given poll ID
async fn handle_results(db: web::Data<DbPool>, poll_id: web::Path<String>) -> Result<HttpResponse> {
    let poll_id: PollID = PollID::try_from(poll_id.as_str())?;

    let poll = db::get_poll(&db, poll_id).await?;

    let content = poll
        .format
        .results_site(&poll.data)
        .map_err(|e| UserError::InternalError(e.into()))?;

    return_html!(content)
}
