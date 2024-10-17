use stackfuture::StackFuture;
use std::io; // pretend a Result is defined somewhere without std

#[pollster::main]
async fn main() {
    let mut file = _impl::File::new();
    _ = dbg!(file.read(&mut []).await);
    _ = dbg!(call(&mut file).await);
}

pub trait AsyncRead {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> StackFuture<'a, io::Result<usize>, 128>;
}

pub async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
    file.read(&mut []).await
}

mod _impl {
    use crate::{io, AsyncRead, StackFuture};

    pub struct File {}

    impl File {
        pub fn new() -> Self {
            Self {}
        }

        pub async fn inner_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
    }

    impl AsyncRead for File {
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> StackFuture<'a, io::Result<usize>, 128> {
            StackFuture::from_or_box(self.inner_read(buf))
        }
    }
}
