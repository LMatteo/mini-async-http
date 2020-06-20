extern crate chrono;
extern crate mio;
extern crate regex;
#[macro_use] extern crate log;

mod aioserver;
mod http;
mod request;
mod response;

use crate::aioserver::AIOServer;
use crate::http::Headers;
use crate::http::Method;
use crate::http::Version;
use crate::request::Request;
use crate::response::{Response, ResponseBuilder};


fn main() {
    env_logger::init();
    
    let server = AIOServer::new(16, "0.0.0.0:7878", |request|{
        let mut builder = ResponseBuilder::new_builder();
        builder
            .set_code(200)
            .set_reason(String::from("OK"))
            .set_version(Version::HTTP11)
            .set_body(String::from(request.method.as_str()))
            .set_header("Content-Type", "text/plain")
            .set_header(
                "Content-Length",
                request.method.as_str().len().to_string().as_str(),
            );

        let response = builder.build().unwrap();

        return response;
    });

    server.start();
}
