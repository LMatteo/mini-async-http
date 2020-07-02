use mio::net::TcpStream;
use std::io::{ErrorKind, Write};
use std::ops::Deref;
use std::sync::mpsc::{channel, SendError};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use log::trace;

use crate::aioserver::enhanced_stream::{EnhancedStream, RequestError};
use crate::aioserver::event_channel::EventedSender;
use crate::aioserver::server::LoopTask;
use crate::aioserver::server::SafeStream;
use crate::http::parser::ParseError;
use crate::request::Request;
use crate::response::Response;

type SafeReceiver = Arc<Mutex<Receiver<Job>>>;

pub(crate) enum Job {
    Stream(SafeStream<TcpStream>),
    Stop,
}

pub(crate) struct WorkerPool<H> {
    job_channel: (Sender<Job>, SafeReceiver),
    job_handle: EventedSender<LoopTask>,
    handler: Arc<H>,
    size: i32,
    handles: Vec<JoinHandle<()>>,
}

impl<H> WorkerPool<H>
where
    H: Send + Sync + 'static + Fn(&Request) -> Response,
{
    pub fn new(handler: Arc<H>, size: i32, job_handle: EventedSender<LoopTask>) -> WorkerPool<H> {
        let (sender, receiver) = channel();
        let receiver = Arc::from(Mutex::from(receiver));
        WorkerPool {
            job_channel: (sender, receiver),
            job_handle,
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
        let sender = &self.job_handle.clone();

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

    pub fn join(&mut self) {
        let (sender, _) = &self.job_channel;
        for _ in &self.handles {
            sender.send(Job::Stop).unwrap();
        }

        while let Some(join) = self.handles.pop() {
            join.join().unwrap();
        }
    }
}

struct Worker<H> {
    receiver: SafeReceiver,
    delete_sender: EventedSender<LoopTask>,
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

                match write!(stream, "{}", response) {
                    Ok(_) => trace!("Written to id {}", stream.id()),
                    Err(e) => trace!("Error({}) when writing to connection {}", e, stream.id()),
                }

                let connection = request.headers().get_header("Connection");
                if let Some(val) = connection {
                    if val == "close" {
                        self.close_stream(stream.deref())
                    }
                }
            }
        }
    }

    fn close_stream(&self, stream: &EnhancedStream<TcpStream>) {
        self.delete_sender
            .send(LoopTask::Close(stream.id()))
            .unwrap();
    }
}
