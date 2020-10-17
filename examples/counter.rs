extern crate mini_async_http;

use mini_async_http::AIOServer;
use mini_async_http::ResponseBuilder;
use std::sync::{Arc, Mutex};

pub fn main() {
    let counter = Arc::from(Mutex::from(0));

    let mut server = AIOServer::new("0.0.0.0:7878".parse().unwrap(), move |_request| {
        let lock = counter.clone();
        let mut counter = lock.lock().unwrap();

        let body = counter.to_string();
        *counter += 1;

        ResponseBuilder::empty_200()
            .body(body.as_bytes())
            .content_type("text/plain")
            .build()
            .unwrap()
    });

    server.start();
}
