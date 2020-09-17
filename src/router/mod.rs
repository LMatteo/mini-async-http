mod route;

use crate::Request;
use std::collections::HashMap;

pub(crate) struct Router {
    routes: HashMap<route::Route, Box<dyn Send + Sync + 'static + Fn(&Request)>>,
}

impl Router {
    pub(crate) fn new() -> Router {
        Router {
            routes: HashMap::new(),
        }
    }

    pub(crate) fn is_matching(&self, req: &crate::Request) -> bool {
        self.routes.keys().any(|key| key.is_match(&req))
    }

    pub(crate) fn add_route<T>(&mut self, route: route::Route, handler: T)
    where
        T: Send + Sync + 'static + Fn(&Request),
    {
        self.routes.insert(route, Box::from(handler));
    }

    pub(crate) fn exec(&self, req: &crate::Request) {
        if let Some((_, handler)) = self.routes.iter().find(|(route, _)| route.is_match(req)) {
            handler(req);
        }
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

        router.add_route(route::Route::new("/test", Method::GET), |req| {});

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

        router.add_route(route::Route::new("/test", Method::GET), |req| {});

        let req = RequestBuilder::new()
            .method(Method::POST)
            .path(String::from("/test/diff"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        assert!(!router.is_matching(&req));
    }

    #[test]
    fn router_exec() {
        let mut router = Router::new();

        let (sender, receiver) = crossbeam_channel::unbounded();

        router.add_route(route::Route::new("/test", Method::GET), move |_req| {
            sender.send(()).unwrap();
        });

        let req = RequestBuilder::new()
            .method(Method::GET)
            .path(String::from("/test"))
            .version(crate::Version::HTTP11)
            .build()
            .expect("Error when building request");

        router.exec(&req);

        assert!(receiver.try_recv().is_ok())
    }
}
