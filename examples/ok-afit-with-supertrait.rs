use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{future::poll_fn, io};

#[pollster::main]
async fn main() {
    _ = dbg!(call(&mut _impl::File::new()).await);
}

pub trait Read {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>;
}

#[allow(async_fn_in_trait)]
pub trait AsyncRead: Read + Unpin {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut pinned = Pin::new(self);
        poll_fn(|cx| pinned.as_mut().poll_read(cx, buf)).await
    }
}

impl<T: Read + Unpin + ?Sized> AsyncRead for T {}

pub async fn call(file: &mut (dyn Read + Unpin)) -> io::Result<usize> {
    file.read(&mut []).await
}

mod _impl {
    use crate::{io, Context, Pin, Poll, Read};

    pub struct File {}

    impl File {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Read for File {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(buf.len()))
        }
    }
}
