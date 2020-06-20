#[derive(Debug, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
        }
    }

    pub fn from_str(method: &str) -> Option<Method> {
        match method {
            "GET" => Option::Some(Method::GET),
            "POST" => Option::Some(Method::POST),
            "DELETE" => Option::Some(Method::DELETE),
            "PUT" => Option::Some(Method::PUT),
            _ => Option::None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn as_str() {
        assert_eq!(Method::GET.as_str(), "GET");
        assert_eq!(Method::PUT.as_str(), "PUT");
        assert_eq!(Method::DELETE.as_str(), "DELETE");
        assert_eq!(Method::POST.as_str(), "POST");
    }
}
