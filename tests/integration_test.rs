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
        let resp = reqwest::blocking::get(config.http_addr.as_str()).unwrap();

        let body = resp.text().unwrap();

        assert_eq!("GET", body);
    })
}

#[test]
fn simple_post_request() {
    run_test(|config| {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(config.http_addr.as_str())
            .body("the exact body that is sent")
            .send()
            .unwrap();

        let body = resp.text().unwrap();

        assert_eq!("POST", body);
    })
}

#[test]
fn multiple_post_connection() {
    run_test(|config| {
        let (sender, receiver) = channel();
        let receiver = Arc::from(Mutex::from(receiver));

        let counter = Arc::new(Mutex::from(0));
        let addr = config.http_addr;

        let mut handlers = Vec::new();

        for _ in 0..20 {
            let clone = receiver.clone();
            let counter = counter.clone();
            let addr = addr.clone();
            handlers.push(std::thread::spawn(move || {
                let client = reqwest::blocking::Client::new();

                loop {
                    match clone.lock().unwrap().recv().unwrap() {
                        Job::DefaultRequest => {
                            let resp = client
                                .post(addr.as_str())
                                .body("the exact body that is sent")
                                .send()
                                .unwrap();

                            let body = resp.text().unwrap();

                            assert_eq!("POST", body);
                        }
                        Job::Stop => return,
                        _ => panic!("Unexpected Job"),
                    }

                    let mut val = counter.lock().unwrap();
                    *val += 1;
                }
            }));
        }

        for _ in 0..200 {
            sender.send(Job::DefaultRequest).unwrap();
        }

        for _ in 0..20 {
            sender.send(Job::Stop).unwrap();
        }

        for handle in handlers {
            handle.join().unwrap();
        }

        assert_eq!(200, *counter.lock().unwrap());
    });
}
