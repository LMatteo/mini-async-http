extern crate mio;
extern crate httparse;
extern crate log;

mod aioserver;
mod http;
mod request;
mod response;

use std::sync::{Arc,Mutex};

pub fn main(){
    let counter = Arc::from(Mutex::from(0));

    let server = aioserver::AIOServer::new(3, "0.0.0.0:7878", move |request|{
        let lock = counter.clone();
        let mut counter = lock.lock().unwrap();

        let body = counter.to_string();
        *counter +=1;

        response::ResponseBuilder::empty_200()
            .body(body.as_bytes())
            .content_type("text/plain")
            .build()
            .unwrap()
    });

    server.start();
}