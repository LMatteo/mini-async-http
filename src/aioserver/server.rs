use crate::aioserver::EnhancedStream;
use crate::aioserver::IdGenerator;
use crate::aioserver::WorkerPool;
use crate::request::Request;
use crate::response::Response;

use std::io::ErrorKind;

use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixDatagram;

use log::{error, trace};
use std::ops::Drop;

use std::collections::HashMap;

use std::sync::{Arc, Condvar, Mutex};

use mio::net::{TcpListener, TcpStream};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

const SERVER: Token = Token(0);
const SHUTDOWN: Token = Token(1);
const DELETE: Token = Token(2);

type Status = Arc<(Mutex<bool>, Condvar)>;
pub type SafeStream<R> = Arc<Mutex<EnhancedStream<R>>>;

pub struct AIOServer<H> {
    addr: String,
    handler: Arc<H>,
    datagram: (UnixDatagram, UnixDatagram),
    ready: Status,
    size: i32,
}

impl<H> AIOServer<H>
where
    H: Send + Sync + 'static + Fn(&Request) -> Response,
{
    pub fn new(size: i32, addr: &str, handler: H) -> AIOServer<H> {
        let handler = Arc::from(handler);
        AIOServer {
            addr: String::from(addr),
            handler,
            datagram: UnixDatagram::pair().expect("Could not create datagram"),
            ready: Arc::new((Mutex::from(false), Condvar::new())),
            size,
        }
    }

    pub fn start(&self) {
        let mut map = HashMap::new();

        let mut poll = Poll::new().unwrap();

        let mut events = Events::with_capacity(32768);

        let mut gen = IdGenerator::new(3);
        let mut server = TcpListener::bind(self.addr.parse().unwrap()).unwrap();

        poll.registry()
            .register(&mut server, SERVER, Interest::READABLE)
            .unwrap();

        let (_, receiver) = &self.datagram;

        poll.registry()
            .register(
                &mut SourceFd(&receiver.as_raw_fd()),
                SHUTDOWN,
                Interest::READABLE,
            )
            .unwrap();

        let handler = self.handler.clone();

        let mut pool = WorkerPool::new(handler, self.size);

        poll.registry()
            .register(&mut pool, DELETE, Interest::READABLE)
            .unwrap();

        pool.start();

        self.set_ready(true);

        loop {
            trace!("Opened connection: {}", map.len());
            trace!("Connection pool capacity: {}", map.capacity());
            poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                match event.token() {
                    SERVER => loop {
                        let connection = match server.accept() {
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

                        match poll
                            .registry()
                            .register(&mut stream, token, Interest::READABLE)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                stream.shutdown();
                                error!("Error when registering conn : {}", e);
                                continue;
                            }
                        }

                        let stream = Arc::from(Mutex::from(stream));
                        map.insert(token, stream.clone());
                        pool.work(stream).unwrap();

                        trace!("New client with id : {}", id);
                    },
                    SHUTDOWN => {
                        let mut buf: [u8; 10] = [0; 10];
                        let (_, stop_receiver) = &self.datagram;
                        stop_receiver.recv(&mut buf).unwrap();

                        pool.join();

                        self.set_ready(false);

                        trace!("Shutting down");
                        return;
                    }
                    DELETE => {
                        AIOServer::remove_connection(&pool, &mut map, &poll);
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

                        pool.work(stream).unwrap();
                    }
                }
            }
        }
    }

    pub fn remove_connection(
        pool: &WorkerPool<H>,
        map: &mut HashMap<Token, SafeStream<TcpStream>>,
        poll: &Poll,
    ) {
        loop {
            let id = match pool.closed_stream() {
                Some(val) => val,
                None => break,
            };

            let to_close = match map.remove(&Token(id)) {
                Some(val) => val,
                None => continue,
            };

            let mut to_close = to_close.lock().unwrap();

            match to_close.shutdown() {
                Ok(_) => {}
                Err(e) => {
                    error!("Issue when closing TCP connection {} : {}", id, e);
                    continue;
                }
            };

            match poll.registry().deregister(&mut (*to_close)) {
                Ok(_) => {}
                Err(e) => {
                    error!("Issue when deregistering connection {} : {}", id, e);
                    continue;
                }
            };
        }
    }
}

impl<H> AIOServer<H> {
    fn set_ready(&self, ready_val: bool) {
        let (lock, cvar) = &*self.ready;
        let mut ready = lock.lock().unwrap();
        *ready = ready_val;

        cvar.notify_all();
    }

    pub fn shutdown(&self) {
        let (sender, _) = &self.datagram;
        sender.send(b"S").expect("Could not write to unix stream");

        let (lock, cvar) = &*self.ready;
        let mut started = lock.lock().unwrap();

        while *started {
            started = cvar.wait(started).unwrap();
        }
    }

    pub fn ready(&self) {
        let (lock, cvar) = &*self.ready;
        let mut started = lock.lock().unwrap();

        while !*started {
            started = cvar.wait(started).unwrap();
        }
    }
}

impl<H> Drop for AIOServer<H> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
