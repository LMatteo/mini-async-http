use crossbeam_channel::SendError;

pub(crate) fn global_injector<T>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = crossbeam_channel::unbounded();

    (Sender { inner: sender }, Receiver { inner: receiver })
}

#[derive(Debug)]
pub enum InjectorError<T> {
    Send(T),
    Recv,
}

#[derive(Debug)]
pub struct Sender<T> {
    inner: crossbeam_channel::Sender<T>,
}

impl<T> Sender<T> {
    pub(crate) fn send(&self, val: T) -> Result<(), InjectorError<T>> {
        match self.inner.send(val) {
            Ok(_) => Ok(()),
            Err(send_error) => Err(InjectorError::Send(send_error.into_inner())),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Receiver<T> {
    inner: crossbeam_channel::Receiver<T>,
}

impl<T> Receiver<T> {
    pub(crate) fn recv(&self) -> Result<T, InjectorError<T>> {
        match self.inner.recv() {
            Ok(val) => Ok(val),
            Err(_) => Err(InjectorError::Recv),
        }
    }

    pub(crate) fn try_recv(&self) -> Result<T, InjectorError<T>> {
        match self.inner.try_recv() {
            Ok(val) => Ok(val),
            Err(_) => Err(InjectorError::Recv),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn empty() {
        let (sender, receiver) = global_injector::<()>();

        assert!(receiver.try_recv().is_err());
        assert_eq!(0, receiver.len());
        assert_eq!(0, sender.len());
    }

    #[test]
    fn disconnected_recv() {
        let (_, receiver) = global_injector::<()>();

        assert!(receiver.recv().is_err());
        assert_eq!(0, receiver.len());
        assert!(receiver.is_empty());
    }

    #[test]
    fn disconnected_send() {
        let (sender, _) = global_injector::<()>();

        assert!(sender.send(()).is_err());
        assert_eq!(0, sender.len());
        assert!(sender.is_empty());
    }

    #[test]
    fn simple_send_recv() {
        let (sender, receiver) = global_injector();

        let val = 3;

        sender.send(val).expect("Error when sending");

        assert_eq!(val, receiver.try_recv().expect("Error when receiving"));
        assert!(sender.is_empty());
        assert!(receiver.is_empty());
    }

    #[test]
    fn multiple_send_recv() {
        let (sender, receiver) = global_injector();

        let val = 3;
        const NB_SEND: usize = 10000;

        for _ in 0..NB_SEND {
            sender.send(val).expect("Error when sending");
        }

        for _ in 0..NB_SEND {
            assert_eq!(val, receiver.try_recv().expect("Error when receiving"));
        }
    }

    #[test]
    fn parallel_send_recv() {
        const NB_SEND: usize = 1000;

        let (sender, receiver) = global_injector();
        let barrier = Arc::new(Barrier::new(NB_SEND));

        let val = 3;

        for _ in 0..NB_SEND {
            let sender = sender.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                sender.send(val).expect("Error when sending");
            });
        }

        for _ in 0..NB_SEND {
            assert_eq!(val, receiver.recv().expect("Error when receiving"));
        }
    }

    #[test]
    fn send_parralel_recv() {
        let (sender, receiver) = global_injector();

        let val = 3;
        const NB_SEND: usize = 1000;

        let barrier = Arc::new(Barrier::new(NB_SEND));

        for _ in 0..NB_SEND {
            sender.send(val).expect("Error when sending");
        }

        for _ in 0..NB_SEND {
            let receiver = receiver.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                assert_eq!(val, receiver.recv().expect("Error when receiving"));
            });
        }
    }

    #[test]
    fn parralel_send_parralel_recv() {
        let (sender, receiver) = global_injector();

        let mut handles = Vec::new();

        let val = 3;
        const NB_SEND: usize = 1000;

        let barrier = Arc::new(Barrier::new(NB_SEND * 2));

        for _ in 0..NB_SEND {
            let barrier = barrier.clone();
            let sender = sender.clone();
            let handle = std::thread::spawn(move || {
                barrier.wait();
                sender.send(val).expect("Error when sending");
            });

            handles.push(handle);
        }

        for _ in 0..NB_SEND {
            let receiver = receiver.clone();
            let barrier = barrier.clone();
            let handle = std::thread::spawn(move || {
                barrier.wait();
                assert_eq!(val, receiver.recv().expect("Error when receiving"));
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Join error");
        }
    }
}
