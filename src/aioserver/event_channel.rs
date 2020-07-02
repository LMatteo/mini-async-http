use log::trace;
use mio::event::Source;
use mio::unix::SourceFd;
use mio::{Interest, Registry, Token, Waker, Poll};
use std::io;
use std::ops::Deref;
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};
use std::sync::Arc;

/// Create a pair of evented channel that can be integrated to the mio event loop
/// 
/// The behaviour is similar to the std channel 
pub (crate) fn channel<T>(waker: Arc<Waker>) -> (EventedSender<T>, Receiver<T>) {
    let (sender, receiver) = std::sync::mpsc::channel();

    let sender = EventedSender::new(sender, waker   );

    (sender, receiver)
}

pub (crate) struct EventedSender<T> {
    inner: Sender<T>,
    waker: Arc<Waker>,
}

impl<T> EventedSender<T> {
    fn new(inner: Sender<T>, waker: Arc<Waker>) -> EventedSender<T> {
        EventedSender {
            inner,
            waker,
        }
    }

    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        self.inner.send(t)?;
        self.waker.wake().unwrap();

        Ok(())
    }
}

impl<T> Clone for EventedSender<T> {
    fn clone(&self) -> Self {
        EventedSender::new(self.inner.clone(), self.waker.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn send() {
        let mut poll = Poll::new().unwrap();
        let waker = Arc::new(Waker::new(poll.registry(), Token(0)).unwrap());

        let (sender, receiver) = channel(waker);

        sender.send('r').unwrap();

        let recv = receiver.try_recv().unwrap();

        assert_eq!('r', recv);
    }
}
