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
    pub body: Option<String>,
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

        match &self.body {
            Some(body) => buf.push_str(body.as_str()),
            None => {}
        };

        write!(f, "{}", buf)
    }
}

impl Response {
    pub fn empty_500() -> Response {
        Response {
            code: Reason::INTERNAL500.code(),
            reason: Reason::INTERNAL500.reason(),
            headers: Headers::new_headers(),
            version: Version::HTTP11,
            body: None,
        }
    }

    pub fn empty_200() -> Response {
        Response {
            code: Reason::OK200.code(),
            reason: Reason::OK200.reason(),
            headers: Headers::new_headers(),
            version: Version::HTTP11,
            body: None,
        }
    }

    pub fn empty_400() -> Response {
        Response {
            code: Reason::BADREQUEST400.code(),
            reason: Reason::BADREQUEST400.reason(),
            headers: Headers::new_headers(),
            version: Version::HTTP11,
            body: None,
        }
    }

    pub fn set_header(&mut self, key: &str, value: &str) {
        let key = &String::from(key);
        let value = &String::from(value);

        self.headers.set_header(key, value);
    }

    pub fn get_code(&self) -> i32 {
        self.code
    }
}

pub struct ResponseBuilder {
    code: Option<i32>,
    reason: Option<String>,
    version: Option<Version>,
    headers: Option<Headers>,
    body: Option<String>,
}

impl ResponseBuilder {
    pub fn new_builder() -> ResponseBuilder {
        ResponseBuilder {
            code: Option::None,
            reason: Option::None,
            version: Option::Some(Version::HTTP11),
            headers: Option::Some(Headers::new_headers()),
            body: Option::None,
        }
    }

    pub fn set_code(&mut self, code: i32) -> &mut ResponseBuilder {
        self.code = Option::Some(code);
        self
    }

    pub fn set_reason(&mut self, reason: String) -> &mut ResponseBuilder {
        self.reason = Option::Some(reason);
        self
    }

    pub fn set_version(&mut self, version: Version) -> &mut ResponseBuilder {
        self.version = Option::Some(version);
        self
    }

    pub fn set_headers(&mut self, headers: Headers) -> &mut ResponseBuilder {
        self.headers = Option::Some(headers);
        self
    }

    pub fn set_header(&mut self, key: &str, value: &str) -> &mut ResponseBuilder {
        let key = &String::from(key);
        let value = &String::from(value);

        match self.headers.as_mut() {
            Some(headers) => headers.set_header(key, value),
            None => {
                let mut headers = Headers::new_headers();
                headers.set_header(key, value);
                self.headers = Some(headers);
            }
        };

        self
    }

    pub fn set_body(&mut self, body: String) -> &mut ResponseBuilder {
        self.body = Option::Some(body);
        self
    }

    pub fn set_status(&mut self, status: Reason) -> &mut ResponseBuilder {
        self.code = Some(status.code());
        self.reason = Some(status.reason());

        self
    }

    pub fn build(self) -> Result<Response, BuildError> {
        let code = match self.code {
            Some(val) => val,
            None => return Result::Err(BuildError::incomplete),
        };

        let reason = match self.reason {
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

        return Result::Ok(Response {
            code,
            reason,
            version,
            headers,
            body: self.body,
        });
    }
}
