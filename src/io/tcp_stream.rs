use mio::net;
use futures::AsyncRead;
use futures::io::Error;

use std::sync::Arc;
use std::future::Future;
use std::task::Poll;
use std::task::Context; 
use std::pin::Pin;
use std::io::Read;

use crate::io::reactor::IoWaker;
use crate::io::reactor::Handle;

pub struct TcpStream {
    inner: net::TcpStream,
    waker: Arc<IoWaker>,
}

impl TcpStream{
    pub(crate) fn from_stream(inner: net::TcpStream, handle: &Handle) -> TcpStream {
        let mut inner = inner;
        let waker = handle.register(&mut inner);
        TcpStream{
            inner,
            waker,
        }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8]
    ) -> Poll<Result<usize, Error>> {
        self.waker.set_waker(cx.waker().clone());



        match self.get_mut().inner.read(buf) {
            Ok(n) => {
                Poll::Ready(Ok(n))                
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Poll::Pending
            },
            Err(e) => Poll::Ready(Err(e))
        }
    }
}

