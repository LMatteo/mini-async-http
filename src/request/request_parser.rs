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
use crate::http::Headers;

pub struct RequestParser {
    firstRe: Regex,
    parser: http::Parser,
}

impl RequestParser {
    pub fn new() -> RequestParser {
        return RequestParser {
            firstRe: Regex::new(r"(?x)(?P<method>.+)\x20(?P<path>.+)\x20(?P<version>.+)\r\n")
                .unwrap(),
            parser: http::Parser::new(),
        };
    }

    pub fn parse_u8(&self, reader: &[u8]) -> Result<(Request, usize), ParseError> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        let res = match req.parse(reader) {
            Ok(httparse::Status::Partial) => return Err(ParseError::UnexpectedEnd),
            Ok(httparse::Status::Complete(n)) => n,
            Err(e) => return Err(ParseError::from(e)),
        };

        let mut builder = RequestBuilder::new()
            .method(Method::from_str(req.method.unwrap()).unwrap())
            .path(String::from(req.path.unwrap()))
            .version(Version::HTTP11);

        let mut headers = Headers::new();
        
        for header in req.headers{
            let name = String::from(header.name);
            let val = String::from_utf8(header.value.to_vec()).unwrap();

            headers.set_header(&name, &val)
        };

        let length = match headers.get_header(&String::from("Content-length")) {
            Some(n) => n,
            None => {
                builder = builder.headers(headers);
                let request = match builder.build() {
                    Ok(req) => req,
                    Err(e) => return Err(ParseError::BuilderError(e)),
                };

                return Ok((request,res))
            }
        };

        let length = match length.parse::<usize>(){
            Ok(val) => val,
            Err(e) => return Err(ParseError::LengthParse)
        };

        if reader.len() < res + length {
            return Err(ParseError::UnexpectedEnd);
        }

        let body = &reader[res..res+length];
        let builder = builder.body(body.to_vec());
        let builder = builder.headers(headers);

        let request = match builder.build() {
            Ok(req) => req,
            Err(e) => return Err(ParseError::BuilderError(e)),
        };

        return Ok((request,res + length))

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

    fn get_resource_string(path: &str) -> String {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d.push(path);

        return fs::read_to_string(d).unwrap();
    }

    #[test]
    fn print() {
        let parser = RequestParser::new();
        let input = get_resource_string("http_request.txt").as_bytes().to_vec();
        let (a, _) = parser.parse_u8(&input).expect("Error when parsing");

        let reader = a.to_string().as_bytes().to_vec();

        let (b, _) = parser.parse_u8(&reader).expect("Error when parsing");

        assert_eq!(a, b);
    }

    #[test]
    fn print_with_body() {
        let parser = RequestParser::new();
        let mut input = get_resource_string("http_body.txt").as_bytes().to_vec();
        let (a, _) = parser.parse_u8(&input).expect("Error when parsing");

        let mut reader = a.to_string().as_bytes().to_vec();

        let (b, _) = parser.parse_u8(&reader).expect("Error when parsing");

        assert_eq!(a, b);
        assert_eq!(a.body_as_string().unwrap(), String::from("teststststststst"));
    }

    #[test]
    fn from_u8() {
        let parser = RequestParser::new();
        let mut input = get_resource_string("http_request.txt").as_bytes().to_vec();
        let (request, n) = parser.parse_u8(&input).expect("Error when parsing");

        assert_eq!(78, n);
        assert_eq!(*request.method(), Method::GET);
        assert_eq!(request.path().as_str(), "/");
        assert_eq!(*request.version(), Version::HTTP11);

        assert_eq!(
            request
                .headers()
                .get_header(&String::from("host"))
                .unwrap()
                .as_str(),
            "localhost:8080"
        );
        assert_eq!(
            request
                .headers()
                .get_header(&String::from("Accept"))
                .unwrap()
                .as_str(),
            "*/*"
        );
        assert_eq!(
            request
                .headers()
                .get_header(&String::from("user-agent"))
                .unwrap()
                .as_str(),
            "curl/7.54.0"
        );

        match request.body() {
            Some(_) => panic!(),
            _ => {}
        }
    }

    #[test]
    fn partial() {
        let  input = get_resource_string("http_body.txt");
        let  input = input.as_bytes();
        let parser = RequestParser::new();
        let mut body = Vec::new();

        for byte in 0..input.len() -1 {
            body.push(input[byte]);

            match parser.parse_u8(&body) {
                Ok(_) => panic!("Should not be ok"),
                Err(ParseError::UnexpectedEnd) => {},
                Err(e) => panic!("Wrong error kind {:?}",e),
            }
        }

        body.push(input[input.len() -1]);

        match parser.parse_u8(&body) {
            Ok(_) => {},
            Err(e) => panic!("Should be ok got error {:?}",e),
        }
    }

    #[test]
    fn first_line_error(){
        let input = b"zaezaexq\r\n";
        let parser = RequestParser::new();

        match parser.parse_u8(input) {
            Ok(_) => panic!("Should have first line error"),
            Err(_) => {},
        }
    }

}
