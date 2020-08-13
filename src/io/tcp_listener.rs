use mio::net;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use crate::io::context;
use crate::io::reactor::Handle;
use crate::io::reactor::IoWaker;

pub(crate) struct TcpListener {
    inner: net::TcpListener,
    waker: Arc<IoWaker>,
}

#[derive(Debug)]
pub(crate) enum AcceptError {
    Err,
}

impl TcpListener {
    pub(crate) fn bind(addr: std::net::SocketAddr) -> TcpListener {
        let mut inner = net::TcpListener::bind(addr).unwrap();

        let handle = context::handle().expect("Context not initialized");
        let waker = handle.register(&mut inner);

        TcpListener { inner, waker }
    }

    pub(crate) async fn accept(
        &self,
    ) -> Result<(net::TcpStream, std::net::SocketAddr), AcceptError> {
        AcceptFuture {
            waker: self.waker.clone(),
            listener: self,
        }
        .await
    }
}

pub(crate) struct AcceptFuture<'a> {
    waker: Arc<IoWaker>,
    listener: &'a TcpListener,
}

impl Future for AcceptFuture<'_> {
    type Output = Result<(net::TcpStream, std::net::SocketAddr), AcceptError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.waker.set_waker(cx.waker().clone());

        match self.listener.inner.accept() {
            Ok(result) => Poll::Ready(Ok(result)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(AcceptError::Err)),
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let handle = match context::handle() {
            Some(handle) => handle,
            None => return,
        };

        handle.deregister(&mut self.inner, self.waker.clone());
    }
}
