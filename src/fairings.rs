use std::{
    io::Cursor,
    sync::atomic::{AtomicUsize, Ordering},
};

use rocket::{
    async_trait,
    fairing::{Fairing, Info, Kind},
    http::{ContentType, Method, Status},
};

use crate::{get_hostname, get_nodename};

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

    async fn on_request(&self, _req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        self.requests_served.fetch_add(1, Ordering::Relaxed);
    }

    async fn on_response<'r>(&self, req: &'r rocket::Request<'_>, res: &mut rocket::Response<'r>) {
        if res.status() != Status::NotFound {
            return;
        }

        let count = self.requests_served.load(Ordering::Relaxed);

        if req.method() == Method::Get && req.uri().path() == "/count" {
            let body = format!(
                "Requests served from {} on node {}: {}\n",
                get_hostname(),
                get_nodename(),
                count
            );
            res.set_status(Status::Ok);
            res.set_header(ContentType::Plain);
            res.set_sized_body(body.len(), Cursor::new(body));
        }
    }
}
