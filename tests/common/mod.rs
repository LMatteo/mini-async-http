use mini_async_http::{router, AIOServer, Method, Request, Response, ResponseBuilder, Version};

use std::sync::Mutex;

extern crate lazy_static;
use lazy_static::lazy_static;

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
    pub fn server(&self) -> (AIOServer, ServerConfig) {
        let portstr = self.incr().to_string();

        let server = server(portstr.as_str());

        let config = ServerConfig {
            addr: addr(portstr.as_str()),
            http_addr: http_addr(portstr.as_str()),
        };

        (server, config)
    }

    pub fn routed_server(&self) -> (AIOServer, ServerConfig) {
        let portstr = self.incr().to_string();

        let server = router_server(portstr.as_str());

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

fn server(port: &str) -> AIOServer {
    let addr = format!("127.0.0.1:{}", port);
    AIOServer::new(addr.as_str().parse().unwrap(), Box::new(handler_basic))
}

fn router_server(port: &str) -> AIOServer {
    let addr = format!("127.0.0.1:{}", port);

    let router = router!(
        "/router/parametrized/{parameter}", Method::GET => |_req, params| {
            let val = params.get("parameter").unwrap();
            let len = val.as_bytes().len();

            let builder = ResponseBuilder::new()
                .code(200)
                .reason(String::from("OK"))
                .version(Version::HTTP11)
                .body(val.as_bytes())
                .header("Content-Type", "text/plain")
                .header("Content-Length", &len.to_string());

            let response = builder.build().unwrap();

            return response;
        },
        "/router/post", Method::POST => |_req, _| {
            let builder = ResponseBuilder::new()
                .code(200)
                .reason(String::from("OK"))
                .version(Version::HTTP11)
                .body(b"POST")
                .header("Content-Type", "text/plain")
                .header("Content-Length", "4");

            let response = builder.build().unwrap();

            return response;
        },
        "/router/get", Method::GET => |_req, _| {
            let builder = ResponseBuilder::new()
                .code(200)
                .reason(String::from("OK"))
                .version(Version::HTTP11)
                .body(b"GET")
                .header("Content-Type", "text/plain")
                .header("Content-Length", "3");

            let response = builder.build().unwrap();

            return response;
        }
    );

    AIOServer::from_router(addr.as_str().parse().unwrap(), router)
}

fn addr(port: &str) -> String {
    format!("127.0.0.1:{}", port)
}

fn http_addr(port: &str) -> String {
    format!("http://{}", addr(port))
}

pub fn run_test<T>(test: T) -> ()
where
    T: FnOnce(ServerConfig) -> () + std::panic::UnwindSafe,
{
    let (mut server, config) = GENERATOR.server();
    let handle = server.handle();
    std::thread::spawn(move || {
        server.start();
    });

    handle.ready();

    let result = std::panic::catch_unwind(|| test(config));

    handle.shutdown();

    assert!(result.is_ok())
}

pub fn run_test_routed_server<T>(test: T) -> ()
where
    T: FnOnce(ServerConfig) -> () + std::panic::UnwindSafe,
{
    let (mut server, config) = GENERATOR.routed_server();
    let handle = server.handle();
    std::thread::spawn(move || {
        server.start();
    });

    handle.ready();

    let result = std::panic::catch_unwind(|| test(config));

    handle.shutdown();

    assert!(result.is_ok())
}
