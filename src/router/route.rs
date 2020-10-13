use crate::Method;
use crate::Request;

use regex::Regex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Route {
    path: Regex,
    parameters: Vec<String>,
    method: Method,
}

#[derive(Debug)]
pub enum RegexError {
    Build(regex::Error),
    Match,
}

fn route_to_regex(path: &str) -> Result<(Vec<String>, Regex), RegexError> {
    let re = match Regex::new(r"^(/[^/?]*)+$") {
        Ok(re) => re,
        Err(e) => return Err(RegexError::Build(e)),
    };

    if !re.is_match(path) {
        return Err(RegexError::Match);
    }

    let mut pattern = String::from("^");
    let mut args = Vec::new();

    path.split('/').for_each(|s| {
        if s.starts_with('{') && s.ends_with('}') {
            let s = s.trim_matches(|c| c == '{' || c == '}');
            pattern.push_str(&format!(r"/(?P<{}>[^/?]*)", s));
            args.push(String::from(s));
        } else if !s.is_empty() {
            pattern.push('/');
            pattern.push_str(s);
        }
    });

    if pattern.len() == 1 {
        pattern.push('/');
    }
    pattern.push('$');

    Ok((args, Regex::new(&pattern).unwrap()))
}

impl Route {
    pub fn new(path: &str, method: Method) -> Result<Route, RegexError> {
        let (parameters, reg) = match route_to_regex(path) {
            Ok((parameters, reg)) => (parameters, reg),
            Err(e) => return Err(e),
        };

        Ok(Route {
            path: reg,
            parameters,
            method,
        })
    }

    pub(crate) fn is_match(&self, req: &Request) -> bool {
        let path = req.path().trim_end_matches('/');
        &self.method == req.method() && self.path.is_match(path)
    }

    pub(crate) fn parse_request(&self, req: &Request) -> Option<HashMap<String, String>> {
        let path = req.path().trim_end_matches('/');
        let caps = match self.path.captures(path) {
            Some(caps) => caps,
            None => return None,
        };

        let parameters = self
            .parameters
            .iter()
            .filter_map(|name| {
                let val = match caps.name(name) {
                    Some(val) => String::from(val.as_str()),
                    None => return None,
                };

                Some((String::from(name), val))
            })
            .collect();

        Some(parameters)
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_str() == other.path.as_str() && self.method == other.method
    }
}

impl Eq for Route {}

impl Hash for Route {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.as_str().hash(state)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::RequestBuilder;

    #[test]
    fn route_eq() {
        let a = Route::new("/test", Method::GET).unwrap();
        let b = Route::new("/test", Method::GET).unwrap();

        assert_eq!(a, b);
    }

    #[test]
    fn route_path_not_eq() {
        let a = Route::new("/different", Method::GET).unwrap();
        let b = Route::new("/test", Method::GET).unwrap();

        assert_ne!(a, b);
    }

    #[test]
    fn route_meth_not_eq() {
        let a = Route::new("/test", Method::POST).unwrap();
        let b = Route::new("/test", Method::GET).unwrap();

        assert_ne!(a, b);
    }

    #[test]
    fn route_meth_path_not_eq() {
        let a = Route::new("/diff", Method::POST).unwrap();
        let b = Route::new("/test", Method::GET).unwrap();

        assert_ne!(a, b);
    }

    #[test]
    fn route_match() {
        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test/"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let route = Route::new("/test/", Method::GET).unwrap();

        assert!(route.is_match(&req));
    }

    #[test]
    fn route_path_not_match() {
        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test/"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let route = Route::new("/diff/", Method::GET).unwrap();

        assert!(!route.is_match(&req));
    }

    #[test]
    fn route_meth_not_match() {
        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test/"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let route = Route::new("/test/", Method::POST).unwrap();

        assert!(!route.is_match(&req));
    }

    #[test]
    fn route_parametrized() {
        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test/parameter"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let route = Route::new("/test/{param}", Method::GET).unwrap();

        assert!(route.is_match(&req));

        let params = route.parse_request(&req).unwrap();

        assert_eq!(params.get("param").unwrap(), "parameter")
    }

    #[test]
    fn route_multi_parameters() {
        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test/parameter/ids"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let route = Route::new("/{first}/{param}/{last}", Method::GET).unwrap();

        assert!(route.is_match(&req));

        let params = route.parse_request(&req).unwrap();

        assert_eq!(params.get("param").unwrap(), "parameter");
        assert_eq!(params.get("first").unwrap(), "test");
        assert_eq!(params.get("last").unwrap(), "ids");
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn simple_reg() {
        let (lst, reg) = route_to_regex("/test/test").unwrap();

        assert_eq!(lst.len(), 0);
        assert!(reg.is_match("/test/test"));
        assert!(!reg.is_match("/test/test/add"));
        assert!(!reg.is_match("/test"));
    }

    #[test]
    fn match_error() {
        let res = route_to_regex("wrongPath");

        assert!(res.is_err())
    }

    #[test]
    fn param_reg() {
        let (lst, reg) = route_to_regex("/{param}/test").unwrap();

        assert_eq!(lst.len(), 1);
        assert!(lst.contains(&String::from("param")));

        let cap = reg.captures("/test/test").unwrap();
        assert_eq!(cap.name("param").unwrap().as_str(), "test");
    }

    #[test]
    fn root_path_reg() {
        let (lst, reg) = route_to_regex("/").unwrap();

        assert_eq!(lst.len(), 0);

        assert!(reg.is_match("/"));
        assert!(!reg.is_match("/test"));
    }
}
