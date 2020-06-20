use std::convert::From;

#[derive(Debug)]
pub enum BuildError {
    Incomplete,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEnd,
    BuilderError(BuildError),
    LengthParse,
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
