// SPDX-License-Identifier: MIT
//
// Distributed printer facts in Rust, inspired by Christine Dodrill.
// Copyright (c) 2021  William Findlay
//
// September 25, 2021  William Findlay  Created this.

use std::{
    io::Cursor,
    sync::atomic::{AtomicUsize, Ordering},
};

use rocket::{
    async_trait,
    fairing::{Fairing, Info, Kind},
    http::{ContentType, Method, Status},
};
use serde::Serialize;
use serde_json::to_string;

use crate::get_hostname;

#[derive(Serialize)]
pub struct CounterStats {
    server: String,
    counter: usize,
}

/// Counts how many requests have been served from a given pod.
#[derive(Default)]
pub struct Counter {
    requests_served: AtomicUsize,
}

#[async_trait]
impl Fairing for Counter {
    fn info(&self) -> Info {
        Info {
            name: "Request Counter",
            kind: Kind::Request | Kind::Response,
        }
    }

    /// Increment request count when receiving a request.
    async fn on_request(&self, _req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        self.requests_served.fetch_add(1, Ordering::Relaxed);
    }

    /// Return request count on GET request to endpoint /count
    async fn on_response<'r>(&self, req: &'r rocket::Request<'_>, res: &mut rocket::Response<'r>) {
        if res.status() != Status::NotFound {
            return;
        }

        let count = self.requests_served.load(Ordering::Relaxed);

        if req.method() == Method::Get && req.uri().path() == "/count" {
            let body = to_string(&CounterStats {
                server: get_hostname().await,
                counter: count.into(),
            })
            .unwrap();
            res.set_status(Status::Ok);
            res.set_header(ContentType::JSON);
            res.set_sized_body(body.len(), Cursor::new(body));
        }
    }
}
