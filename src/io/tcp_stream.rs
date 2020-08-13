use futures::io::Error;
use futures::AsyncRead;
use mio::net;

use std::future::Future;
use std::io::Read;
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use crate::io::context;
use crate::io::reactor::Handle;
use crate::io::reactor::IoWaker;

pub struct TcpStream {
    inner: net::TcpStream,
    waker: Arc<IoWaker>,
}

impl TcpStream {
    pub(crate) fn from_stream(inner: net::TcpStream) -> TcpStream {
        let mut inner = inner;

        let handle = context::handle().expect("Context not initialized");
        let waker = handle.register(&mut inner);
        TcpStream { inner, waker }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        self.waker.set_waker(cx.waker().clone());

        match self.get_mut().inner.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let handle = match context::handle() {
            Some(handle) => handle,
            None => return,
        };

        handle.deregister(&mut self.inner, self.waker.clone());
    }
}
