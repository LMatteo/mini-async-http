extern crate mini_async_http;

use mini_async_http::AIOServer;
use mini_async_http::ResponseBuilder;

pub fn main() {
    let mut server = AIOServer::new(3, "0.0.0.0:7878", move |request| {
        ResponseBuilder::empty_200()
            .body(b"Hello")
            .content_type("text/plain")
            .build()
            .unwrap()
    });

    server.start();
}
