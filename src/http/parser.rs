use crate::http::Headers;

use regex::Regex;
use std::convert::From;
use std::io::BufRead;
use std::io::Error;
use std::io::Read;

#[derive(Debug)]
pub enum BuildError {
    Incomplete,
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