use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::io;

fn main() {}

pub trait Read {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>;
}

#[allow(async_fn_in_trait)]
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
    file.read(&mut vec![])
}

// error[E0038]: the trait `AsyncRead` cannot be made into an object
//   --> src/main.rs:22:26
//    |
// 22 | async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
//    |                          ^^^^^^^^^^^^^ `AsyncRead` cannot be made into an object
//    |
// note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically; for more information visit <https://doc.rust-lang.
// org/reference/items/traits.html#object-safety>
//   --> src/main.rs:19:14
//    |
// 18 | pub trait AsyncRead {
//    |           --------- this trait cannot be made into an object...
// 19 |     async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
//    |              ^^^^ ...because method `read` is `async`
//    = help: consider moving `read` to another trait
//
// error[E0038]: the trait `AsyncRead` cannot be made into an object
//   --> src/main.rs:23:10
//    |
// 23 |     file.read(&mut vec![])
//    |          ^^^^ `AsyncRead` cannot be made into an object
//    |
// note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically; for more information visit <https://doc.rust-lang.
// org/reference/items/traits.html#object-safety>
//   --> src/main.rs:19:14
//    |
// 18 | pub trait AsyncRead {
//    |           --------- this trait cannot be made into an object...
// 19 |     async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
//    |              ^^^^ ...because method `read` is `async`
//    = help: consider moving `read` to another trait
//
// error[E0038]: the trait `AsyncRead` cannot be made into an object
//   --> src/main.rs:23:5
//    |
// 23 |     file.read(&mut vec![])
//    |     ^^^^^^^^^^^^^^^^^^^^^^ `AsyncRead` cannot be made into an object
//    |
// note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically; for more information visit <https://doc.rust-lang.
// org/reference/items/traits.html#object-safety>
//   --> src/main.rs:19:14
//    |
// 18 | pub trait AsyncRead {
//    |           --------- this trait cannot be made into an object...
// 19 |     async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
//    |              ^^^^ ...because method `read` is `async`
//    = help: consider moving `read` to another trait
//
// For more information about this error, try `rustc --explain E0038`.
// error: could not compile `dyn-afit` (bin "dyn-afit") due to 3 previous errors
