use crossbeam_utils::atomic::AtomicCell;

pub struct AtomicTake<T>{
    inner: AtomicCell<Option<T>>,
}

impl<T> AtomicTake<T> {
    pub(crate) fn new() -> AtomicTake<T>{
        AtomicTake{
            inner: AtomicCell::new(None)
        }
    }

    pub(crate) fn from(value: T) -> AtomicTake<T> {
        AtomicTake{
            inner: AtomicCell::new(Option::from(value)),
        }
    }

    pub(crate) fn take(&self) -> Option<T> {
        self.inner.take()
    }

    pub(crate) fn store(&self, value: T) {
        self.inner.store(Option::from(value));
    }
}

#[cfg(test)]
mod test{
    use super::*;
    use std::sync::mpsc;
    use std::sync::{Arc,Barrier};

    #[test]
    fn take() {
        let val = 3;
        let take = AtomicTake::from(val);

        assert_eq!(val,take.take().expect("Missing value"));
    }

    #[test]
    fn double_take() {
        let val = 3;
        let take = AtomicTake::from(val);

        take.take();

        assert!(take.take().is_none());
    }

    #[test]
    fn store() {
        let val = 3;
        let take = AtomicTake::from(val);

        let store = 5;
        take.store(store);

        assert_eq!(store,take.take().expect("Missing value"));
    }

    #[test]
    fn parallel_take() {
        let (sender,receiver) = mpsc::channel();

        let nb_thread = 20;

        let val = 3;
        let take = Arc::from(AtomicTake::from(val));
        let barrier = Arc::new(Barrier::new(nb_thread));

        for _ in 0..nb_thread {
            let take = take.clone();
            let sender = sender.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                match take.take() {
                    Some(val) => {
                        sender.send(val).expect("Error when sending value");
                    }
                    None => {},
                }
            });
        }

        assert_eq!(val,receiver.recv().expect("Missing value"));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn empty() {
        let take = AtomicTake::new();
        assert!(take.take().is_none());

        let val = 3;
        take.store(val);

        assert_eq!(val,take.take().expect("Missing value"));
    }
}



