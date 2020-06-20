use crate::http::Headers;

use regex::Regex;
use std::convert::From;
use std::io::BufRead;
use std::io::Error;
use std::io::Read;

#[derive(Debug)]
pub enum BuildError {
    incomplete,
}

#[derive(Debug)]
pub enum ParseError {
    FirstLine,
    WrongMethod,
    WrongVersion,
    ReadError(Error),
    UnexpectedEnd,
    HeaderError,
    BuilderError(BuildError),
    LengthParse,
    BodyReadException,
    CodeParseError,
    HeaderName,
    HeaderValue,
    NewLine,
    Status,
    Token,
    TooManyHeaders,
    Version,
}

impl From<httparse::Error> for ParseError {
    fn from(error: httparse::Error) -> Self {
        match error {
            httparse::Error::HeaderName => ParseError::HeaderName,
            httparse::Error::HeaderValue => ParseError::HeaderValue,
            httparse::Error::NewLine => ParseError::NewLine,
            httparse::Error::Status => ParseError::Status,
            httparse::Error::Token => ParseError::Token,
            httparse::Error::TooManyHeaders => ParseError::TooManyHeaders,
            httparse::Error::Version => ParseError::Version,
        }
    }
}

pub struct Parser {
    headerRe: Regex,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            headerRe: Regex::new(r"(?P<header>[^:]+): (?P<value>.*)\r\n").unwrap(),
        }
    }

    pub fn parse(
        &self,
        reader: &mut dyn BufRead,
    ) -> Result<(Headers, Option<Vec<u8>>, usize), ParseError> {
        let mut headers = Headers::new();

        let mut nb = 0;

        loop {
            let mut buf = String::new();
            match reader.read_line(&mut buf) {
                Ok(0) => return Result::Err(ParseError::UnexpectedEnd),
                Ok(n) => {
                    nb += n;
                    if buf == "\r\n" {
                        break;
                    }
                    if !buf.ends_with("\r\n") {
                        return Err(ParseError::UnexpectedEnd);
                    }

                    let caps = match self.headerRe.captures(buf.as_str()) {
                        Some(caps) => caps,
                        None => return Result::Err(ParseError::HeaderError),
                    };

                    headers.set_header(
                        &String::from(caps.name("header").unwrap().as_str()),
                        &String::from(caps.name("value").unwrap().as_str()),
                    );
                }
                Err(e) => return Result::Err(ParseError::ReadError(e)),
            }
        }

        let content_length = match headers.get_header(&String::from("content-length")) {
            Some(val) => val,
            None => return Result::Ok((headers, Option::None, nb)),
        };

        let content_length = match content_length.parse::<u64>() {
            Ok(val) => val,
            Err(_) => return Result::Err(ParseError::LengthParse),
        };

        let mut bodyHandle = reader.take(content_length);
        let mut buffer = vec![0; content_length as usize];

        match bodyHandle.read(&mut buffer) {
            Err(e) => return Result::Err(ParseError::ReadError(e)),
            Ok(n) => {
                if n != content_length as usize {
                    println!("got n : {} instead of {}", n, content_length);
                    return Err(ParseError::UnexpectedEnd);
                }
                nb += n;
            }
        };

        return Result::Ok((headers, Option::Some(buffer), nb));
    }
}
