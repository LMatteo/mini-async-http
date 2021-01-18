# mini-async-http

[Documentation](https://docs.rs/mini_async_http/0.1.0/mini_async_http) | [Crates.io](https://crates.io/crates/mini_async_http)

mini-async-http is a tiny http server. I have built it in order to practive and learn the rust language. It is 
based on the [mio](https://github.com/tokio-rs/mio) library for all that is related to async io.

## Example 

You can create a server with the following code :

```rust
extern crate mini_async_http;

use mini_async_http::AIOServer;
use mini_async_http::ResponseBuilder;

pub fn main(){
    let mut server = AIOServer::new("0.0.0.0:7878", move |request|{
        ResponseBuilder::empty_200()
            .body(b"Hello")
            .content_type("text/plain")
            .build()
            .unwrap()
    });

    server.start();
}
```
Here the server is created with 3 worker threads and will bind to the port 7878. 
When a new request is received, the response body is set to "Hello" and sent. 

You can share a state between the calls with the usual rust sync mechanism.
Here is an example with a simple counter that is incremented on every request : 


```rust
extern crate mini_async_http;

use mini_async_http::AIOServer;
use mini_async_http::ResponseBuilder;
use std::sync::{Arc, Mutex};

pub fn main() {
    let counter = Arc::from(Mutex::from(0));

    let mut server = AIOServer::new("0.0.0.0:7878", move |request| {
        let lock = counter.clone();
        let mut counter = lock.lock().unwrap();

        let body = counter.to_string();
        *counter += 1;

        ResponseBuilder::empty_200()
            .body(body.as_bytes())
            .content_type("text/plain")
            .build()
            .unwrap()
    });

    server.start();
}
```


## Architecture

This implementation is based on [mio](https://github.com/tokio-rs/mio).
All the new connection and requests are handled by an event loop.
The event loop is handling 4 different type of event :

1. New connection : when a new client is connecting, the resulting tcp stream is registered
in the mio registry in order to be notified when data is received through this connection.

2. New data : A stream is available to be read. The stream is sent to the worler pool to be handled.

3. Closed connection : A connection has been closed either by the client or the server itself. It must 
be removed from the watch list and dropped.

4. Server shutdown : The server is shutting down, it is now waiting on the worker threads to stop 
to end the event loop

It is important that no calculation is done in the event loop as it is the core of the server performance. 

