use futures::task::waker_ref;
use futures::FutureExt;
use std::future::Future;
use std::task::Context;
use std::task::Poll;

use std::sync::Arc;

use log::error;

use crate::data::AtomicTake;
use crate::data::{LocalQueue, QueueError, Receiver, Sender};
use crate::executor::{ExecutorMessage, Task};

#[derive(Clone)]
pub(crate) struct Worker {
    local: Arc<LocalQueue<Arc<Task>>>,
    global_sender: Sender<ExecutorMessage>,
    global_receiver: Receiver<ExecutorMessage>,
}

impl Worker {
    pub(crate) fn new(
        sender: Sender<ExecutorMessage>,
        receiver: Receiver<ExecutorMessage>,
    ) -> Worker {
        Worker {
            local: Arc::from(LocalQueue::new()),
            global_sender: sender,
            global_receiver: receiver,
        }
    }

    pub(crate) fn enqueue<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        let task = Arc::new(Task {
            future: AtomicTake::from(future.boxed()),
            task_sender: self.global_sender.clone(),
            notify_queue: None,
        });

        if let Err(QueueError::Push(task)) = self.local.push(task) {
            if self
                .global_sender
                .send(ExecutorMessage::Task(task))
                .is_err()
            {
                error!("Could not push task onto the global queue")
            }
        }
    }

    pub(crate) fn run(&self) {
        while let Some(task) = self.pop_task() {
            let future_slot = task.future.take();
            if let Some(mut future) = future_slot {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&*waker);

                if let Poll::Pending = future.as_mut().poll(context) {
                    task.future.store(future);
                } else {
                    task.notify();
                }
            }
        }
    }

    fn pop_task(&self) -> Option<Arc<Task>> {
        match self.local.pop() {
            Ok(task) => Some(task),
            Err(_) => {
                if let Ok(ExecutorMessage::Task(task)) = self.global_receiver.recv() {
                    Some(task)
                } else {
                    None
                }
            }
        }
    }
}
