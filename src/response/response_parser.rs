use regex::Regex;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use crate::http;

use crate::http::ParseError;
use crate::http::Version;
use crate::response::Response;
use crate::response::ResponseBuilder;

pub struct ResponseParser {
    firstRe: Regex,
    parser: http::Parser,
}

impl ResponseParser {
    pub fn new_parser() -> ResponseParser {
        return ResponseParser {
            firstRe: Regex::new(
                r"(?x)(?P<version>[^\x20]+)\x20(?P<code>[^\x20]+)\x20(?P<reason>.+)\r\n",
            )
            .unwrap(),
            parser: http::Parser::new(),
        };
    }

    pub fn parse(&self, stream: &mut dyn Read) -> Result<Response, ParseError> {
        let mut reader = BufReader::new(stream);
        let mut builder = ResponseBuilder::new();

        let mut buf = String::new();
        let builder = match reader.read_line(&mut buf) {
            Ok(_) => {
                let caps = match self.firstRe.captures(buf.as_str()) {
                    Some(caps) => caps,
                    None => return Result::Err(ParseError::FirstLine),
                };

                let code = caps.name("code").unwrap().as_str();
                let reason = caps.name("reason").unwrap().as_str();

                builder.code(match code.parse() {
                    Ok(val) => val,
                    Err(_) => return Result::Err(ParseError::CodeParseError),
                }).version(
                    match Version::from_str(caps.name("version").unwrap().as_str()) {
                        Some(version) => version,
                        None => return Result::Err(ParseError::WrongVersion),
                    },
                ).reason(String::from(reason))
            }
            Err(e) => return Result::Err(ParseError::ReadError(e)),
        };

        let builder = match self.parser.parse(&mut reader) {
            Err(e) => return Result::Err(e),
            Ok((headers, Some(body), _)) => {
                builder.headers(headers).body(body)
            }
            Ok((headers, None, _)) => {
                builder.headers(headers)
            }
        };

        return match builder.build() {
            Ok(request) => Result::Ok(request),
            Err(e) => Result::Err(ParseError::BuilderError(e)),
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn get_resource(path: &str) -> impl Read {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d.push(path);

        return File::open(d).unwrap();
    }

    #[test]
    fn parse() {
        let parser = ResponseParser::new_parser();
        let mut input = get_resource("response.txt");

        let a = parser.parse(&mut input).unwrap();

        let mut reader = Cursor::new(a.to_string());

        let b = parser.parse(&mut reader).unwrap();

        assert_eq!(a, b);
    }
}
