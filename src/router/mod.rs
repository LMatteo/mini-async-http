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

    pub(crate) fn is_matching(&self, req: crate::Request) -> bool {
        self.routes.keys().find(|key| key.is_match(&req)).is_some()
    }

    pub(crate) fn add_route<T>(&mut self, route: route::Route, handler: T)
    where
        T: Send + Sync + 'static + Fn(&Request),
    {
        self.routes.insert(route, Box::from(handler));
    }
}
