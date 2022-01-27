use std::{sync::Mutex, vec};

use askama::Template;
use poll::PollType;
use serde::Deserialize;

use actix_web::{
    get, middleware, web, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Result,
};

mod poll;
mod templates;

struct AppState {
    polls: Mutex<Vec<poll::Poll>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info,polls=info");
    env_logger::init();
    log::info!("Polls started!");

    let state = web::Data::new(AppState {
        polls: Mutex::new(vec![]),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(state.clone())
            .configure(app_config)
    })
    .bind("127.0.0.1:8080")?
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
                .service(web::resource("/").to(index))
                .service(handle_create)
                .service(handle_create_desc)
                .service(handle_vote)
                .service(handle_vote_desc)
                .service(handle_results),
        );
}

async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html")))
}

#[derive(Deserialize, Debug)]
struct CreateParams {
    poll_type: Option<String>,
}

#[get("/create")]
async fn handle_create(web::Query(params): web::Query<CreateParams>) -> Result<HttpResponse> {
    if let Some(ptype) = params.poll_type {
        let poll_type = PollType::try_parse(&ptype).map_err(|_| {
            HttpResponse::BadRequest()
                .content_type("text/plain")
                .body("Invalid poll type")
        })?;
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

#[derive(Deserialize, Debug)]
struct PollDesc {
    ptype: String,
    name: String,
    options: String,
}

#[get("/create/poll")]
async fn handle_create_desc(
    state: web::Data<AppState>,
    web::Query(desc): web::Query<PollDesc>,
) -> Result<HttpResponse> {
    // TODO: Rate limiting here
    let poll = poll::Poll {
        name: desc.name,
        ptype: PollType::try_parse(&desc.ptype).map_err(|_| HttpResponse::BadRequest())?,
        voters: 0,
        options: desc.options.split(',').map(|opt| (opt.into(), 0)).collect(),
    };

    let mut polls = state.polls.lock().unwrap();
    polls.push(poll);

    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(&format!("poll num: {}", polls.len() - 1)))
}

#[get("/vote/{poll_id}")]
async fn handle_vote(
    state: web::Data<AppState>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let polls = state
        .polls
        .lock()
        .map_err(|_| HttpResponse::InternalServerError())?;
    let idx: usize = poll_id.parse().map_err(|_| HttpResponse::BadRequest())?;

    if let Some(poll) = polls.get(idx) {
        let content = poll
            .vote_template(idx)
            .map_err(|_| HttpResponse::InternalServerError())?;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content))
    } else {
        Ok(HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body(&format!("poll num {} does not exist", idx)))
    }
}

#[derive(Deserialize, Debug)]
struct VoteResponseDesc(Vec<usize>);

impl actix_web::FromRequest for VoteResponseDesc {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        use std::future::ready;

        ready(|| -> Result<Self, Self::Error> {
            let query = req.query_string();
            let options: Vec<usize> = query
                .split('&')
                .map(Self::parse_option)
                .collect::<Result<Vec<usize>, _>>()
                .map_err(|_| HttpResponse::BadRequest())?;
            if options.is_empty() {
                return Err(HttpResponse::BadRequest().into());
            }

            Ok(VoteResponseDesc(options))
        }())
    }
}

impl VoteResponseDesc {
    fn parse_option(opt: &str) -> Result<usize, actix_web::Error> {
        let eq = opt.find('=').ok_or_else(HttpResponse::BadRequest)?;
        let opt_num: usize = opt[eq + 1..]
            .parse()
            .map_err(|_| HttpResponse::BadRequest())?;
        Ok(opt_num)
    }
}

#[get("/vote/{poll_id}/response")]
async fn handle_vote_desc(
    req: HttpRequest,
    state: web::Data<AppState>,
    web::Path(poll_id): web::Path<usize>,
) -> Result<HttpResponse> {
    // TODO: Rate limiting here

    let mut polls = state
        .polls
        .lock()
        .map_err(|_| HttpResponse::InternalServerError())?;

    if let Some(poll) = polls.get_mut(poll_id) {
        let desc = VoteResponseDesc::from_request(&req, &mut actix_web::dev::Payload::None).await?;
        poll.voters += 1;
        match poll.ptype {
            PollType::Single => {
                let opt = poll
                    .options
                    .get_mut(desc.0[0])
                    .ok_or_else(HttpResponse::BadRequest)?;
                opt.1 += 1;
            }
            PollType::Multiple => {
                for option in desc.0 {
                    let opt = poll
                        .options
                        .get_mut(option)
                        .ok_or_else(HttpResponse::BadRequest)?;
                    opt.1 += 1;
                }
            }
            PollType::Ranked(system) => {
                for (idx, option) in desc.0.iter().enumerate() {
                    let opt = poll
                        .options
                        .get_mut(idx)
                        .ok_or_else(HttpResponse::BadRequest)?;
                    let max = desc.0.len();
                    match system {
                        poll::PositionalSystem::Borda => opt.1 += (max - (option+1)) as u64,
                        poll::PositionalSystem::Dowdall => todo!(),
                        poll::PositionalSystem::Score(_) => todo!(),
                    }
                }
            }
        }

        Ok(HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body("Voted."))
    } else {
        Ok(HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body(&format!("poll num {} does not exist", poll_id)))
    }
}

#[get("/results/{poll_id}")]
async fn handle_results(
    state: web::Data<AppState>,
    web::Path(poll_id): web::Path<String>,
) -> Result<HttpResponse> {
    let polls = state
        .polls
        .lock()
        .map_err(|_| HttpResponse::InternalServerError())?;
    let idx: usize = poll_id.parse().map_err(|_| HttpResponse::BadRequest())?;

    if let Some(poll) = polls.get(idx) {
        let content = poll
            .results_template(idx)
            .map_err(|_| HttpResponse::InternalServerError())?;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content))
    } else {
        Ok(HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body(&format!("poll num {} does not exist", idx)))
    }
}
