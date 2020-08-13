use crossbeam_channel::{bounded, Receiver, Sender};
use slab::Slab;

use std::sync::Arc;

use std::task::Waker;

use crossbeam_utils::atomic::AtomicCell;

const DEFAULT_SLAB_SIZE: usize = 4096;
const DEFAULT_EVENTS_SIZE: usize = 4096;

pub(crate) struct Reactor {
    poll: mio::Poll,
    events: mio::Events,

    io_wakers: Slab<Arc<IoWaker>>,

    id_sender: Sender<Arc<IoWaker>>,
    id_receiver: Receiver<Arc<IoWaker>>,

    waker: Arc<mio::Waker>,
    waker_token: usize,
}

impl Reactor {
    pub(crate) fn new() -> Reactor {
        let poll = mio::Poll::new().unwrap();
        let events = mio::Events::with_capacity(DEFAULT_EVENTS_SIZE);

        let mut io_wakers = Slab::with_capacity(DEFAULT_SLAB_SIZE);
        let (id_sender, id_receiver) = bounded(DEFAULT_SLAB_SIZE);

        let waker_entry = io_wakers.vacant_entry();
        let waker_token = waker_entry.key();
        waker_entry.insert(Arc::from(IoWaker::new(waker_token)));

        let waker = Arc::new(mio::Waker::new(poll.registry(), mio::Token(waker_token)).unwrap());

        while io_wakers.len() < io_wakers.capacity() {
            let entry = io_wakers.vacant_entry();
            let waker = Arc::from(IoWaker::new(entry.key()));
            entry.insert(waker.clone());

            id_sender.send(waker).unwrap();
        }

        Reactor {
            poll,
            events,
            io_wakers,
            id_sender,
            id_receiver,
            waker,
            waker_token,
        }
    }

    pub(crate) fn event_loop(&mut self) {
        loop {
            self.turn();
        }
    }

    fn turn(&mut self) {
        self.poll.poll(&mut self.events, None).unwrap();

        for event in self.events.iter() {
            self.handle_event(event);
        }
    }

    fn handle_event(&self, event: &mio::event::Event) {
        if event.token().0 == self.waker_token {
            return;
        }

        if let Some(waker) = self.io_wakers.get(event.token().0) {
            match waker.take() {
                Some(val) => val.wake(),
                None => return,
            }
        }
    }

    pub(crate) fn handle(&self) -> Handle {
        Handle {
            id_receiver: self.id_receiver.clone(),
            id_sender: self.id_sender.clone(),
            registry: self.poll.registry().try_clone().unwrap(),
        }
    }
}

pub(crate) struct Handle {
    id_receiver: Receiver<Arc<IoWaker>>,
    id_sender: Sender<Arc<IoWaker>>,
    registry: mio::Registry,
}

impl Handle {
    pub(crate) fn register(&self, source: &mut dyn mio::event::Source) -> Arc<IoWaker> {
        let waker = self.id_receiver.try_recv().expect("No id available");

        self.registry
            .register(source, mio::Token(waker.key()), mio::Interest::READABLE)
            .unwrap();

        waker
    }

    pub(crate) fn deregister(&self, source: &mut dyn mio::event::Source, waker: Arc<IoWaker>) {
        self.registry.deregister(source).unwrap();
        self.id_sender.send(waker).unwrap();
    }

    pub(crate) fn try_clone(&self) -> std::io::Result<Self> {
        let registry = self.registry.try_clone()?;

        Ok(Handle {
            id_receiver: self.id_receiver.clone(),
            id_sender: self.id_sender.clone(),
            registry,
        })
    }
}

enum CloneError {}

enum Message {
    DelSource(usize),
}

pub(crate) struct IoWaker {
    key: usize,
    waker: AtomicCell<Option<Waker>>,
}

impl IoWaker {
    fn new(key: usize) -> IoWaker {
        IoWaker {
            key,
            waker: AtomicCell::new(None),
        }
    }

    pub fn key(&self) -> usize {
        self.key
    }

    pub fn take(&self) -> Option<Waker> {
        self.waker.take()
    }

    pub fn set_waker(&self, waker: Waker) {
        self.waker.store(Some(waker));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init() {
        let reactor = Reactor::new();

        assert_eq!(reactor.io_wakers.len(), DEFAULT_SLAB_SIZE);
        assert_eq!(reactor.io_wakers.len(), reactor.io_wakers.capacity());
    }

    #[test]
    fn empty_waker() {
        let waker = IoWaker::new(0);
        assert!(waker.take().is_none());
    }

    #[test]
    fn register() {
        let reactor = Reactor::new();
        let handle = reactor.handle();

        assert_eq!(DEFAULT_SLAB_SIZE - 1, reactor.id_receiver.len());
        assert_eq!(DEFAULT_SLAB_SIZE - 1, reactor.id_sender.len());

        let mut stream = mio::net::TcpListener::bind("0.0.0.0:29808".parse().unwrap()).unwrap();

        let waker = handle.register(&mut stream);

        assert_eq!(DEFAULT_SLAB_SIZE - 2, reactor.id_receiver.len());
        assert_eq!(DEFAULT_SLAB_SIZE - 2, reactor.id_sender.len());

        handle.deregister(&mut stream, waker);

        assert_eq!(DEFAULT_SLAB_SIZE - 1, reactor.id_receiver.len());
        assert_eq!(DEFAULT_SLAB_SIZE - 1, reactor.id_sender.len());
    }
}
