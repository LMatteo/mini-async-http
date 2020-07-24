use crate::http::parser::BuildError;
use crate::http::Headers;
use crate::http::Method;
use crate::http::Version;

use std::convert::TryFrom;
use std::fmt;

/// Represent an http request.  
#[derive(Debug, PartialEq)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    headers: Headers,
    body: Option<Vec<u8>>,
}

impl Request {
    /// Return the request Method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Return the target path of the request
    pub fn path(&self) -> &String {
        &self.path
    }

    /// Return the HTTP version of the request
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Return the headers of the request
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Return the body of the request as byte vector
    pub fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

    /// Return the body of the request interpreted as utf 8 string
    pub fn body_as_string(&self) -> Option<String> {
        match self.body.as_ref() {
            Some(val) => match String::from_utf8(val.to_vec()) {
                Ok(body) => Some(body),
                Err(_) => None,
            },
            None => None,
        }
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();

        buf.push_str(
            format!(
                "{} {} {}\r\n",
                self.method.as_str(),
                self.path,
                self.version.as_str()
            )
            .as_str(),
        );

        self.headers
            .iter()
            .for_each(|(key, value)| buf.push_str(format!("{}: {}\r\n", key, value).as_str()));

        buf.push_str("\r\n");

        match &self.body_as_string() {
            Some(body) => buf.push_str(body.as_str()),
            None => {}
        };

        write!(f, "{}", buf)
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = crate::http::parser::ParseError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let parser = crate::request::request_parser::RequestParser::new();

        match parser.parse_u8(slice) {
            Ok((request, _)) => Ok(request),
            Err(e) => Err(e),
        }
    }
}

/// Build a request
pub struct RequestBuilder {
    method: Option<Method>,
    path: Option<String>,
    version: Option<Version>,
    headers: Headers,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn new() -> RequestBuilder {
        RequestBuilder {
            method: Option::None,
            path: Option::None,
            version: Option::None,
            headers: Headers::new(),
            body: Option::None,
        }
    }

    /// Provide the method for the request
    pub fn method(mut self, method: Method) -> Self {
        self.method = Option::Some(method);
        self
    }

    /// Provide the path for the request
    pub fn path(mut self, path: String) -> Self {
        self.path = Option::Some(path);
        self
    }

    /// Provide the version for the request
    pub fn version(mut self, version: Version) -> Self {
        self.version = Option::Some(version);
        self
    }

    /// Provide the headers for the request
    pub fn headers(mut self, headers: Headers) -> Self {
        self.headers = headers;
        self
    }

    /// Provide the body for the request
    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = Option::Some(body.to_vec());
        self
    }

    /// Build the request with provided informations.
    /// If some informations are missing, BuildError will occur
    pub fn build(self) -> Result<Request, BuildError> {
        let method = match self.method {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        let path = match self.path {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        let version = match self.version {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        Result::Ok(Request {
            method,
            path,
            version,
            headers: self.headers,
            body: self.body,
        })
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        RequestBuilder::new()
    }
}
