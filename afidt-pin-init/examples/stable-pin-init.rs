//! This file demostrates another pin-init version that can be used on
//! stable Rust by avoiding Pointer Metadata APIs and reinventing trait objects.
//!
//! Original athuor is [@loichyan](https://github.com/loichyan).
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::disallowed_names)]
#![allow(clippy::missing_safety_doc)]

use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomPinned;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};

// =========== 获取函数返回类型 ===========
// 参考 <https://docs.rs/axum/latest/axum/handler/trait.Handler.html>
mod function {
    use std::any::Any;
    use std::convert::Infallible;

    use super::*;

    pub trait Function<Input> {
        type Output;
    }

    macro_rules! impl_function {
    ($($i:ident),* -> $o:ident) => {
        impl<Fn, $($i,)* $o> Function<($($i,)*)> for Fn
        where
            Fn: FnOnce($($i,)*) -> $o,
        {
            type Output = $o;
        }
    };
}
    impl_function!(A                -> R);
    impl_function!(A, B             -> R);
    impl_function!(A, B, C          -> R);
    impl_function!(A, B, C, D       -> R);
    impl_function!(A, B, C, D, E    -> R);
    impl_function!(A, B, C, D, E, F -> R);

    pub const fn return_type_dangling_ptr<I, F: Function<I>>(_: &F) -> *mut F::Output {
        std::ptr::dangling_mut()
    }

    pub const fn return_type_layout<I, F: Function<I>>(_: &F) -> Layout {
        Layout::new::<F::Output>()
    }

    pub const fn return_type_cast_ptr<I, F: Function<I>>(
        _: &F,
        ptr: VoidPtr,
    ) -> NonNull<F::Output> {
        ptr.cast()
    }

    pub fn test_return_type_layout() {
        fn f1(_: usize, _: usize) -> usize {
            todo!()
        }
        fn f2(_: &str) -> &str {
            todo!()
        }
        fn f3(_: String, _: Vec<u8>, _: &dyn Any) -> Box<dyn Any> {
            todo!()
        }
        fn f4(_: usize, _: usize) -> Infallible {
            todo!()
        }

        assert_eq!(Layout::new::<usize>(), return_type_layout(&f1));
        assert_eq!(Layout::new::<&str>(), return_type_layout(&f2));
        assert_eq!(Layout::new::<Box<dyn Any>>(), return_type_layout(&f3));
        assert_eq!(Layout::new::<Infallible>(), return_type_layout(&f4));
        println!("test_return_type_layout pass");
    }
}
pub use function::*;

// =========== 模拟 dyn object ===========
mod dyn_object {

    use super::*;

    pub unsafe trait DynCompatible {
        type Object;

        unsafe fn construct(data: VoidPtr) -> Self::Object
        where
            Self: Sized;
        fn data(this: &Self::Object) -> VoidPtr;
        fn layout(this: &Self::Object) -> Layout;
    }

    pub type VoidPtr = NonNull<Void>;
    pub enum Void {}

    pub struct DynFuture<Fut: ?Sized + Future> {
        data: VoidPtr,
        vtable: *const FutureVtable<Fut>,
        #[allow(dead_code)]
        unpin: PhantomPinned,
    }
    struct FutureVtable<Fut: ?Sized + Future> {
        layout: fn() -> Layout,
        poll_fn: unsafe fn(VoidPtr, cx: &mut Context) -> Poll<Fut::Output>,
        drop_fn: unsafe fn(VoidPtr),
    }
    unsafe impl<Fut> DynCompatible for Fut
    where
        Fut: ?Sized + Future,
    {
        type Object = DynFuture<dyn Future<Output = Fut::Output>>;

        unsafe fn construct(data: VoidPtr) -> Self::Object
        where
            Self: Sized,
        {
            // 参考 <https://github.com/dtolnay/anyhow/blob/69295727cefb015a184f9b780fcc51ef905a798c/src/error.rs#L155>
            unsafe fn poll_fn<Fut: Future>(data: VoidPtr, cx: &mut Context) -> Poll<Fut::Output> {
                Pin::new_unchecked(data.cast::<Fut>().as_mut()).poll(cx)
            }
            unsafe fn drop_fn<Fut: Future>(data: VoidPtr) {
                data.cast::<Fut>().drop_in_place();
            }
            fn vtable<Fut: Future>() -> *const FutureVtable<dyn Future<Output = Fut::Output>> {
                &FutureVtable {
                    layout: Layout::new::<Fut>,
                    poll_fn: poll_fn::<Fut>,
                    drop_fn: drop_fn::<Fut>,
                }
            }
            DynFuture {
                data: data.cast(),
                vtable: vtable::<Self>(),
                unpin: PhantomPinned,
            }
        }
        fn data(this: &Self::Object) -> VoidPtr {
            this.data
        }
        fn layout(this: &Self::Object) -> Layout {
            unsafe { ((*this.vtable).layout)() }
        }
    }

    impl<Fut: ?Sized + Future> Future for DynFuture<Fut> {
        type Output = Fut::Output;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unsafe {
                let this = self.get_unchecked_mut();
                ((*this.vtable).poll_fn)(this.data, cx)
            }
        }
    }
    impl<Fut: ?Sized + Future> Drop for DynFuture<Fut> {
        fn drop(&mut self) {
            // 只调用析构函数，data 本身占用的内存不回收
            unsafe { ((*self.vtable).drop_fn)(self.data) }
        }
    }
}
pub use dyn_object::*;

// =========== 封装 DynObject ===========
mod dyn_init {
    use std::ops::{Deref, DerefMut};

    pub use super::*;

    pub struct DynInit<Dyn: ?Sized + DynCompatible, Args> {
        args: Args,
        layout: fn() -> Layout,
        init: fn(VoidPtr, Args) -> Dyn::Object,
    }

    impl<Dyn: ?Sized + DynCompatible, Args> DynInit<Dyn, Args> {
        pub unsafe fn new(
            args: Args,
            layout: fn() -> Layout,
            init: fn(VoidPtr, Args) -> Dyn::Object,
        ) -> Self {
            Self { args, layout, init }
        }

        pub fn layout(&self) -> Layout {
            (self.layout)()
        }

        pub unsafe fn init(self, slot: VoidPtr) -> Dyn::Object {
            (self.init)(slot, self.args)
        }
    }

    pub struct DynBox<T: ?Sized + DynCompatible>(ManuallyDrop<T::Object>);

    impl<T: ?Sized + DynCompatible> DynBox<T> {
        pub fn init<Args>(init: DynInit<T, Args>) -> Self {
            unsafe {
                let layout = init.layout();
                let slot = NonNull::new(std::alloc::alloc(layout))
                    .unwrap_or_else(|| std::alloc::handle_alloc_error(layout));
                let obj = init.init(slot.cast());
                Self(ManuallyDrop::new(obj))
            }
        }
    }
    impl<T: ?Sized + DynCompatible> Unpin for DynBox<T> {}
    impl<T: ?Sized + DynCompatible> Deref for DynBox<T> {
        type Target = T::Object;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: ?Sized + DynCompatible> DerefMut for DynBox<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<T: ?Sized + DynCompatible> Drop for DynBox<T> {
        fn drop(&mut self) {
            unsafe {
                let obj = ManuallyDrop::take(&mut self.0);
                let ptr = T::data(&obj);
                let layout = T::layout(&obj);
                drop(obj);
                std::alloc::dealloc(ptr.as_ptr().cast(), layout);
            }
        }
    }
}
pub use dyn_init::*;

// =========== 例子 ===========
mod example {
    pub use super::*;

    pub trait Async {
        type Item;

        async fn foo(&mut self, args: String) -> Self::Item;
    }

    pub trait DynAsync {
        type Item;

        fn foo(
            &mut self,
            args: String,
        ) -> DynInit<dyn Future<Output = Self::Item>, (VoidPtr, String)>;
    }

    impl<T> DynAsync for T
    where
        T: Async + Sized,
    {
        type Item = T::Item;

        fn foo(
            &mut self,
            args: String,
        ) -> DynInit<dyn Future<Output = Self::Item>, (VoidPtr, String)>
        where
            Self: Sized,
        {
            #[allow(unused_unsafe)]
            unsafe {
                DynInit::new(
                    (NonNull::from(self).cast(), args),
                    || return_type_layout(&<Self as Async>::foo),
                    |slot, (this, arg)| {
                        let foo = <Self as Async>::foo;
                        let val = unsafe { foo(this.cast().as_mut(), arg) };
                        unsafe { return_type_cast_ptr(&foo, slot).write(val) }
                        unsafe { return_type_object(&foo, slot) }
                    },
                )
            }
        }
    }

    pub unsafe fn return_type_object<I, F: Function<I>>(
        _: &F,
        data: VoidPtr,
    ) -> <F::Output as DynCompatible>::Object
    where
        F::Output: DynCompatible,
    {
        <F::Output as DynCompatible>::construct(data)
    }
}
use example::*;

async fn dynamic_dispatch<Item>(imp: &mut dyn DynAsync<Item = Item>, arg: String) -> Item {
    let foo_init = imp.foo(arg);
    let layout = dbg!(foo_init.layout());
    let mut stack = [0u8; 64];

    let start = &raw mut stack as *mut u8;
    let end = start.wrapping_add(stack.len());
    let slot = start.wrapping_add(start.align_offset(layout.align()));
    let slot_end = slot.wrapping_add(layout.size());

    // let byte_offset = unsafe { end.byte_offset_from(start) };
    // dbg!( start, end, byte_offset, slot, slot_end, stack.len(), layout.align(), layout.size());
    if slot >= start && slot_end <= end {
        println!("stack");
        unsafe { foo_init.init(NonNull::new_unchecked(slot).cast()).await }
    } else {
        println!("heap");
        unsafe { Pin::new_unchecked(DynBox::init(foo_init)) }.await
    }
}

struct AppendYay;
impl Async for AppendYay {
    type Item = String;
    async fn foo(&mut self, args: String) -> Self::Item {
        args + ", yay!"
    }
}
struct PrintYay;
impl Async for PrintYay {
    type Item = ();
    async fn foo(&mut self, args: String) -> Self::Item {
        println!("{args}, yay!");
    }
}

fn main() {
    pollster::block_on(run());
}

struct BorrowIt<'a>(&'a str);
impl<'a> Async for BorrowIt<'a> {
    type Item = &'a str;
    async fn foo(&mut self, _args: String) -> Self::Item {
        self.0
    }
}

async fn run() {
    test_return_type_layout();

    dynamic_dispatch(&mut PrintYay, "foo".to_owned()).await;

    let item = dynamic_dispatch(&mut AppendYay, "foo".to_owned()).await;
    assert_eq!(item, "foo, yay!");

    let s = String::from(":)");
    let mut borrow_it = BorrowIt(&s);
    let item = dynamic_dispatch(&mut borrow_it, Default::default()).await;
    assert_eq!(item, ":)");
}
