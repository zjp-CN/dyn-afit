use std::io;

fn main() {}

#[async_trait::async_trait] // trait object safe
pub trait AsyncRead {
    // but at the cost of returning Box<...>
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

pub async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
    file.read(&mut []).await
}
