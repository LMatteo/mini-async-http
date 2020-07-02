use mini_async_http::{
    AIOServer, Headers, Method, Request, RequestBuilder, Response, ResponseBuilder, Version,
};

use std::sync::Mutex;

extern crate lazy_static;
use lazy_static::lazy_static;

pub type Handler = Box<dyn Send + Sync + 'static + Fn(&Request) -> Response>;

pub struct ServerConfig {
    pub addr: String,
    pub http_addr: String,
}

impl Clone for ServerConfig {
    fn clone(&self) -> ServerConfig {
        ServerConfig {
            addr: self.addr.clone(),
            http_addr: self.http_addr.clone(),
        }
    }
}

pub struct ServerGenerator {
    port: Mutex<u32>,
}

lazy_static! {
    static ref GENERATOR: ServerGenerator = {
        ServerGenerator {
            port: Mutex::from(12343),
        }
    };
}

impl ServerGenerator {
    pub fn server(&self) -> (AIOServer<Handler>, ServerConfig) {
        let portstr = self.incr().to_string();

        let server = server(portstr.as_str());

        let config = ServerConfig {
            addr: addr(portstr.as_str()),
            http_addr: http_addr(portstr.as_str()),
        };

        (server, config)
    }

    fn incr(&self) -> u32 {
        let mut port = self.port.lock().unwrap();
        let val = *port;

        *port = *port + 1;
        val
    }
}

pub fn handler_basic(request: &Request) -> Response {
    let body = request.method().as_str().to_string().as_bytes().to_vec();

    let builder = ResponseBuilder::new()
        .code(200)
        .reason(String::from("OK"))
        .version(Version::HTTP11)
        .body(&body)
        .header("Content-Type", "text/plain")
        .header(
            "Content-Length",
            request.method().as_str().len().to_string().as_str(),
        );

    let response = builder.build().unwrap();

    return response;
}

fn server(port: &str) -> AIOServer<Handler> {
    let addr = format!("127.0.0.1:{}", port);
    AIOServer::new(1, addr.as_str(), Box::new(handler_basic))
}

fn addr(port: &str) -> String {
    format!("0.0.0.0:{}", port)
}

fn http_addr(port: &str) -> String {
    format!("http://{}", addr(port))
}

pub fn request() -> Request {
    let _body = String::from("TEST BODY").as_bytes().to_vec();

    let mut headers = Headers::new();
    headers.set_header(&String::from("content-length"), &String::from("9"));

    RequestBuilder::new()
        .method(Method::GET)
        .path(String::from("/"))
        .version(Version::HTTP11)
        .headers(headers)
        .body(b"TEST BODY")
        .build()
        .unwrap()
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce(ServerConfig) -> () + std::panic::UnwindSafe,
{
    println!("TEST STARTED");
    let (mut server, config) = GENERATOR.server();
    let handle = server.handle();
    println!("SERVER STARTED");
    std::thread::spawn(move || {
        server.start();
    });

    handle.ready();

    let result = std::panic::catch_unwind(|| test(config));

    handle.shutdown();

    assert!(result.is_ok())
}
