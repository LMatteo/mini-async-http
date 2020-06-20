use log::trace;
use mio::event::Source;
use mio::unix::SourceFd;
use mio::{Interest, Registry, Token};
use std::io;
use std::ops::Deref;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixDatagram;
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};
use std::sync::Arc;

pub fn channel<T>() -> (EventedSender<T>, EventedReceiver<T>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let (dsender, dreceiver) = UnixDatagram::pair().unwrap();

    let sender = EventedSender::new(sender, Arc::from(dsender));
    let receiver = EventedReceiver::new(receiver, dreceiver);

    (sender, receiver)
}

pub struct EventedReceiver<T> {
    inner: Receiver<T>,
    receiver: UnixDatagram,
}

impl<T> EventedReceiver<T> {
    fn new(inner: Receiver<T>, receiver: UnixDatagram) -> EventedReceiver<T> {
        receiver.set_nonblocking(true).unwrap();
        EventedReceiver { inner, receiver }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut buf: [u8; 10] = [0; 10];
        match self.receiver.recv(&mut buf) {
            Ok(_) => {}
            Err(e) => trace!("Error when reading on evented channel datagram {}", e),
        }

        self.inner.try_recv()
    }
}

impl<T> Deref for EventedReceiver<T> {
    type Target = Receiver<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Source for EventedReceiver<T> {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        let fd = &self.receiver.as_raw_fd();
        SourceFd(fd).register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        let fd = &self.receiver.as_raw_fd();
        SourceFd(fd).reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        let fd = &self.receiver.as_raw_fd();
        SourceFd(fd).deregister(registry)
    }
}

pub struct EventedSender<T> {
    inner: Sender<T>,
    sender: Arc<UnixDatagram>,
    buf: [u8; 1],
}

impl<T> EventedSender<T> {
    fn new(inner: Sender<T>, sender: Arc<UnixDatagram>) -> EventedSender<T> {
        sender.set_nonblocking(true).unwrap();
        EventedSender {
            inner,
            sender,
            buf: [1; 1],
        }
    }

    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        let result = self.inner.send(t)?;
        match self.sender.send(&self.buf[0..1]) {
            Ok(_) => {}
            Err(e) => trace!("Error when writing on evented channel datagram {}", e),
        };

        Ok(result)
    }
}

impl<T> Clone for EventedSender<T> {
    fn clone(&self) -> Self {
        EventedSender::new(self.inner.clone(), self.sender.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn send() {
        let (sender, receiver) = channel();

        sender.send('r').unwrap();

        let recv = receiver.try_recv().unwrap();

        assert_eq!('r', recv);
    }
}
