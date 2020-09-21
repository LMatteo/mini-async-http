pub mod route;

use crate::{Request, Response, ResponseBuilder};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Router {
    routes: HashMap<route::Route, Arc<dyn Send + Sync + 'static + Fn(&Request) -> Response>>,
}

impl Router {
    pub fn new() -> Router {
        Router {
            routes: HashMap::new(),
        }
    }

    pub fn is_matching(&self, req: &crate::Request) -> bool {
        self.routes.keys().any(|key| key.is_match(&req))
    }

    pub fn add_route<T>(&mut self, route: route::Route, handler: T)
    where
        T: Send + Sync + 'static + std::ops::Fn(&Request) -> Response,
    {
        self.routes.insert(route, Arc::from(handler));
    }

    pub fn exec(&self, req: &crate::Request) -> Response {
        if let Some((_, handler)) = self.routes.iter().find(|(route, _)| route.is_match(req)) {
            return handler(req);
        }

        ResponseBuilder::empty_404().build().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::RequestBuilder;
    use crate::Method;

    #[test]
    fn router_match() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), |req| {
            ResponseBuilder::empty_200().body(b"test").build().unwrap()
        });

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        assert!(router.is_matching(&req));
    }

    #[test]
    fn router_no_match() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), |req| {
            ResponseBuilder::empty_200().body(b"test").build().unwrap()
        });

        let req = RequestBuilder::new()
            .method(Method::POST)
            .path(String::from("/test/diff"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        assert!(!router.is_matching(&req));
    }

    #[test]
    fn router_exec_single_route() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            ResponseBuilder::empty_200().body(b"test").build().unwrap()
        });

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 200);
        assert_eq!(response.body().unwrap(), b"test");
    }

    #[test]
    fn router_exec_double_route_path() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            ResponseBuilder::empty_200().body(b"test").build().unwrap()
        });

        router.add_route(route::Route::new("/test2", Method::GET), move |_req| {
            ResponseBuilder::empty_200().body(b"test2").build().unwrap()
        });

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 200);
        assert_eq!(response.body().unwrap(), b"test");

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test2"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 200);
        assert_eq!(response.body().unwrap(), b"test2");
    }

    #[test]
    fn router_exec_double_route_method() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            ResponseBuilder::empty_200().body(b"GET").build().unwrap()
        });

        router.add_route(route::Route::new("/test", Method::POST), move |_req| {
            ResponseBuilder::empty_200().body(b"POST").build().unwrap()
        });

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 200);
        assert_eq!(response.body().unwrap(), b"GET");

        let req = RequestBuilder::new()
            .method(Method::POST)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 200);
        assert_eq!(response.body().unwrap(), b"POST");
    }

    #[test]
    fn router_add_same_route() {
        let mut router = Router::new();

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            ResponseBuilder::empty_200().build().unwrap()
        });

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            ResponseBuilder::empty_200().build().unwrap()
        });

        assert_eq!(router.routes.len(), 1)
    }

    #[test]
    fn router_missing_route() {
        let mut router = Router::new();

        let req = RequestBuilder::new()
            .method(Method::POST)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 404);
    }
}
