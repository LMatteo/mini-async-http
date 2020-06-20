use crate::http::BuildError;
use crate::http::Headers;
use crate::http::Method;
use crate::http::Version;

use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub headers: Headers,
    pub body: Option<String>,
}

impl Request {
    pub fn get_method(&self) -> &Method {
        &self.method
    }

    pub fn get_path(&self) -> &String {
        &self.path
    }

    pub fn get_version(&self) -> &Version {
        &self.version
    }

    pub fn get_headers(&self) -> &Headers {
        &self.headers
    }

    pub fn get_body(&self) -> Option<&String> {
        self.body.as_ref()
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

        match &self.body {
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
    headers: Option<Headers>,
    body: Option<String>,
}

impl RequestBuilder {
    pub fn new_builder() -> RequestBuilder {
        RequestBuilder {
            method: Option::None,
            path: Option::None,
            version: Option::None,
            headers: Option::None,
            body: Option::None,
        }
    }

    pub fn set_method(&mut self, method: Method) {
        self.method = Option::Some(method);
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Option::Some(path);
    }

    pub fn set_version(&mut self, version: Version) {
        self.version = Option::Some(version);
    }

    pub fn set_headers(&mut self, headers: Headers) {
        self.headers = Option::Some(headers);
    }

    pub fn set_body(&mut self, body: String) {
        self.body = Option::Some(body);
    }

    pub fn build(self) -> Result<Request, BuildError> {
        let method = match self.method {
            Some(val) => val,
            None => return Result::Err(BuildError::incomplete),
        };

        let path = match self.path {
            Some(val) => val,
            None => return Result::Err(BuildError::incomplete),
        };

        let version = match self.version {
            Some(val) => val,
            None => return Result::Err(BuildError::incomplete),
        };

        let headers = match self.headers {
            Some(val) => val,
            None => return Result::Err(BuildError::incomplete),
        };

        return Result::Ok(Request {
            method,
            path,
            version,
            headers,
            body: self.body,
        });
    }
}
