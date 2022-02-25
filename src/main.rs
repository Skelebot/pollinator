use actix_web::{
    dev::Service,
    rt::{self, time},
};
use askama::Template;
use db::DbPool;
use futures::future::{self, Either};
use poll::{Poll, PollData, PollID, PollType};
use qstring::QString;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use templates::ReturnTemplate;

mod db;
mod poll;
mod rate;
mod templates;
mod util;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    std::env::set_var("RUST_LOG", "actix_web=info,polls=info");
    env_logger::init();
    log::info!("Polls started!");

    // SQLite database connection
    let manager = SqliteConnectionManager::file(args[1].as_str());
    let pool = db::DbPool::new(manager).unwrap();

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
}

fn app_config(config: &mut web::ServiceConfig) {
    config
        .service(
            actix_files::Files::new("/static", "static/")
                .prefer_utf8(true)
                .show_files_listing(),
        )
        .service(
            web::scope("")
                .service(web::resource("/").name("index").to(index))
                // Poll creation screen
                .service(web::resource("/create").to(handle_create))
                // Poll creation callback
                .service(
                    web::resource("/create/poll")
                        .wrap_fn(|req, srv| {
                            // todo: rate limit only if address.is_global()
                            if let Err(r) = rate::limit_create(&req) {
                                return Either::Left(future::ready(Ok(req.into_response(r))));
                            }
                            Either::Right(srv.call(req))
                        })
                        .to(handle_create_desc),
                )
                // Poll voting screen
                .service(
                    web::resource("/vote/{poll_id}")
                        .name("vote")
                        .to(handle_vote),
                )
                // Poll voting callback
                .service(
                    web::resource("/vote/{poll_id}/response")
                        .wrap_fn(|req, srv| {
                            // todo: rate limit only if address.is_global()
                            if let Err(r) = rate::limit_vote(&req) {
                                return Either::Left(future::ready(Ok(req.into_response(r))));
                            }
                            Either::Right(srv.call(req))
                        })
                        .to(handle_vote_desc),
                )
                // Poll results screen
                .service(
                    web::resource("/results/{poll_id}")
                        .name("results")
                        .to(handle_results),
                )
                // 404 screen
                .default_service(web::to(handle_default)),
        );
}

async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html")))
}

async fn handle_default() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/404.html")))
}

#[derive(Deserialize, Debug)]
struct CreateParams {
    poll_type: Option<String>,
}

async fn handle_create(web::Query(params): web::Query<CreateParams>) -> Result<HttpResponse> {
    if let Some(ptype) = params.poll_type {
        let poll_type = PollType::try_parse(&ptype).map_err(|_| HttpResponse::BadRequest())?;
        let content = templates::CreateTemplate { poll_type }
            .render()
            .map_err(|_| HttpResponse::InternalServerError())?;

        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content))
    } else {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/create.html")))
    }
}

async fn handle_create_desc(req: HttpRequest, db: web::Data<DbPool>) -> Result<HttpResponse> {
    if req.query_string().len() > 560 {
        return Ok(HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body("Query string length must not exceed 560 characters."));
    }
    let query = QString::from(req.query_string());

    let ptype = query.get("type").ok_or_else(HttpResponse::BadRequest)?;
    let ptype = PollType::try_parse(ptype).map_err(|_| HttpResponse::BadRequest())?;
    let name = query.get("name").ok_or_else(HttpResponse::BadRequest)?;
    //if name.len() > 200 { return Ok(HttpResponse::BadRequest().body("")) }

    let format = poll::create_poll_format_from_query(ptype, &query)?;

    let id = db::last_id(&db)
        .await
        .map_err(|_| HttpResponse::InternalServerError())?;
    let id = PollID::generate(id as u64 + 1);
    let poll = Poll {
        data: PollData {
            id,
            ptype,
            name: name.to_string(),
            date_created: chrono::Utc::now(),
            admin_link: util::random_base64_u64(),
            voters: 0,
        },
        format,
    };

    log::info!("Creating poll...");
    db::insert_poll(&db, poll)
        .await
        .map_err(|_| HttpResponse::BadRequest())?;

    let body = ReturnTemplate {
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
        ],
    }
    .render()
    .map_err(|_| HttpResponse::InternalServerError())?;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

async fn handle_vote(
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let poll_id: PollID =
        PollID::try_from(poll_id.as_str()).map_err(|_| HttpResponse::BadRequest())?;

    let poll = db::get_poll(&db, poll_id)
        .await
        .map_err(|_| HttpResponse::BadRequest())?;
    let content = poll
        .format
        .voting_site(&poll.data)
        .map_err(|_| HttpResponse::InternalServerError())?;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(content))
}

async fn handle_vote_desc(
    req: HttpRequest,
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    if req.query_string().len() > 560 {
        return Ok(HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body("Query string length must not exceed 560 characters."));
    }
    let poll_id: PollID =
        PollID::try_from(poll_id.as_str()).map_err(|_| HttpResponse::BadRequest())?;

    let mut poll = db::get_poll(&db, poll_id)
        .await
        .map_err(|_| HttpResponse::BadRequest())?;

    let query = QString::from(req.query_string());

    poll.format.register_votes(&query).map_err(|e| {
        HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body(e)
    })?;

    db::update_poll(&db, &poll)
        .await
        .map_err(|_| HttpResponse::BadRequest())?;

    let body = ReturnTemplate {
        heading: "Voted.",
        links: &[(
            "See results",
            req.url_for("results", &[poll_id.to_string()])
                .unwrap()
                .as_str(),
        )],
    }
    .render()
    .map_err(|_| HttpResponse::InternalServerError())?;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

async fn handle_results(
    db: web::Data<DbPool>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let poll_id: PollID =
        PollID::try_from(poll_id.as_str()).map_err(|_| HttpResponse::BadRequest())?;

    let poll = db::get_poll(&db, poll_id)
        .await
        .map_err(|_| HttpResponse::BadRequest())?;

    let content = poll
        .format
        .results_site(&poll.data)
        .map_err(|_| HttpResponse::InternalServerError())?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(content))
}

fn bad_poll_id_page(id: impl std::fmt::Display) -> Result<HttpResponse> {
    let body = ReturnTemplate {
        heading: &format!("Poll id {} does not exist", id),
        links: &[],
    }
    .render()
    .map_err(|_| HttpResponse::InternalServerError())?;

    Ok(HttpResponse::BadRequest()
        .content_type("text/html; charset=utf-8")
        .body(body))
}
