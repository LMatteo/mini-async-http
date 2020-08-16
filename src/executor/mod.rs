use {
    futures::{
        future::{BoxFuture, FutureExt},
        task::{waker_ref, ArcWake},
    },
    std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll},
    },
};

use log::error;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;

use std::sync::mpsc;

use crate::data::AtomicTake;

pub mod thread_pool;
pub mod worker;

pub(crate) enum ExecutorMessage {
    Task(Arc<Task>),
    Stop,
}

/// Task executor that receives tasks off of a channel and runs them.
pub struct Executor {
    ready_queue: Receiver<ExecutorMessage>,
}

/// `Spawner` spawns new futures onto the task channel.
#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<ExecutorMessage>,
}

/// A future that can reschedule itself to be polled by an `Executor`.
pub struct Task {
    future: AtomicTake<BoxFuture<'static, ()>>,

    /// Handle to place the task itself back onto the task queue.
    task_sender: Sender<ExecutorMessage>,

    notify_queue: Option<mpsc::SyncSender<()>>,
}

impl Task {
    pub(crate) fn notify(&self) {
        if let Some(ref queue) = self.notify_queue {
            if queue.send(()).is_err() {
                error!("Issue when notifying block on request");
            }
        }
    }
}

pub fn new_executor_and_spawner() -> (Executor, Spawner) {
    // Maximum number of tasks to allow queueing in the channel at once.
    // This is just to make `sync_channel` happy, and wouldn't be present in
    // a real executor.
    const MAX_QUEUED_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = crossbeam_channel::bounded(MAX_QUEUED_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}

impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: AtomicTake::from(future),
            task_sender: self.task_sender.clone(),
            notify_queue: None,
        });
        self.task_sender
            .send(ExecutorMessage::Task(task))
            .expect("too many tasks queued");
    }

    pub fn stop(&self) {
        self.task_sender
            .send(ExecutorMessage::Stop)
            .expect("too many tasks queued")
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(ExecutorMessage::Task(cloned))
            .expect("too many tasks queued");
    }
}

impl Executor {
    pub fn run(&self) {
        while let Ok(ExecutorMessage::Task(task)) = self.ready_queue.recv() {
            // Take the future, and if it has not yet completed (is still Some),
            // poll it in an attempt to complete it.
            let future_slot = task.future.take();
            if let Some(mut future) = future_slot {
                // Create a `LocalWaker` from the task itself
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&*waker);
                // `BoxFuture<T>` is a type alias for
                // `Pin<Box<dyn Future<Output = T> + Send + 'static>>`.
                // We can get a `Pin<&mut dyn Future + Send + 'static>`
                // from it by calling the `Pin::as_mut` method.
                if let Poll::Pending = future.as_mut().poll(context) {
                    // We're not done processing the future, so put it
                    // back in its task to be run again in the future.
                    task.future.store(future);
                }
            }
        }
    }
}
