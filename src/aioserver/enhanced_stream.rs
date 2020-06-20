use mio::event::Source;
use mio::net::TcpStream;
use mio::{Interest, Registry, Token};

use log::trace;
use std::io::prelude::*;
use std::io::Error;

use crate::http::ParseError;
use crate::request::Request;
use crate::request::RequestParser;

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

#[derive(Debug)]
pub enum RequestError {
    EOF,
    ReadError(Error),
    ParseError(ParseError),
}

pub struct EnhancedStream<T> {
    id: usize,
    stream: T,
    parser: RequestParser,
    read: Vec<u8>,
    buffer: [u8; DEFAULT_BUF_SIZE],
}

impl<T: Read> EnhancedStream<T> {
    pub fn new(id: usize, stream: T) -> EnhancedStream<T> {
        EnhancedStream {
            id,
            stream,
            parser: RequestParser::new(),
            read: Vec::new(),
            buffer: [0; DEFAULT_BUF_SIZE],
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn requests(&mut self) -> Result<Vec<Request>, RequestError> {
        match self.stream.read(&mut self.buffer) {
            Ok(0) => {
                trace!("Reached EOF for {}", self.id);
                return Err(RequestError::EOF);
            }
            Ok(n) => {
                self.read.extend_from_slice(&self.buffer[0..n]);
                trace!("Read {} bytes from {}", n, self.id);
            }
            Err(e) => {
                trace!("Error {:?} when reading {}", e, self.id);
                return Err(RequestError::ReadError(e));
            }
        }

        let mut requests = Vec::new();

        loop {
            match self.parser.parse_u8(&self.read) {
                Ok((req, n)) => {
                    requests.push(req);
                    self.read = self.read.split_off(n);

                    if self.read.len() == 0 {
                        break;
                    }
                }
                Err(ParseError::UnexpectedEnd) => break,
                Err(e) => return Err(RequestError::ParseError(e)),
            }
        }

        Ok(requests)
    }
}

impl EnhancedStream<std::net::TcpStream> {
    pub fn shutdown(&self) -> std::io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

impl EnhancedStream<mio::net::TcpStream> {
    pub fn shutdown(&self) -> std::io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

impl<T: Write> Write for EnhancedStream<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Source for EnhancedStream<TcpStream> {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> std::io::Result<()> {
        // Delegate the `register` call to `socket`
        self.stream.register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> std::io::Result<()> {
        // Delegate the `reregister` call to `socket`
        self.stream.reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> std::io::Result<()> {
        // Delegate the `deregister` call to `socket`
        self.stream.deregister(registry)
    }
}
