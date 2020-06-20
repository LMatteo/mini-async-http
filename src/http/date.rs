use chrono::prelude::*;
use std::fmt;

pub struct HTTPDate {
    d: DateTime<Utc>,
}

impl HTTPDate {
    pub fn new() -> HTTPDate {
        HTTPDate { d: Utc::now() }
    }
}

impl fmt::Display for HTTPDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.d.format("%a, %e %b %Y %H:%M:%S GMT"))
    }
}
