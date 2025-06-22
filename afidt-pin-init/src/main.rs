#![feature(ptr_metadata)]

use pin_init::{DynInPlaceInit, dyn_init};
use std::{pin::Pin, ptr};

trait Async {
    #[dyn_init]
    async fn foo(&self);
}

struct A {}

impl Async for A {
    #[dyn_init]
    async fn foo(&self) {
        println!("foo!")
    }
}

fn main() {
    pollster::block_on(async {
        let a = A {};

        // static dispatch
        a.foo().await;

        let ref_a: &dyn Async = &a;
        dynamic_dispatch(ref_a).await;
    })
}

async fn dynamic_dispatch(ref_a: &dyn Async) {
    let dyn_foo = ref_a.dyn_foo();
    let layout = dbg!(dyn_foo.layout());

    if layout.size() > 16 {
        // heap allocation if the future is too large
        Box::into_pin(Box::dyn_init(dyn_foo)).await;
    } else {
        let mut stack = [0; 16];

        let slot = &mut stack as *mut _ as *mut ();

        let pin_dyn_fut = unsafe {
            let meta = dyn_foo.init(slot).unwrap();
            dbg!(meta);
            let ptr_dyn_fut = ptr::from_raw_parts_mut::<dyn Future<Output = ()>>(&mut stack, meta);
            Pin::new_unchecked(&mut *ptr_dyn_fut)
        };

        // no allocation if it's small enough
        pin_dyn_fut.await;
    }
}

// [OUTPUT]
// foo!
// [src/main.rs:38:18] dyn_foo.layout() = Layout {
//     size: 16,
//     align: 8 (1 << 3),
// }
// [src/main.rs:44:13] meta = DynMetadata(
//     0x0000000000046358,
// )
// foo!
