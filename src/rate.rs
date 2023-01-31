use std::net::IpAddr;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use actix_web::{web, HttpRequest};
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
    pub vote: Mutex<HashMap<(IpAddr, usize), Instant>>,
    create_limit: Duration,
    vote_limit: Duration,
}

impl LimitStore {
    pub fn new(create_limit: Duration, vote_limit: Duration) -> Self {
        LimitStore {
            create_limit,
            vote_limit,
            ..Default::default()
        }
    }

    // Called periodically to clean up now-irrelevant limits
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.create
            .lock()
            .unwrap()
            .retain(|_, v| now - *v <= self.create_limit);
        self.vote
            .lock()
            .unwrap()
            .retain(|_, v| now - *v <= self.vote_limit);
    }

    /// Resets all limits
    pub fn reset(&self) {
        self.create.lock().unwrap().clear();
        self.vote.lock().unwrap().clear();
    }

    // Returns true if the address should be rate-limited;
    // inserts it otherwise
    pub fn check_create(&self, addr: IpAddr) -> bool {
        let mut limits = self.create.lock().unwrap();
        let now = Instant::now();
        if let Some(instant) = limits.get(&addr) {
            if now - *instant < self.create_limit {
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
        if let Some(instant) = limits.get(&(addr, poll_id.index())) {
            if now - *instant < self.vote_limit {
                true
            } else {
                limits.remove(&(addr, poll_id.index()));
                false
            }
        } else {
            limits.insert((addr, poll_id.index()), now);
            false
        }
    }
}

/// Checks whether a request's address should be rate-limited
pub fn limit_create(req: &HttpRequest) -> bool {
    let addr = req.peer_addr();
    let addr = if let Some(addr) = addr {
        addr.ip()
    } else {
        // TODO: error?
        return false;
    };
    if addr.is_loopback() {
        return false;
    }
    // rate limit only if the address is globally routable (currently unstable)
    // if(!addr.is_global()) { return false; }
    let store = &req.app_data::<web::Data<LimitStore>>().unwrap();

    store.check_create(addr)
}

/// Checks whether a request's address should be rate-limited
/// PollID must be valid.
pub fn limit_vote(req: &HttpRequest, poll_id: PollID) -> bool {
    let addr = req.peer_addr();
    let addr = if let Some(addr) = addr {
        addr.ip()
    } else {
        // TODO: error?
        return false;
    };
    if addr.is_loopback() {
        return false;
    }
    // rate limit only if the address is globally routable (currently unstable)
    // if(!addr.is_global()) { return false; }
    let store = &req.app_data::<web::Data<LimitStore>>().unwrap();

    store.check_vote(addr, poll_id)
}
