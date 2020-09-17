use mini_async_http::Request;

use std::net::TcpStream;

mod common;

use common::*;

enum Job {
    Request(Request),
    DefaultRequest,
    Stop,
}

#[test]
fn simple_get_request() {
    run_test(|config| {
        let mut writer = Vec::new();
        let _res = http_req::request::get(config.http_addr.as_str(), &mut writer).unwrap();

        let body = std::str::from_utf8(&writer).unwrap();

        assert_eq!("GET", body);
    })
}

#[test]
fn simple_post_request() {
    run_test(|config| {
        let mut writer = Vec::new();
        let body = b"RequestBody";
        let _res = http_req::request::post(config.http_addr.as_str(), body, &mut writer).unwrap();

        let body = std::str::from_utf8(&writer).unwrap();

        assert_eq!("POST", body);
    })
}

#[test]
fn multiple_get() {
    run_test(|config| {
        let mut handles = Vec::new();

        const NB_PARALLEL: i8 = 20;
        const NB_REQUEST: i8 = 20;

        for _i in 0..NB_PARALLEL {
            let config = config.clone();
            handles.push(std::thread::spawn(move || {
                let addr = config.http_addr.as_str();
                let uri: http_req::uri::Uri = addr.parse().unwrap();
                let mut stream =
                    TcpStream::connect((uri.host().unwrap(), uri.corr_port())).unwrap();

                for _i in 0..NB_REQUEST {
                    let mut writer = Vec::new();

                    let _response = http_req::request::RequestBuilder::new(&uri)
                        .method(http_req::request::Method::GET)
                        .header("Connection", "Keep-Alive")
                        .send(&mut stream, &mut writer)
                        .unwrap();

                    let body = std::str::from_utf8(&writer).unwrap();
                    assert_eq!("GET", body);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    })
}

#[test]
fn multiple_post() {
    run_test(|config| {
        let mut handles = Vec::new();

        const NB_PARALLEL: i8 = 20;
        const NB_REQUEST: i8 = 20;

        for _i in 0..NB_PARALLEL {
            let config = config.clone();
            handles.push(std::thread::spawn(move || {
                let addr = config.http_addr.as_str();
                let uri: http_req::uri::Uri = addr.parse().unwrap();
                let mut stream =
                    TcpStream::connect((uri.host().unwrap(), uri.corr_port())).unwrap();

                for _i in 0..NB_REQUEST {
                    let mut writer = Vec::new();

                    let _response = http_req::request::RequestBuilder::new(&uri)
                        .method(http_req::request::Method::POST)
                        .body(b"TEST")
                        .header("Content-length", "4")
                        .header("Connection", "Keep-Alive")
                        .send(&mut stream, &mut writer)
                        .unwrap();

                    let body = std::str::from_utf8(&writer).unwrap();
                    assert_eq!("POST", body);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    })
}

#[test]
fn close_connection() {
    run_test(|config| {
        let addr = config.http_addr.as_str();
        let uri: http_req::uri::Uri = addr.parse().unwrap();
        let mut stream = TcpStream::connect((uri.host().unwrap(), uri.corr_port())).unwrap();

        let mut writer = Vec::new();

        let response = http_req::request::RequestBuilder::new(&uri)
            .method(http_req::request::Method::POST)
            .body(b"TEST")
            .header("Content-length", "4")
            .header("Connection", "Close")
            .send(&mut stream, &mut writer)
            .unwrap();

        let body = std::str::from_utf8(&writer).unwrap();
        assert_eq!("POST", body);
        assert!(response.status_code().is(|code| { code == 200 }));

        let mut writer = Vec::new();
        let response = http_req::request::RequestBuilder::new(&uri)
            .method(http_req::request::Method::POST)
            .body(b"TEST")
            .header("Content-length", "4")
            .header("Connection", "Close")
            .send(&mut stream, &mut writer);

        assert!(response.is_err());
    })
}
