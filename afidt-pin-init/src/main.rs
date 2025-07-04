#![feature(ptr_metadata)]

use pin_init::{DynInPlaceInit, dyn_init};
use std::{
    pin::{Pin, pin},
    ptr,
};

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

struct B;
impl Async for B {
    #[dyn_init]
    async fn foo(&self) {
        let large_size = [0; 100];
        async { println!("B") }.await;
        // Use the large buffer across await point to make the future large.
        _ = large_size;
    }
}

fn main() {
    pollster::block_on(run())
}

async fn run() {
    let a = A {};

    // static dispatch
    a.foo().await;

    let ref_a: &dyn Async = &a;
    dynamic_dispatch(ref_a).await;

    dynamic_dispatch(&B).await;
}

const FUT_STACK_SIZE: usize = 64;

async fn dynamic_dispatch(ref_a: &dyn Async) {
    let dyn_foo = ref_a.dyn_foo();
    let layout = dyn_foo.layout();
    let fut_size = layout.size();

    if fut_size > FUT_STACK_SIZE {
        println!("Heap allocation as the future is too large.");
        Box::into_pin(Box::dyn_init(dyn_foo)).await;
    } else {
        let mut stack = pin!([0u8; FUT_STACK_SIZE]);

        // alignment adjustment
        let start = stack.as_mut_ptr();
        let end = start.wrapping_add(FUT_STACK_SIZE);
        let slot = start.wrapping_add(start.align_offset(layout.align()));
        let slot_end = slot.wrapping_add(fut_size);

        // dbg!(start, slot, slot_end, end, fut_size, layout.align());
        if !(start <= slot && slot_end <= end) {
            println!("Heap allocation due to stack is not enough.");
            Box::into_pin(Box::dyn_init(dyn_foo)).await;
            return;
        }

        unsafe {
            // get the pinned Future from stack
            let meta = dyn_foo.init(slot as *mut ()).unwrap();
            let ptr_dyn_fut: *mut dyn Future<Output = ()> = ptr::from_raw_parts_mut(slot, meta);
            let pin_dyn_fut = Pin::new_unchecked(&mut *ptr_dyn_fut);

            // poll the future
            println!("Stack allocation. 🦀");
            pin_dyn_fut.await;

            // drop the future
            ptr::drop_in_place(ptr_dyn_fut);
        }
    }
}

// [OUTPUT]
// foo!
// Stack allocation. 🦀
// foo!
// Heap allocation as the future is too large.
// B
