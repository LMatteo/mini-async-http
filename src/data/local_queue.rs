use std::cell::UnsafeCell;

const DEFAULT_SIZE: usize = u16::MAX as usize;

#[derive(Debug)]
pub(crate) enum QueueError<T> {
    Push(T),
    Empty,
}

pub(crate) struct LocalQueue<T> {
    inner: UnsafeCell<Vec<T>>,
}

impl<T> LocalQueue<T> {
    pub(crate) fn new() -> LocalQueue<T> {
        LocalQueue {
            inner: UnsafeCell::from(Vec::with_capacity(DEFAULT_SIZE)),
        }
    }

    pub(crate) fn push(&self, val: T) -> Result<(), QueueError<T>> {
        let inner: &mut Vec<T> = unsafe { &mut *self.inner.get() };
        if inner.len() >= DEFAULT_SIZE {
            return Err(QueueError::Push(val));
        }

        inner.push(val);
        Ok(())
    }

    pub(crate) fn pop(&self) -> Result<T, QueueError<T>> {
        let inner: &mut Vec<T> = unsafe { &mut *self.inner.get() };

        if let Some(val) = inner.pop() {
            return Ok(val);
        }

        Err(QueueError::Empty)
    }
}

unsafe impl<T> Sync for LocalQueue<T> {}

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
