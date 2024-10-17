use std::io; // pretend a Result is defined somewhere without std

#[pollster::main]
async fn main() {
    let mut file = _impl::File::new();

    // dynamic dispatch: with boxing overhead once (in calling read on DynAsyncRead)
    let dyn_async_read = DynAsyncRead::from_mut(&mut file);
    _ = dbg!(call(dyn_async_read).await);

    // dynamic dispatch: with boxing overhead twice in
    // * creating a Boxed DynAsyncRead value
    // * and calling read on DynAsyncRead
    let mut box_dyn_async_read = DynAsyncRead::boxed(file);
    _ = dbg!(call(&mut box_dyn_async_read).await);
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

// #[allow(async_fn_in_trait)]
// pub trait AsyncRead {
//     async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
// }
// mod _dynosaur_macro_dynasyncread {
//     use super::*;
//     #[allow(async_fn_in_trait)]
//     pub trait ErasedAsyncRead {
//         fn read<'life0, 'life1, 'dynosaur>(
//             &'life0 mut self,
//             buf: &'life1 mut [u8],
//         ) -> ::core::pin::Pin<
//             Box<dyn ::core::future::Future<Output = io::Result<usize>> + 'dynosaur>,
//         >
//         where
//             'life0: 'dynosaur,
//             'life1: 'dynosaur,
//             Self: 'dynosaur;
//     }
//     impl<DYNOSAUR: AsyncRead> ErasedAsyncRead for DYNOSAUR {
//         fn read<'life0, 'life1, 'dynosaur>(
//             &'life0 mut self,
//             buf: &'life1 mut [u8],
//         ) -> ::core::pin::Pin<
//             Box<dyn ::core::future::Future<Output = io::Result<usize>> + 'dynosaur>,
//         >
//         where
//             'life0: 'dynosaur,
//             'life1: 'dynosaur,
//             Self: 'dynosaur,
//         {
//             Box::pin(<Self as AsyncRead>::read(self, buf))
//         }
//     }
//     #[repr(transparent)]
//     pub struct DynAsyncRead<'dynosaur_struct> {
//         ptr: dyn ErasedAsyncRead + 'dynosaur_struct,
//     }
//     impl<'dynosaur_struct> AsyncRead for DynAsyncRead<'dynosaur_struct> {
//         fn read(
//             &mut self,
//             buf: &mut [u8],
//         ) -> impl ::core::future::Future<Output = io::Result<usize>> {
//             let fut: ::core::pin::Pin<
//                 Box<dyn ::core::future::Future<Output = io::Result<usize>> + '_>,
//             > = self.ptr.read(buf);
//             let fut: ::core::pin::Pin<
//                 Box<dyn ::core::future::Future<Output = io::Result<usize>> + 'static>,
//             > = unsafe { ::core::mem::transmute(fut) };
//             fut
//         }
//     }
//     impl<'dynosaur_struct> DynAsyncRead<'dynosaur_struct> {
//         pub fn new(
//             value: Box<impl AsyncRead + 'dynosaur_struct>,
//         ) -> Box<DynAsyncRead<'dynosaur_struct>> {
//             let value: Box<dyn ErasedAsyncRead + 'dynosaur_struct> = value;
//             unsafe { ::core::mem::transmute(value) }
//         }
//         pub fn boxed(
//             value: impl AsyncRead + 'dynosaur_struct,
//         ) -> Box<DynAsyncRead<'dynosaur_struct>> {
//             Self::new(Box::new(value))
//         }
//         pub fn from_ref(
//             value: &(impl AsyncRead + 'dynosaur_struct),
//         ) -> &DynAsyncRead<'dynosaur_struct> {
//             let value: &(dyn ErasedAsyncRead + 'dynosaur_struct) = &*value;
//             unsafe { ::core::mem::transmute(value) }
//         }
//         pub fn from_mut(
//             value: &mut (impl AsyncRead + 'dynosaur_struct),
//         ) -> &mut DynAsyncRead<'dynosaur_struct> {
//             let value: &mut (dyn ErasedAsyncRead + 'dynosaur_struct) = &mut *value;
//             unsafe { ::core::mem::transmute(value) }
//         }
//     }
// }
// use _dynosaur_macro_dynasyncread::DynAsyncRead;
