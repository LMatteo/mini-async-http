use crate::http::Version;
use crate::http::{Headers, ParseError};
use crate::response::Response;
use crate::response::ResponseBuilder;

pub struct ResponseParser {}

impl ResponseParser {
    pub fn new() -> ResponseParser {
        return ResponseParser {};
    }

    pub fn parse_u8(&self, reader: &[u8]) -> Result<(Response, usize), ParseError> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut resp = httparse::Response::new(&mut headers);

        let res = match resp.parse(reader) {
            Ok(httparse::Status::Partial) => return Err(ParseError::UnexpectedEnd),
            Ok(httparse::Status::Complete(n)) => n,
            Err(e) => return Err(ParseError::from(e)),
        };

        let mut builder = ResponseBuilder::new()
            .code(resp.code.unwrap().into())
            .reason(String::from(resp.reason.unwrap()))
            .version(Version::HTTP11);

        let mut headers = Headers::new();

        for header in resp.headers {
            let name = String::from(header.name);
            let val = String::from_utf8(header.value.to_vec()).unwrap();

            headers.set_header(&name, &val)
        }

        let length = match headers.get_header(&String::from("Content-length")) {
            Some(n) => n,
            None => {
                builder = builder.headers(headers);
                let request = match builder.build() {
                    Ok(req) => req,
                    Err(e) => return Err(ParseError::BuilderError(e)),
                };

                return Ok((request, res));
            }
        };

        let length = match length.parse::<usize>() {
            Ok(val) => val,
            Err(_e) => return Err(ParseError::LengthParse),
        };

        if reader.len() < res + length {
            return Err(ParseError::UnexpectedEnd);
        }

        let body = &reader[res..res + length];
        let builder = builder.body(body).headers(headers);

        let request = match builder.build() {
            Ok(req) => req,
            Err(e) => return Err(ParseError::BuilderError(e)),
        };

        return Ok((request, res + length));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn get_resource_string(path: &str) -> String {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d.push(path);

        return fs::read_to_string(d).unwrap();
    }

    #[test]
    fn parse() {
        let parser = ResponseParser::new();
        let input = get_resource_string("response.txt").as_bytes().to_vec();

        let (a, _) = parser.parse_u8(&input).unwrap();

        let reader = a.to_string().as_bytes().to_vec();

        let (b, _) = parser.parse_u8(&reader).unwrap();

        assert_eq!(a, b);
    }
}
