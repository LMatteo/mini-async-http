use crate::http::BuildError;
use crate::http::Headers;
use crate::http::Version;
use crate::response::Reason;

use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Response {
    pub code: i32,
    pub reason: String,
    pub version: Version,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();

        buf.push_str(format!("{} {} {}", self.version.as_str(), self.code, self.reason).as_str());
        buf.push_str("\r\n");

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

impl Response {
    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn reason(&self) -> &String {
        &self.reason
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

pub struct ResponseBuilder {
    code: Option<i32>,
    reason: Option<String>,
    version: Option<Version>,
    headers: Option<Headers>,
    body: Option<Vec<u8>>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            code: Option::None,
            reason: Option::None,
            version: Option::Some(Version::HTTP11),
            headers: Option::Some(Headers::new()),
            body: Option::None,
        }
    }

    pub fn empty_500() -> Self {
        ResponseBuilder::new()
            .code(Reason::INTERNAL500.code())
            .reason(Reason::INTERNAL500.reason())
            .version(Version::HTTP11)
    }

    pub fn empty_200() -> Self {
        ResponseBuilder::new()
            .code(Reason::OK200.code())
            .reason(Reason::OK200.reason())
            .version(Version::HTTP11)
    }

    pub fn empty_400() -> Self {
        ResponseBuilder::new()
            .code(Reason::BADREQUEST400.code())
            .reason(Reason::BADREQUEST400.reason())
            .version(Version::HTTP11)
    }

    pub fn code(mut self, code: i32) -> Self {
        self.code = Option::Some(code);
        self
    }

    pub fn reason(mut self, reason: String) -> Self {
        self.reason = Option::Some(reason);
        self
    }

    pub fn version(mut self, version: Version) -> Self {
        self.version = Option::Some(version);
        self
    }

    pub fn headers(mut self, headers: Headers) -> Self {
        self.headers = Option::Some(headers);
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        let key = &String::from(key);
        let value = &String::from(value);

        match self.headers.as_mut() {
            Some(headers) => headers.set_header(key, value),
            None => {
                let mut headers = Headers::new();
                headers.set_header(key, value);
                self.headers = Some(headers);
            }
        };

        self
    }

    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = Option::Some(body.to_vec());
        self
    }

    pub fn status(mut self, status: Reason) -> Self {
        self.code = Some(status.code());
        self.reason = Some(status.reason());

        self
    }

    pub fn build(self) -> Result<Response, BuildError> {
        let code = match self.code {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        let reason = match self.reason {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        let version = match self.version {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        let headers = match self.headers {
            Some(val) => val,
            None => return Result::Err(BuildError::Incomplete),
        };

        return Result::Ok(Response {
            code,
            reason,
            version,
            headers,
            body: self.body,
        });
    }
}
