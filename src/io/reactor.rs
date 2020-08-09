use mio;
use slab::Slab;

use std::task::Waker;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;


const DEFAULT_SLAB_SIZE : usize = 4096;
const DEFAULT_EVENTS_SIZE : usize = 4096;

pub(crate) struct Reactor{
    poll : mio::Poll,
    events: mio::Events,

    io_wakers: Slab<Arc<IoWaker>>,

    id_sender: mpsc::Sender<Arc<IoWaker>>,
    id_receiver: Arc<Mutex<mpsc::Receiver<Arc<IoWaker>>>>,

    waker: Arc<mio::Waker>,
    waker_token: usize,

    message_receiver: mpsc::Receiver<Message>,
    message_sender: mpsc::Sender<Message>
}

impl Reactor{
    pub(crate) fn new() -> Reactor{
        let poll = mio::Poll::new().unwrap();
        let events = mio::Events::with_capacity(DEFAULT_EVENTS_SIZE);

        let mut io_wakers = Slab::with_capacity(DEFAULT_SLAB_SIZE);
        let(id_sender,id_receiver) = mpsc::channel();
        let (message_sender, message_receiver) = mpsc::channel();

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

        let id_receiver = Arc::from(Mutex::from(id_receiver));
        Reactor{
            poll,
            events,
            io_wakers,
            id_sender,
            id_receiver,
            waker,
            waker_token,
            message_receiver,
            message_sender,
        }

    }

    pub(crate) fn event_loop(&mut self) {
        loop {self.turn();}
    }

    fn turn(&mut self) {
        self.poll.poll(&mut self.events, None).unwrap();

        for event in self.events.iter() {
            self.handle_event(event);
        }
    }

    fn handle_event(&self, event: &mio::event::Event){
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
        Handle{
            id_receiver: self.id_receiver.clone(),
            registry: self.poll.registry().try_clone().unwrap(),
            message_sender: self.message_sender.clone(),
        }
    }
}

pub (crate) struct Handle{
    id_receiver: Arc<Mutex<mpsc::Receiver<Arc<IoWaker>>>>,
    registry : mio::Registry,
    message_sender: mpsc::Sender<Message>,
}

impl Handle{
    pub(crate) fn register(&self, source: &mut dyn mio::event::Source) -> Arc<IoWaker> {
        let waker = self.id_receiver.lock().unwrap().try_recv().expect("No id available");

        self.registry.register(source, mio::Token(waker.key()), mio::Interest::READABLE).unwrap();

        waker
    }

    fn deregister(&self, source: &mut dyn mio::event::Source, token: usize) {
        self.registry.deregister(source).unwrap();

        self.message_sender.send(Message::DelSource(token)).unwrap();
    }

    pub (crate) fn try_clone(&self) -> std::io::Result<Self> {
        let registry = self.registry.try_clone()?;

        Ok(Handle{
            id_receiver: self.id_receiver.clone(),
            message_sender: self.message_sender.clone(),
            registry,
        })
    }
}

enum CloneError{

}

enum Message{
    DelSource(usize),
}

pub(crate) struct IoWaker{
    key: usize,
    waker: Mutex<Option<Waker>>,
}

impl IoWaker{
    fn new(key: usize) -> IoWaker{
        IoWaker{
            key,
            waker: Mutex::from(None),
        }
    }

    pub fn key(&self) -> usize {
        self.key
    }

    pub fn take(&self) -> Option<Waker> {
        let mut guard = self.waker.lock().unwrap();
        
        let waker = match &mut *guard {
            Some(waker) => {
                Some(waker.clone())
            },
            None => None,
        };

        *guard = None;
        waker
    }
    
    pub fn set_waker(&self, waker: Waker){
        let mut guard = self.waker.lock().unwrap();
        *guard = Some(waker);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init() {
        let reactor = Reactor::new();

        assert_eq!(reactor.io_wakers.len(),DEFAULT_SLAB_SIZE);
        assert_eq!(reactor.io_wakers.len(),reactor.io_wakers.capacity());
    }

    #[test]
    fn empty_waker() {
        let waker = IoWaker::new(0);
        assert!(waker.take().is_none());
    }
}