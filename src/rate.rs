use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use actix_web::dev::ServiceRequest;
use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::poll::PollID;

#[derive(Default)]
pub struct LimitMap(HashMap<IpAddr, Instant>);

impl Deref for LimitMap {
    type Target = HashMap<IpAddr, Instant>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for LimitMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct LimitStore {
    pub create: Mutex<HashMap<IpAddr, Instant>>,
    // TODO: optimize (usize instead of PollID)
    pub vote: Mutex<HashMap<(IpAddr, PollID), Instant>>,
}

impl LimitStore {
    // a single IP must wait 10 minutes before creating a new poll
    const CREATE_LIMIT: Duration = Duration::from_secs(10 * 60);
    // a single IP must wait 30 minutes before voting again
    const VOTE_LIMIT: Duration = Duration::from_secs(30 * 60);

    // Called periodically to clean up now-irrelevant limits
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.create
            .lock()
            .unwrap()
            .retain(|_, v| now - *v <= Self::CREATE_LIMIT);
        self.vote
            .lock()
            .unwrap()
            .retain(|_, v| now - *v <= Self::VOTE_LIMIT);
    }

    // Returns true if the address should be rate-limited;
    // inserts it otherwise
    pub fn check_create(&self, addr: IpAddr) -> bool {
        let mut limits = self.create.lock().unwrap();
        let now = Instant::now();
        if let Some(instant) = limits.get(&addr) {
            if now - *instant < Self::CREATE_LIMIT {
                true
            } else {
                limits.remove(&addr);
                false
            }
        } else {
            limits.insert(addr, now);
            false
        }
    }

    // Returns true if the address should be rate-limited;
    // inserts it otherwise
    pub fn check_vote(&self, addr: IpAddr, poll_id: PollID) -> bool {
        let mut limits = self.vote.lock().unwrap();
        let now = Instant::now();
        if let Some(instant) = limits.get(&(addr, poll_id)) {
            if now - *instant < Self::VOTE_LIMIT {
                true
            } else {
                limits.remove(&(addr, poll_id));
                false
            }
        } else {
            limits.insert((addr, poll_id), now);
            false
        }
    }
}

pub fn limit_create(req: &ServiceRequest) -> Result<(), actix_web::HttpResponse> {
    // todo: rate limit only if address.is_global()
    let addr = req.peer_addr();
    let addr = if let Some(addr) = addr {
        addr.ip()
    } else {
        return Ok(());
    };
    if addr.is_loopback() {
        return Ok(());
    }
    let store = &req.app_data::<web::Data<LimitStore>>().unwrap();

    if store.check_create(addr) {
        Err(HttpResponse::TooManyRequests()
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/limit.html")))
    } else {
        Ok(())
    }
}

pub fn limit_vote(req: &ServiceRequest) -> Result<(), actix_web::HttpResponse> {
    let poll_id = req.match_info().query("poll_id");
    let id = PollID::try_from(poll_id).map_err(|_| crate::bad_poll_id_page(poll_id))?;

    // todo: rate limit only if address.is_global()
    let addr = req.peer_addr();
    let addr = if let Some(addr) = addr {
        addr.ip()
    } else {
        return Ok(());
    };
    if addr.is_loopback() {
        return Ok(());
    }

    let store = &req.app_data::<web::Data<LimitStore>>().unwrap();

    if store.check_vote(addr, id) {
        Err(HttpResponse::TooManyRequests()
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/limit.html")))
    } else {
        Ok(())
    }
}
