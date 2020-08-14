use crate::executor::new_executor_and_spawner;
use crate::executor::Executor;
use crate::executor::Spawner;
use crate::io::reactor::Handle;
use crate::io::reactor::Reactor;
use crate::executor::thread_pool::{PoolHandle,PoolError,ThreadPoolBuilder};


use std::cell::RefCell;
use std::future::Future;

thread_local! {
    static HANDLE : RefCell<Option<Handle>> = RefCell::from(None);
    static EXECUTOR : RefCell<Option<PoolHandle>> = RefCell::from(None);
}

pub(crate) fn start() {
    let mut reactor = Reactor::new();

    let reactor_handle = reactor.handle();
    set_handle(reactor_handle.try_clone().expect("Reactor could not start"));

    std::thread::spawn(move || {
        reactor.event_loop();
    });

    let pool = ThreadPoolBuilder::new()
                .size(num_cpus::get_physical())
                .after_start(move |_,handle|{
                    set_pool(handle);
                    set_handle(reactor_handle.try_clone().expect("Reactor could not start"));
                })
                .build();
    
    set_pool(pool);
}

pub(crate) fn handle() -> Option<Handle> {
    HANDLE.with(|ctx| match *ctx.borrow() {
        Some(ref handle) => handle.try_clone().ok(),
        None => None,
    })
}

fn set_handle(handle: Handle) {
    HANDLE.with(|ctx| ctx.replace(Some(handle)));
}

fn set_pool(pool: PoolHandle){
    EXECUTOR.with(|ctx| ctx.replace(Some(pool)));
}

pub(crate) fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        Some(ref spawner) => {spawner.spawn(future);},
        _ => {},
    });
}

pub(crate) fn block_on<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        Some(ref spawner) => {spawner.block_on(future);},
        _ => {},
    });
}

pub(crate) fn stop() {
    EXECUTOR.with(|ctx| match *ctx.borrow() {
        Some(ref spawner) => {spawner.stop();},
        _ => {},
    });
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
