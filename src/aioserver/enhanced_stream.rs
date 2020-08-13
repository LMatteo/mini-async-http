use mio::event::Source;
use mio::net::TcpStream;
use mio::{Interest, Registry, Token};

use log::trace;
use std::io::prelude::*;
use std::io::Error;

use crate::http::parser::ParseError;
use crate::request::request_parser::RequestParser;
use crate::request::Request;

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

#[derive(Debug)]
pub(crate) enum RequestError {
    EOF,
    ReadError(Error),
    ParseError(ParseError),
}
/// Wrapper for a stream to read data from.
/// It will try and buffer the maximum data that can be read from the inner Read and store it into its inner buffer
///
/// Warning : the buffer size is not limited which can be a major security issue
///
/// Once the stream is read it will try and parse http request, if no request can be parsed from the buffer, it will be left untouched
/// Everytime a request is read from the buffer, the corresponding section of the buffer is cleared
pub(crate) struct EnhancedStream<T> {
    id: usize,
    stream: T,
    parser: RequestParser,
    read: Vec<u8>,
    buffer: [u8; DEFAULT_BUF_SIZE],
}

impl<T> EnhancedStream<T> {
    fn parse_buf(&mut self) -> Result<Vec<Request>, RequestError> {
        let mut requests = Vec::new();

        loop {
            match self.parser.parse_u8(&self.read) {
                Ok((req, n)) => {
                    requests.push(req);
                    self.read = self.read.split_off(n);

                    if self.read.is_empty() {
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

impl<T: Read> EnhancedStream<T> {
    /// Create a new EnhancedStream from a Read struct and the associated id
    pub fn new(id: usize, stream: T) -> EnhancedStream<T> {
        EnhancedStream {
            id,
            stream,
            parser: RequestParser::new(),
            read: Vec::new(),
            buffer: [0; DEFAULT_BUF_SIZE],
        }
    }

    /// return the id associated to the EnhancedStream instance
    pub fn id(&self) -> usize {
        self.id
    }

    /// Read the inner Read struct and fill the buffer with the data
    /// If a request can be parsed from the inner buffer but is not finished will return an Unexpected End error
    /// Return an error if the inner Stream has reached EOF
    /// if the stream of byte received is not correctly formated, an error is returned and the stream is stopped
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

        self.parse_buf()
    }
}

impl<T> EnhancedStream<T>
    where T: futures::AsyncReadExt + Unpin {
    pub(crate) async fn poll_requests(&mut self) -> Result<Vec<Request>, RequestError> {
        match self.stream.read(&mut self.buffer).await {
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

            self.parse_buf()
        }
    }
} 

/// Implement Shutdown for the std implementation of TcpStream
impl EnhancedStream<std::net::TcpStream> {
    pub fn shutdown(&self) -> std::io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

/// Implement Shutdown for the mio implementation of TcpStream
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
