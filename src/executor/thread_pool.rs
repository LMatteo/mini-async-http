use std::future::Future;
use std::sync::Arc;

use futures::FutureExt;

use std::sync::mpsc;

use crate::data::AtomicTake;
use crate::data::{global_injector, Receiver, Sender};
use crate::executor::worker::Worker;
use crate::executor::ExecutorMessage;
use crate::executor::Task;
use crate::io::context;

use log::trace;

type Result = std::result::Result<(), PoolError>;

#[derive(Debug)]
pub(crate) enum PoolError {
    Spawn,
    Join,
    Block,
    Stop,
}

pub(crate) struct ThreadPoolBuilder {
    size: usize,
    start: Arc<dyn Fn(usize, PoolHandle) + Send + Sync + 'static>,
    stop: Arc<dyn Fn(usize) + Send + Sync + 'static>,
}

impl ThreadPoolBuilder {
    pub(crate) fn new() -> ThreadPoolBuilder {
        ThreadPoolBuilder {
            size: 1,
            start: Arc::from(|id, _| {
                trace!("Starting thread {}", id);
            }),
            stop: Arc::from(|id| {
                trace!("Stopping thread {}", id);
            }),
        }
    }

    pub(crate) fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub(crate) fn after_start<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, PoolHandle) + Send + Sync + 'static,
    {
        self.start = Arc::from(f);
        self
    }

    pub(crate) fn before_stop<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.stop = Arc::from(f);
        self
    }

    pub(crate) fn build(self) -> PoolHandle {
        let (sender, ready_queue) = global_injector();
        let (handle_sender, handle_receiver) = global_injector();

        let handle = PoolHandle {
            sender: sender.clone(),
            handles: handle_receiver,
        };

        for i in 0..self.size {
            let ready_queue = ready_queue.clone();
            let start = self.start.clone();
            let stop = self.stop.clone();
            let handle = handle.clone();
            let worker = Worker::new(sender.clone(), ready_queue);

            let handle = std::thread::spawn(move || {
                (start)(i, handle);
                context::set_worker(worker.clone());

                worker.run();

                (stop)(i);
            });
            handle_sender
                .send(handle)
                .expect("Issue when starting thread pool");
        }

        handle
    }
}
#[derive(Clone)]
pub(crate) struct PoolHandle {
    sender: Sender<ExecutorMessage>,
    handles: Receiver<std::thread::JoinHandle<()>>,
}

impl PoolHandle {
    pub(crate) fn spawn<F>(&self, future: F) -> Result
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: AtomicTake::from(future),
            task_sender: self.sender.clone(),
            notify_queue: None,
        });

        match self.sender.send(ExecutorMessage::Task(task)) {
            Ok(_) => Result::Ok(()),
            Err(_) => Result::Err(PoolError::Spawn),
        }
    }

    pub(crate) fn block_on<F>(&self, future: F) -> Result
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = future.boxed();

        let (sender, receiver) = mpsc::sync_channel(1);

        let task = Arc::new(Task {
            future: AtomicTake::from(future),
            task_sender: self.sender.clone(),
            notify_queue: Some(sender),
        });

        if self.sender.send(ExecutorMessage::Task(task)).is_err() {
            return Result::Err(PoolError::Spawn);
        }

        if receiver.recv().is_err() {
            return Result::Err(PoolError::Block);
        }

        Result::Ok(())
    }

    pub(crate) fn stop(&self) -> Result {
        if self.handles.is_empty() {
            return Err(PoolError::Stop);
        }

        for _ in 0..self.handles.len() {
            if self.sender.send(ExecutorMessage::Stop).is_err() {
                return Err(PoolError::Stop);
            }
        }

        while let Ok(handle) = self.handles.try_recv() {
            if handle.join().is_err() {
                return Err(PoolError::Join);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[test]
    fn block_on() {
        let pool = ThreadPoolBuilder::new().size(20).build();

        let (sender, receiver) = mpsc::channel();

        pool.block_on(async move {
            sender.send(3).unwrap();
        })
        .expect("Error when spawning block on task");

        assert_eq!(receiver.try_recv().unwrap(), 3);
    }

    #[test]
    fn spawn() {
        let size = 20;
        let pool = ThreadPoolBuilder::new().size(size).build();

        let (sender, receiver) = mpsc::channel();

        for _ in 0..size {
            let sender = sender.clone();
            pool.spawn(async move {
                sender.send(3).unwrap();
            });
        }

        for _ in 0..size {
            assert_eq!(receiver.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
        }
    }

    #[test]
    fn start_stop_func() {
        let size = 20;
        let (pstart, cstart) = mpsc::sync_channel(size);
        let (pstop, cstop) = mpsc::sync_channel(size);

        let start = move |_id, _| {
            pstart.send(()).unwrap();
        };

        let stop = move |_id| {
            pstop.send(()).unwrap();
        };

        let pool = ThreadPoolBuilder::new()
            .after_start(start)
            .before_stop(stop)
            .size(size)
            .build();

        for _ in 0..20 {
            cstart
                .recv_timeout(Duration::from_secs(1))
                .expect("Start thread func did not execute");
        }

        pool.stop();

        for _ in 0..20 {
            cstop
                .recv_timeout(Duration::from_secs(1))
                .expect("Stop thread func did not execute");
        }
    }

    #[test]
    fn spawn_error() {
        let size = 20;
        let pool = ThreadPoolBuilder::new().size(size).build();

        assert!(pool.stop().is_ok());

        let spawn = pool.spawn(async {});
        match spawn {
            Err(PoolError::Spawn) => {}
            _ => panic!("Should be spawn error"),
        }

        let spawn = pool.block_on(async {});
        match spawn {
            Err(PoolError::Spawn) => {}
            _ => panic!("Should be spawn error"),
        }
    }

    #[test]
    fn double_stop() {
        let size = 20;
        let pool = ThreadPoolBuilder::new().size(size).build();

        assert!(pool.stop().is_ok());
        let stop = pool.stop();

        match stop {
            Err(PoolError::Stop) => {}
            _ => panic!("Should be stop error"),
        };
    }
}
