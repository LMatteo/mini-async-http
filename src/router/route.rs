use crate::Method;
use crate::Request;

use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub(crate) struct Route {
    path: String,
    method: Method,
}

impl Route {
    pub(crate) fn new(path: &str, method: Method) -> Route {
        Route {
            path: String::from(path),
            method,
        }
    }
    pub(crate) fn is_match(&self, req: &Request) -> bool {
        &self.method == req.method() && &self.path == req.path()
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.method == other.method
    }
}

impl Eq for Route {}

impl Hash for Route {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::RequestBuilder;

    #[test]
    fn route_eq() {
        let a = Route::new("test", Method::GET);
        let b = Route::new("test", Method::GET);

        assert_eq!(a, b);
    }

    #[test]
    fn route_path_not_eq() {
        let a = Route::new("different", Method::GET);
        let b = Route::new("test", Method::GET);

        assert_ne!(a, b);
    }

    #[test]
    fn route_meth_not_eq() {
        let a = Route::new("test", Method::POST);
        let b = Route::new("test", Method::GET);

        assert_ne!(a, b);
    }

    #[test]
    fn route_meth_path_not_eq() {
        let a = Route::new("diff", Method::POST);
        let b = Route::new("test", Method::GET);

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

        let route = Route::new("/test/", Method::GET);

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

        let route = Route::new("/diff/", Method::GET);

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

        let route = Route::new("/test/", Method::POST);

        assert!(!route.is_match(&req));
    }
}
