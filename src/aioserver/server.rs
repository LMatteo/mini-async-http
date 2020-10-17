use crate::aioserver::enhanced_stream::EnhancedStream;
use crate::data::AtomicTake;
use crate::http::header::CLOSE_CONNECTION_HEADER;
use crate::http::header::CONNECTION_HEADER;
use crate::io::context;
use crate::request::Request;
use crate::response::Response;

use std::io::Write;
use std::net::SocketAddr;

use std::ops::Drop;

use std::sync::{Arc, Condvar, Mutex};

use futures::channel::oneshot;
use futures::future::FutureExt;

type Status = Arc<(Mutex<bool>, Condvar)>;
pub(crate) type SafeStream<R> = Arc<Mutex<EnhancedStream<R>>>;

/// Main struct of the crate, represent the http server
pub struct AIOServer {
    handler: Arc<dyn Send + Sync + 'static + Fn(&Request) -> Response>,
    handle: ServerHandle,
    addr: SocketAddr,

    stop_sender: Arc<AtomicTake<oneshot::Sender<()>>>,
}

impl AIOServer {
    /// Start the server with the given thread pool size and bind to the given address
    /// The given function is executed for each http request received
    ///
    /// # Argument
    ///
    /// * `addr` - Address the server will bind to. The format is the same as std::net::TcpListener.
    /// * `handler` - function executed for each received http request
    ///
    /// # Example
    ///
    /// Create a simple server that will respond with a HTTP response with status 200, content type header
    /// "text/plain" and body "Hello"
    ///
    /// ```
    /// let server = mini_async_http::AIOServer::new("127.0.0.1:7878".parse().unwrap(), move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// ```
    pub fn new<H>(addr: SocketAddr, handler: H) -> AIOServer
    where
        H: Send + Sync + 'static + Fn(&Request) -> Response,
    {
        let stop_sender = Arc::from(AtomicTake::<oneshot::Sender<()>>::new());

        AIOServer {
            handler: Arc::from(handler),
            handle: ServerHandle::new(stop_sender.clone()),
            addr,
            stop_sender,
        }
    }

    /// Create a new server from a [`Router`] replacing the handler function
    ///
    /// # Example
    ///
    ///
    ///
    /// ```
    /// use mini_async_http::{Router,ResponseBuilder,AIOServer, Method};
    ///
    /// let router = mini_async_http::router!(
    ///     "/example", Method::GET => |_,_|ResponseBuilder::empty_200().body(b"GET").build().unwrap(),
    ///     "/example2", Method::POST => |_,_|ResponseBuilder::empty_200().body(b"POST").build().unwrap()
    /// );
    ///
    /// let server = mini_async_http::AIOServer::from_router("127.0.0.1:7878".parse().unwrap(),router);
    /// ```
    /// [`Router`]: struct.Router.html
    pub fn from_router(addr: SocketAddr, router: crate::Router) -> AIOServer {
        AIOServer::new(addr, move |req| router.exec(req))
    }

    /// Start the event loop. This call is blocking but you can still interact with the server through the Handle
    ///
    /// # Example
    ///
    /// Create a simple server and then start it.
    /// It is started from another thread as the start call is blocking.
    /// After spawning the thread, wait for the server to be ready and then shut it down
    ///
    /// ```
    /// let mut server = mini_async_http::AIOServer::new("127.0.0.1:7879".parse().unwrap(), move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// let handle = server.handle();
    ///
    /// std::thread::spawn(move || {
    ///     server.start();
    /// });
    ///
    /// handle.ready();
    /// handle.shutdown();
    ///
    /// ```
    pub fn start(&mut self) {
        context::start();

        self.async_run();

        self.handle.set_ready(false);
    }

    fn async_run(&mut self) {
        let handler = self.handler.clone();
        let handle = self.handle();
        let addr = self.addr;

        let (stop_sender, stop_receiver) = oneshot::channel::<()>();
        self.stop_sender.store(stop_sender);

        let server = async move {
            let listener = crate::io::tcp_listener::TcpListener::bind(addr);
            handle.set_ready(true);

            let receiver = stop_receiver.fuse();
            futures::pin_mut!(receiver);

            loop {
                let accept = listener.accept().fuse();
                futures::pin_mut!(accept);

                let connection = futures::select! {
                    conn = accept => conn,
                    _ = receiver => {return},
                };
                let connection = match connection {
                    Ok((conn, _)) => conn,
                    Err(_) => return,
                };

                let handler = handler.clone();
                context::spawn(async move {
                    let connection = crate::io::tcp_stream::TcpStream::from_stream(connection);
                    let mut stream = EnhancedStream::new(0, connection);
                    loop {
                        let requests = match stream.poll_requests().await {
                            Ok(reqs) => reqs,
                            Err(_) => return,
                        };

                        for request in requests {
                            let response = (handler)(&request);
                            write!(stream, "{}", response).unwrap();

                            if let Some(header) = request.headers().get_header(CONNECTION_HEADER) {
                                if header == CLOSE_CONNECTION_HEADER {
                                    return;
                                }
                            }
                        }
                    }
                });
            }
        };
        context::block_on(server);
    }
}

impl AIOServer {
    /// Get a [`ServerHandle`] to this server
    ///
    /// [`ServerHandle`]: struct.ServerHandle.html
    pub fn handle(&self) -> ServerHandle {
        self.handle.clone()
    }
}

impl Drop for AIOServer {
    fn drop(&mut self) {
        self.handle.shutdown();
    }
}
/// Clonable handle to a server.
/// Can only be retrieved from a Server instance.
/// Used to wait for the server to be ready or to shut it down.
#[derive(Clone)]
pub struct ServerHandle {
    ready: Status,
    stop_sender: Arc<AtomicTake<oneshot::Sender<()>>>,
}

impl ServerHandle {
    fn new(stop_sender: Arc<AtomicTake<oneshot::Sender<()>>>) -> Self {
        ServerHandle {
            ready: Arc::new((Mutex::from(false), Condvar::new())),
            stop_sender,
        }
    }

    fn set_ready(&self, ready_val: bool) {
        let (lock, cvar) = &*self.ready;
        let mut ready = lock.lock().unwrap();
        *ready = ready_val;

        cvar.notify_all();
    }

    /// Send a shutdown signal to the server and wait for it to stop.
    /// If the server is not started, the function returns immediately.
    ///
    /// # Example
    ///
    /// Creates a server and starts it. From another thread we send the shutdown signal
    /// causing the server to stop and the execution to end.
    ///
    /// ```
    /// let mut server = mini_async_http::AIOServer::new("127.0.0.1:7880".parse().unwrap(), move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// let handle = server.handle();
    ///
    /// std::thread::spawn(move || {
    ///     handle.ready();
    ///     handle.shutdown();
    /// });
    ///
    /// server.start();
    ///
    /// ```
    pub fn shutdown(&self) {
        let sender = match self.stop_sender.take() {
            Some(val) => val,
            None => return,
        };

        if sender.send(()).is_err() {
            return;
        }

        let (lock, cvar) = &*self.ready;
        let mut started = lock.lock().unwrap();

        while *started {
            started = cvar.wait(started).unwrap();
        }
    }

    /// Block untill the server is ready to receive requests
    ///
    /// # Example
    ///
    /// Creates a server and starts it in a separate thread.
    /// The main thread waits for the server to be ready and then ends
    ///
    /// ```
    /// let mut server = mini_async_http::AIOServer::new("127.0.0.1:7880".parse().unwrap(), move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// let handle = server.handle();
    ///
    /// std::thread::spawn(move || {
    ///     server.start();
    /// });
    ///
    /// handle.ready();
    ///
    /// ```
    pub fn ready(&self) {
        let (lock, cvar) = &*self.ready;
        let mut started = lock.lock().unwrap();

        while !*started {
            started = cvar.wait(started).unwrap();
        }
    }
}
