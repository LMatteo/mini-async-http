use crate::aioserver::enhanced_stream::EnhancedStream;
use crate::aioserver::event_channel::{channel, EventedSender};
use crate::aioserver::id_generator::IdGenerator;
use crate::aioserver::worker::WorkerPool;
use crate::request::Request;
use crate::response::Response;

use std::io::ErrorKind;

use log::{error, trace};
use std::ops::Drop;

use std::collections::HashMap;

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};

use mio::net::{TcpListener, TcpStream};

use mio::{Events, Interest, Poll, Token, Waker};

const SERVER: Token = Token(0);
const SHUTDOWN: Token = Token(1);
const DELETE: Token = Token(2);
const WAKER: Token = Token(3);

type Status = Arc<(Mutex<bool>, Condvar)>;
pub(crate) type SafeStream<R> = Arc<Mutex<EnhancedStream<R>>>;

pub(crate) enum LoopTask {
    Shutdown,
    Close(usize),
}

/// Main struct of the crate, represent the http server
pub struct AIOServer<H> {
    handler: Arc<H>,
    pool: WorkerPool<H>,
    receiver: Receiver<LoopTask>,
    handle: ServerHandle,
    poll: mio::Poll,
    server: TcpListener,
}

impl<H> AIOServer<H>
where
    H: Send + Sync + 'static + Fn(&Request) -> Response,
{
    /// Start the server with the given thread pool size and bind to the given address
    /// The given function is executed for each http request received
    ///
    /// # Argument
    ///
    /// * `size` - Number of thread that will be spawned minimum is 1. The total minimum number of thread is 2 :
    /// 1 to handle request and 1 to run the event loop
    /// * `addr` - Address the server will bind to. The format is the same as std::net::TcpListener.
    /// If the address is incorrect or cannot be bound to, the function will panic
    /// * `handler` - function executed for each received http request
    ///
    /// # Example
    ///
    /// Create a simple server that will respond with a HTTP response with status 200, content type header
    /// "text/plain" and body "Hello"
    ///
    /// ```
    /// let server = mini_async_http::AIOServer::new(3, "127.0.0.1:7878", move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// ```
    pub fn new(size: i32, addr: &str, handler: H) -> AIOServer<H> {
        let handler = Arc::from(handler);
        let poll = Poll::new().unwrap();
        let waker = Arc::new(Waker::new(poll.registry(), WAKER).unwrap());
        let (sender, receiver) = channel(waker);
        let pool = WorkerPool::new(handler.clone(), size, sender.clone());
        let server = TcpListener::bind(addr.parse().unwrap()).unwrap();

        AIOServer {
            handler,
            pool,
            poll,
            receiver,
            handle: ServerHandle::new(sender),
            server,
        }
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
    /// let mut server = mini_async_http::AIOServer::new(3, "127.0.0.1:7879", move |request|{
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
        let mut map = HashMap::new();

        let mut events = Events::with_capacity(32768);

        let mut gen = IdGenerator::new(4);

        self.poll
            .registry()
            .register(&mut self.server, SERVER, Interest::READABLE)
            .unwrap();

        self.pool.start();

        self.handle.set_ready(true);

        loop {
            trace!("Opened connection: {}", map.len());
            trace!("Connection pool capacity: {}", map.capacity());
            self.poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                match event.token() {
                    SERVER => loop {
                        let connection = match self.server.accept() {
                            Ok((conn, _)) => conn,
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                error!("Error when accepting conn : {}", e);
                                continue;
                            }
                        };

                        let id = gen.id();
                        let token = Token(id);
                        let mut stream = EnhancedStream::new(id, connection);

                        match self
                            .poll
                            .registry()
                            .register(&mut stream, token, Interest::READABLE)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                if stream.shutdown().is_err() {
                                    trace!("Error when shutting down not registered connection")
                                }
                                error!("Error when registering conn : {}", e);
                                continue;
                            }
                        }

                        let stream = Arc::from(Mutex::from(stream));
                        map.insert(token, stream.clone());
                        self.pool.work(stream).unwrap();

                        trace!("New client with id : {}", id);
                    },
                    WAKER => {
                        while let Ok(task) = self.receiver.try_recv() {
                            match task {
                                LoopTask::Shutdown => {
                                    trace!("Shutting down");
                                    self.pool.join();
                                    self.handle.set_ready(false);
                                    return;
                                }
                                LoopTask::Close(id) => {
                                    AIOServer::<H>::remove_connection(
                                        id, &mut map, &self.poll, &mut gen,
                                    );
                                }
                            }
                        }
                    }
                    token => {
                        trace!("Data from id : {}", token.0);

                        let stream = match map.get(&token) {
                            Some(stream) => stream.clone(),
                            None => {
                                error!("Could not retrieve stream with id : {}", token.0);
                                continue;
                            }
                        };

                        self.pool.work(stream).unwrap();
                    }
                }
            }
        }
    }
}

impl<H> AIOServer<H> {
    /// Get a [`ServerHandle`] to this server
    ///
    /// [`ServerHandle`]: struct.ServerHandle.html
    pub fn handle(&self) -> ServerHandle {
        self.handle.clone()
    }

    fn remove_connection(
        id: usize,
        map: &mut HashMap<Token, SafeStream<TcpStream>>,
        poll: &Poll,
        generator: &mut IdGenerator,
    ) {
        let to_close = match map.remove(&Token(id)) {
            Some(val) => val,
            None => return,
        };

        let mut to_close = to_close.lock().unwrap();

        match to_close.shutdown() {
            Ok(_) => {}
            Err(e) => {
                error!("Issue when closing TCP connection {} : {}", id, e);
            }
        };

        match poll.registry().deregister(&mut (*to_close)) {
            Ok(_) => {}
            Err(e) => {
                error!("Issue when deregistering connection {} : {}", id, e);
            }
        };

        generator.remove(id);
    }
}

impl<H> Drop for AIOServer<H> {
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
    sender: EventedSender<LoopTask>,
}

impl ServerHandle {
    fn new(sender: EventedSender<LoopTask>) -> Self {
        ServerHandle {
            ready: Arc::new((Mutex::from(false), Condvar::new())),
            sender,
        }
    }

    fn set_ready(&self, ready_val: bool) {
        let (lock, cvar) = &*self.ready;
        let mut ready = lock.lock().unwrap();
        *ready = ready_val;

        cvar.notify_all();
    }

    /// Send a shutdown signal to the server.
    /// The server wait for all the received request to be handled and the stop
    ///
    /// # Example
    ///
    /// Creates a server and starts it. From another thread we send the shutdown signal
    /// causing the server to stop and the program to end.
    ///
    /// ```
    /// let mut server = mini_async_http::AIOServer::new(3, "127.0.0.1:7880", move |request|{
    ///     mini_async_http::ResponseBuilder::empty_200()
    ///         .body(b"Hello")
    ///         .content_type("text/plain")
    ///         .build()
    ///         .unwrap()
    /// });
    /// let handle = server.handle();
    ///
    /// std::thread::spawn(move || {
    ///     handle.shutdown();
    /// });
    ///
    /// server.start();
    ///
    /// ```
    pub fn shutdown(&self) {
        self.sender.send(LoopTask::Shutdown).unwrap();

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
    /// let mut server = mini_async_http::AIOServer::new(3, "127.0.0.1:7880", move |request|{
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
