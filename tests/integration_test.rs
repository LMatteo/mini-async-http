use mini_async_http::Request;

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

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
        let res = http_req::request::get(config.http_addr.as_str(), &mut writer).unwrap();


        let body = std::str::from_utf8(&writer).unwrap();

        assert_eq!("GET", body);
    })
}

#[test]
fn simple_post_request() {
    run_test(|config| {
        let mut writer = Vec::new();
        let body = b"RequestBody";
        let res = http_req::request::post(config.http_addr.as_str(), body, &mut writer).unwrap();

        let body = std::str::from_utf8(&writer).unwrap();

        assert_eq!("POST", body);
    })
}