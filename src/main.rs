extern crate chrono;
extern crate mio;
extern crate regex;
#[macro_use]
extern crate log;
extern crate httparse;

mod aioserver;
mod http;
mod request;
mod response;

use crate::aioserver::AIOServer;

use crate::http::Version;

use crate::response::ResponseBuilder;

fn main() {
    env_logger::init();

    let server = AIOServer::new(16, "0.0.0.0:7878", |request| {
        let builder = ResponseBuilder::new()
            .code(200)
            .reason(String::from("OK"))
            .version(Version::HTTP11)
            .body(request.method().as_str().to_string().as_bytes().to_vec())
            .header("Content-Type", "text/plain")
            .header(
                "Content-Length",
                request.method().as_str().len().to_string().as_str(),
            );

        let response = builder.build().unwrap();

        return response;
    });

    server.start();
}
