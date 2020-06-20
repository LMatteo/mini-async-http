use regex::Regex;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;

use crate::http;

use crate::http::Method;
use crate::http::ParseError;
use crate::http::Version;
use crate::request::Request;
use crate::request::RequestBuilder;

pub struct RequestParser {
    firstRe: Regex,
    parser: http::Parser,
}

impl RequestParser {
    pub fn new_parser() -> RequestParser {
        return RequestParser {
            firstRe: Regex::new(r"(?x)(?P<method>.+)\x20(?P<path>.+)\x20(?P<version>.+)\r\n")
                .unwrap(),
            parser: http::Parser::new(),
        };
    }

    pub fn parse_read(&self, reader: &mut dyn Read) -> Result<(Request, usize), ParseError> {
        let mut buffer = BufReader::new(reader);
        self.parse(&mut buffer)
    }

    pub fn parse_u8(&self, reader: &Vec<u8>) -> Result<(Request, usize), ParseError> {
        let mut buffer = Cursor::new(reader);
        self.parse(&mut buffer)
    }

    pub fn parse(&self, reader: &mut dyn BufRead) -> Result<(Request, usize), ParseError> {
        let mut builder = RequestBuilder::new_builder();
        let mut nb = 0;

        let mut buf = String::new();
        match reader.read_line(&mut buf) {
            Ok(0) => return Err(ParseError::UnexpectedEnd),
            Ok(n) => {
                if !buf.ends_with("\r\n") {
                    return Err(ParseError::UnexpectedEnd);
                }

                nb += n;
                let caps = match self.firstRe.captures(buf.as_str()) {
                    Some(caps) => caps,
                    None => return Result::Err(ParseError::FirstLine),
                };

                let method = caps.name("method").unwrap().as_str();
                builder.set_method(match Method::from_str(method) {
                    Some(method) => method,
                    None => return Result::Err(ParseError::WrongMethod),
                });

                builder.set_version(
                    match Version::from_str(caps.name("version").unwrap().as_str()) {
                        Some(version) => version,
                        None => return Result::Err(ParseError::WrongVersion),
                    },
                );

                builder.set_path(String::from(caps.name("path").unwrap().as_str()));
            }
            Err(e) => return Result::Err(ParseError::ReadError(e)),
        }

        match self.parser.parse(reader) {
            Err(e) => return Result::Err(e),
            Ok((headers, Some(body), size)) => {
                nb += size;
                builder.set_headers(headers);
                builder.set_body(body);
            }
            Ok((headers, None, size)) => {
                nb += size;
                builder.set_headers(headers)
            }
        };

        return match builder.build() {
            Ok(request) => Result::Ok((request, nb)),
            Err(e) => Result::Err(ParseError::BuilderError(e)),
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::io::Cursor;
    use std::io::Read;
    use std::path::PathBuf;
    use std::string::ToString;

    fn get_resource(path: &str) -> impl Read {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d.push(path);

        return File::open(d).unwrap();
    }

    fn get_resource_string(path: &str) -> String {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d.push(path);

        return fs::read_to_string(d).unwrap();
    }

    #[test]
    fn read() {
        let parser = RequestParser::new_parser();
        let mut input = get_resource("http_request.txt");
        let (request, n) = parser.parse_read(&mut input).expect("Error when parsing");

        assert_eq!(78, n);
        assert_eq!(*request.get_method(), Method::GET);
        assert_eq!(request.get_path().as_str(), "/");
        assert_eq!(*request.get_version(), Version::HTTP11);

        assert_eq!(
            request
                .get_headers()
                .get_header(&String::from("host"))
                .unwrap()
                .as_str(),
            "localhost:8080"
        );
        assert_eq!(
            request
                .get_headers()
                .get_header(&String::from("Accept"))
                .unwrap()
                .as_str(),
            "*/*"
        );
        assert_eq!(
            request
                .get_headers()
                .get_header(&String::from("user-agent"))
                .unwrap()
                .as_str(),
            "curl/7.54.0"
        );

        match request.get_body() {
            Some(_) => panic!(),
            _ => {}
        }
    }

    #[test]
    fn print() {
        let parser = RequestParser::new_parser();
        let mut input = get_resource("http_request.txt");
        let (a, _) = parser.parse_read(&mut input).expect("Error when parsing");

        let mut reader = Cursor::new(a.to_string());

        let (b, _) = parser.parse(&mut reader).expect("Error when parsing");

        assert_eq!(a, b);
    }

    #[test]
    fn print_with_body() {
        let parser = RequestParser::new_parser();
        let mut input = get_resource("http_body.txt");
        let (a, _) = parser.parse_read(&mut input).expect("Error when parsing");

        let mut reader = Cursor::new(a.to_string());

        let (b, _) = parser.parse(&mut reader).expect("Error when parsing");

        assert_eq!(a, b);
        assert_eq!(a.get_body().unwrap(), &String::from("teststststststst"));
    }
}
