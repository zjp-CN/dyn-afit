use std::io; // pretend a Result is defined somewhere without std

#[pollster::main]
async fn main() {
    let mut file = _impl::File::new();

    // static dispatch: no overhead
    _ = dbg!(file.read(&mut []).await);

    // dynamic dispatch: with boxing overhead
    let dyn_async_read = DynAsyncRead::from_mut(&mut file);
    _ = dbg!(call(dyn_async_read).await);
}

#[dynosaur::dynosaur(DynAsyncRead)]
#[allow(async_fn_in_trait)]
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

pub async fn call(file: &mut DynAsyncRead<'_>) -> io::Result<usize> {
    file.read(&mut []).await
}

mod _impl {
    use crate::{io, AsyncRead};

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
        async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.inner_read(buf).await
        }
    }
}
