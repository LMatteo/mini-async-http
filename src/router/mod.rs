pub mod route;

use crate::{Request, Response, ResponseBuilder};

use std::collections::HashMap;
use std::sync::Arc;

type RouteMap = HashMap<
    route::Route,
    Arc<dyn Send + Sync + 'static + Fn(&Request, HashMap<String, String>) -> Response>,
>;

#[derive(Clone)]
pub struct Router {
    routes: RouteMap,
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
        T: Send + Sync + 'static + std::ops::Fn(&Request, HashMap<String, String>) -> Response,
    {
        self.routes.insert(route, Arc::from(handler));
    }

    pub fn exec(&self, req: &crate::Request) -> Response {
        if let Some((route, handler)) = self.routes.iter().find(|(route, _)| route.is_match(req)) {
            let parameters = match route.parse_request(req) {
                Some(param) => param,
                None => return ResponseBuilder::empty_500().build().unwrap(),
            };
            return handler(req, parameters);
        }

        ResponseBuilder::empty_404().build().unwrap()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::RequestBuilder;
    use crate::Method;
    use crate::Version;

    #[test]
    fn router_match() {
        let mut router = Router::new();

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            |_req, _| ResponseBuilder::empty_200().body(b"test").build().unwrap(),
        );

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

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            |_req, _| ResponseBuilder::empty_200().body(b"test").build().unwrap(),
        );

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

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().body(b"test").build().unwrap(),
        );

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

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().body(b"test").build().unwrap(),
        );

        router.add_route(
            route::Route::new("/test2", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().body(b"test2").build().unwrap(),
        );

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

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().body(b"GET").build().unwrap(),
        );

        router.add_route(
            route::Route::new("/test", Method::POST).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().body(b"POST").build().unwrap(),
        );

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

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().build().unwrap(),
        );

        router.add_route(
            route::Route::new("/test", Method::GET).unwrap(),
            move |_req, _| ResponseBuilder::empty_200().build().unwrap(),
        );

        assert_eq!(router.routes.len(), 1)
    }

    #[test]
    fn router_missing_route() {
        let router = Router::new();

        let req = RequestBuilder::new()
            .method(Method::POST)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let response = router.exec(&req);

        assert_eq!(response.code(), 404);
    }

    #[test]
    fn parametrized_route() {
        let mut router = Router::new();

        router.add_route(
            route::Route::new("/router/parametrized/{parameter}", Method::GET).unwrap(),
            move |_req, params| {
                let val = params.get("parameter").unwrap();
                let len = val.as_bytes().len();

                let builder = ResponseBuilder::new()
                    .code(200)
                    .reason(String::from("OK"))
                    .version(Version::HTTP11)
                    .body(val.as_bytes())
                    .header("Content-Type", "text/plain")
                    .header("Content-Length", &len.to_string());

                let response = builder.build().unwrap();

                return response;
            },
        );

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/router/parametrized/myParam"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        let resp = router.exec(&req);

        assert_eq!(resp.body().unwrap(), b"myParam");
    }
}
