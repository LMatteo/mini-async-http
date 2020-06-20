use http_server::request::Request;
use http_server::response::ResponseParser;

use std::io::prelude::*;
use std::net::TcpStream;
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

#[test]
fn same_connection() {
    run_test(|config| {
        let parser = ResponseParser::new_parser();

        let request = request();
        let mut stream = TcpStream::connect(config.addr.clone()).expect("Could not connect");

        stream.write(format!("{}", request).as_bytes()).unwrap();
        let received = parser
            .parse(&mut stream)
            .expect("Could not read server response");
        assert_eq!(
            "GET",
            String::from_utf8(received.body.expect("Missing body"))
                .unwrap()
                .as_str()
        );

        stream.write(format!("{}", request).as_bytes()).unwrap();
        let received = parser
            .parse(&mut stream)
            .expect("Could not read server response");
        assert_eq!(
            "GET",
            String::from_utf8(received.body.expect("Missing body"))
                .unwrap()
                .as_str()
        );

        stream.write(format!("{}", request).as_bytes()).unwrap();
        let received = parser
            .parse(&mut stream)
            .expect("Could not read server response");
        assert_eq!(
            "GET",
            String::from_utf8(received.body.expect("Missing body"))
                .unwrap()
                .as_str()
        );

        stream.write(format!("{}", request).as_bytes()).unwrap();
        let received = parser
            .parse(&mut stream)
            .expect("Could not read server response");
        assert_eq!(
            "GET",
            String::from_utf8(received.body.expect("Missing body"))
                .unwrap()
                .as_str()
        );
    })
}

fn multiple_connection() {
    run_test(|config| {
        let (sender, receiver) = channel();
        let receiver = Arc::from(Mutex::from(receiver));

        let counter = Arc::new(Mutex::from(0));
        let addr = config.addr;

        let mut handlers = Vec::new();

        for _ in 0..20 {
            let clone = receiver.clone();
            let counter = counter.clone();
            let addr = addr.clone();
            handlers.push(std::thread::spawn(move || {
                let parser = ResponseParser::new_parser();
                let mut stream = TcpStream::connect(addr).expect("Could not connect");

                loop {
                    match clone.lock().unwrap().recv().unwrap() {
                        Job::Request(req) => {
                            stream.write(format!("{}", req).as_bytes()).unwrap();
                            stream.flush().unwrap();

                            let received = parser
                                .parse(&mut stream)
                                .expect("Could not read server response");
                            assert_eq!(
                                "GET",
                                String::from_utf8(received.body.expect("Missing body"))
                                    .unwrap()
                                    .as_str()
                            );
                        }
                        Job::Stop => return,
                        _ => panic!("Unexpected Job"),
                    }

                    let mut val = counter.lock().unwrap();
                    *val += 1;
                }
            }));
        }

        for _i in 0..200 {
            let request = request();
            sender.send(Job::Request(request)).unwrap();
        }

        for _ in &handlers {
            sender.send(Job::Stop).unwrap();
        }

        for handle in handlers {
            handle.join().unwrap();
        }

        assert_eq!(200, *counter.lock().unwrap());
    });
}

#[test]
fn interuption() {
    run_test(|config| {
        let parser = ResponseParser::new_parser();

        let request = request();
        let mut stream = TcpStream::connect(config.addr.clone()).expect("Could not connect");

        let payload = Vec::from(format!("{}", request).as_bytes());
        let (begin, end) = payload.split_at(payload.len() / 2);

        stream.write(begin).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(200));

        stream.write(end).unwrap();

        let received = parser
            .parse(&mut stream)
            .expect("Could not read server response");
        assert_eq!(
            "GET",
            String::from_utf8(received.body.expect("Missing body"))
                .unwrap()
                .as_str()
        );
    })
}
