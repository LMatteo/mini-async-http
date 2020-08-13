use crate::executor::new_executor_and_spawner;
use crate::executor::Executor;
use crate::executor::Spawner;
use crate::io::reactor::Handle;
use crate::io::reactor::Reactor;

use std::cell::RefCell;
use std::future::Future;

thread_local! {
    static HANDLE : RefCell<Option<Handle>> = RefCell::from(None);
    static EXECUTOR : RefCell<(Option<Executor>, Option<Spawner>)> = RefCell::from((None,None));
}

pub(crate) fn start() {
    let mut reactor = Reactor::new();

    EXECUTOR.with(|ctx| {
        let (exec, spawner) = new_executor_and_spawner();
        ctx.replace((Some(exec), Some(spawner)));
    });

    set_handle(reactor.handle());

    std::thread::spawn(move || {
        reactor.event_loop();
    });
}

pub(crate) fn handle() -> Option<Handle> {
    HANDLE.with(|ctx| match *ctx.borrow() {
        Some(ref handle) => handle.try_clone().ok(),
        None => None,
    })
}

pub(crate) fn set_handle(handle: Handle) {
    HANDLE.with(|ctx| ctx.replace(Some(handle)));
}

pub(crate) fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        (_, Some(ref spawner)) => spawner.spawn(future),
        (_, _) => return,
    })
}

pub(crate) fn block_on<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        (Some(ref exec), Some(ref spawner)) => {
            spawner.spawn(future);
            exec.run();
        }
        (_, _) => return,
    })
}

pub(crate) fn stop() {
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        (_, Some(ref spawner)) => {
            spawner.stop();
        }
        (_, _) => return,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_context() {
        assert!(handle().is_none());
    }

    #[test]
    fn start_context() {
        start();
        assert!(handle().is_some());
    }

    #[test]
    fn start_multithread() {
        start();
        let h = handle().unwrap();

        std::thread::spawn(move || {
            assert!(handle().is_none());

            set_handle(h.try_clone().unwrap());

            assert!(handle().is_some());
        });
    }
}
