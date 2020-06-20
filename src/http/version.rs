#[derive(Debug, PartialEq)]
pub enum Version {
    HTTP11,
}

impl Version {
    pub fn as_str(&self) -> &str {
        match self {
            Version::HTTP11 => "HTTP/1.1",
        }
    }

    pub fn from_str(version: &str) -> Option<Version> {
        match version {
            "HTTP/1.1" => Option::Some(Version::HTTP11),
            _ => Option::None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn as_str() {
        assert_eq!(Version::HTTP11.as_str(), "HTTP/1.1")
    }

    #[test]
    fn from_str() {
        let version = Version::from_str("HTTP/1.1").unwrap();

        match version {
            Version::HTTP11 => {},
        }
    }
}
