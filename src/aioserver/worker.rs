use mio::event::Source;
use mio::net::TcpStream;
use std::io::{ErrorKind, Write};
use std::ops::Deref;
use std::sync::mpsc::{channel, SendError};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use mio::{Interest, Registry, Token};

use log::trace;

use crate::aioserver;
use crate::aioserver::SafeStream;
use crate::aioserver::{EnhancedStream, RequestError};
use crate::aioserver::{EventedReceiver, EventedSender};
use crate::http::ParseError;
use crate::request::Request;
use crate::response::Response;

type SafeReceiver = Arc<Mutex<Receiver<Job>>>;

pub enum Job {
    Stream(SafeStream<TcpStream>),
    Stop,
}

pub struct WorkerPool<H> {
    job_channel: (Sender<Job>, SafeReceiver),
    close_channel: (EventedSender<usize>, EventedReceiver<usize>),
    handler: Arc<H>,
    size: i32,
    handles: Vec<JoinHandle<()>>,
}

impl<H> WorkerPool<H>
where
    H: Send + Sync + 'static + Fn(&Request) -> Response,
{
    pub fn new(handler: Arc<H>, size: i32) -> WorkerPool<H> {
        let (sender, receiver) = channel();
        let receiver = Arc::from(Mutex::from(receiver));
        WorkerPool {
            job_channel: (sender, receiver),
            close_channel: aioserver::channel(),
            handler,
            size,
            handles: Vec::new(),
        }
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn start(&mut self) {
        let (_, receiver) = &self.job_channel;
        let (sender, _) = &self.close_channel;

        for _ in 0..self.size {
            let receiver = receiver.clone();
            let handler = self.handler.clone();
            let delete_sender = sender.clone();

            let join = std::thread::spawn(move || {
                let mut worker = Worker {
                    receiver,
                    delete_sender,
                    handler,
                };

                worker.work();
            });

            self.handles.push(join);
        }
    }

    pub fn work(&self, stream: SafeStream<TcpStream>) -> Result<(), SendError<Job>> {
        let (sender, _) = &self.job_channel;
        sender.send(Job::Stream(stream))
    }

    pub fn join(self) {
        let (sender, _) = &self.job_channel;
        for _ in &self.handles {
            sender.send(Job::Stop).unwrap();
        }

        for join in self.handles {
            join.join().unwrap();
        }
    }

    pub fn closed_stream(&self) -> Option<usize> {
        let (_, receiver) = &self.close_channel;
        match receiver.try_recv() {
            Ok(val) => Some(val),
            _ => None,
        }
    }
}

struct Worker<H> {
    receiver: SafeReceiver,
    delete_sender: EventedSender<usize>,
    handler: Arc<H>,
}

impl<H> Worker<H>
where
    H: Send + Sync + 'static + Fn(&Request) -> Response,
{
    fn work(&mut self) {
        loop {
            let lock = match self.receiver.lock().unwrap().recv().unwrap() {
                Job::Stream(stream) => stream,
                Job::Stop => return,
            };

            let mut stream = lock.lock().unwrap();

            let requests = match stream.requests() {
                Ok(requests) => requests,
                Err(RequestError::ParseError(ParseError::UnexpectedEnd)) => continue,
                Err(RequestError::ReadError(ref e)) if e.kind() == ErrorKind::WouldBlock => {
                    continue
                }
                Err(RequestError::EOF) => {
                    trace!("Reached EOF, closing stream {}", stream.id());
                    self.close_stream(stream.deref());
                    continue;
                }
                Err(e) => {
                    trace!("Error {:?} on reading request from {}", e, stream.id());
                    self.close_stream(stream.deref());
                    continue;
                }
            };

            for request in requests {
                let response = (self.handler)(&request);

                write!(stream, "{}", response);

                match request.headers().get_header(&"Connection".to_string()) {
                    Some(val) => {
                        if val == "close" {
                            self.close_stream(stream.deref())
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn close_stream(&self, stream: &EnhancedStream<TcpStream>) {
        self.delete_sender.send(stream.id()).unwrap();
    }
}

impl<T> Source for WorkerPool<T> {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> std::io::Result<()> {
        let (_, receiver) = &mut self.close_channel;
        receiver.register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> std::io::Result<()> {
        let (_, receiver) = &mut self.close_channel;
        receiver.reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> std::io::Result<()> {
        let (_, receiver) = &mut self.close_channel;
        receiver.deregister(registry)
    }
}
