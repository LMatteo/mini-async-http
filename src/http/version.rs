use std::str::FromStr;

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
}

impl FromStr for Version{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/1.1" => Ok(Version::HTTP11),
            _ => Err(()),
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
            Version::HTTP11 => {}
        }
    }
}
