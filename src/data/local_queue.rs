use crossbeam_queue::{ArrayQueue, PushError};

const DEFAULT_SIZE: usize = 10000;

#[derive(Debug)]
pub enum QueueError<T> {
    Push(T),
    Empty,
}

pub struct LocalQueue<T> {
    inner: ArrayQueue<T>,
}

impl<T> LocalQueue<T> {
    pub(crate) fn new() -> LocalQueue<T> {
        LocalQueue {
            inner: ArrayQueue::new(DEFAULT_SIZE),
        }
    }

    pub(crate) fn push(&self, val: T) -> Result<(), QueueError<T>> {
        if let Err(PushError(val)) = self.inner.push(val) {
            return Err(QueueError::Push(val));
        }

        Ok(())
    }

    pub(crate) fn pop(&self) -> Result<T, QueueError<T>> {
        match self.inner.pop() {
            Ok(val) => Ok(val),
            Err(_) => Err(QueueError::Empty),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_queue() {
        let queue = LocalQueue::<()>::new();

        assert!(queue.pop().is_err());
    }

    #[test]
    fn push_pop_empty() {
        let queue = LocalQueue::new();
        let val = 3;

        queue.push(val).expect("Error when pushing on the queue");

        assert_eq!(val, queue.pop().expect("Missing Value in queue"));
        assert!(queue.pop().is_err());
    }

    #[test]
    fn full_queue() {
        let queue = LocalQueue::new();

        for _ in 0..DEFAULT_SIZE {
            queue.push(3).expect("Error when pushing on the queue");
        }

        assert!(queue.push(3).is_err());
    }
}
