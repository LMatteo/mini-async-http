use crate::http::parser::BuildError;
use crate::http::Headers;
use crate::http::Method;
use crate::http::Version;

use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Request {
    method: Method,
    path: String,
    version: Version,
    headers: Headers,
    body: Option<Vec<u8>>,
}

impl Request {
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

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
            .get_map()
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

    pub fn method(mut self, method: Method) -> Self {
        self.method = Option::Some(method);
        self
    }

    pub fn path(mut self, path: String) -> Self {
        self.path = Option::Some(path);
        self
    }

    pub fn version(mut self, version: Version) -> Self {
        self.version = Option::Some(version);
        self
    }

    pub fn headers(mut self, headers: Headers) -> Self {
        self.headers = headers;
        self
    }

    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = Option::Some(body.to_vec());
        self
    }

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
