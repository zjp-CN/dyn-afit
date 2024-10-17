
# Dynamic dispatch on `async fn`

## What's wanted?

```rust
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

async fn call(file: &mut dyn AsyncRead /* 👈 */) -> io::Result<usize> {
    file.read(&mut vec![]).await
}
```

1. call `async fn` in a trait object
2. no heap allocation on return value, i.e. no `-> Pin<Box<dyn ...>>`

`Send` and `Sync` problems are not considered for now.

## Status quo

### AFIT trait objects are not possible

[Dynamic **async fn in trait** (**AFIT**) is just not supported.](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html#dynamic-dispatch)

You'll see compiler errors like this:

```rust
error[E0038]: the trait `AsyncRead` cannot be made into an object
  --> src/main.rs:22:26
   |
22 | async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
   |                          ^^^^^^^^^^^^^ `AsyncRead` cannot be made into an object
   |
note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically; for more information visit <https://doc.rust-lang.
org/reference/items/traits.html#object-safety>
  --> src/main.rs:19:14
   |
18 | pub trait AsyncRead {
   |           --------- this trait cannot be made into an object...
19 |     async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
   |              ^^^^ ...because method `read` is `async`
   = help: consider moving `read` to another trait
```

The good news is Async WG is working hard on it these years.

### Workaround 1: `#[async_trait]`

[`#[async_trait]`](https://docs.rs/async-trait) is the most widely used approach to get a working async trait object.

The core idea is that [`BoxFuture`] is a concrete return type which meets one of the requirements in [object safety].[^dyn-trait]

[`BoxFuture`]: https://docs.rs/futures/latest/futures/future/type.BoxFuture.html
[object safety]: https://doc.rust-lang.org/reference/items/traits.html#object-safety

[^dyn-trait]: [quinedot explains all dyn trait stuff really well](https://quinedot.github.io/rust-learning/dyn-trait.html) if you're
not familiar with it.

Downside: this allocates a return value on the heap, which is not we want.

```rust
// This expands to pub trait AsyncRead { fn read(...) -> BoxFuture<...>  }
#[async_trait::async_trait]
pub trait AsyncRead { // trait object safe
    // but at the cost of returning Box<...>.
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

pub async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
    file.read(&mut []).await
}
```


## Workaround 2: `StackFuture`

Currently, we can use AFIT with stack allocated return value via [`StackFuture`].

[`StackFuture`]: https://docs.rs/stackfuture/latest/stackfuture/struct.StackFuture.html

```rust
pub trait AsyncRead {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> StackFuture<'a, io::Result<usize>, 128>;
}

pub async fn call(file: &mut dyn AsyncRead) -> io::Result<usize> {
    file.read(&mut []).await
}
```

Most stack allocated types in Rust carry a const generic parameter as a maximum allocation size.

It's also interesting to see `StackFuture` can fall back to heap allocation when the the size of `Future` type
exceeds the const size.

This is just as simple as `async-trait`, and works well.

Technically, the const generic can be moved to the trait argument, and specify the const value when using it.

```rust
pub trait AsyncRead<const N: usize> {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> StackFuture<'a, io::Result<usize>, N>;
}

pub async fn call(file: &mut dyn AsyncRead<64>) -> io::Result<usize> {
    file.read(&mut []).await
}
```

## Workaround 3: static dispatch on trait objects

`async-std` crate defines an extension trait [`ReadExt`](https://docs.rs/async-std/latest/async_std/io/trait.ReadExt.html#method.read) over [`Read`](https://docs.rs/async-std/latest/async_std/io/trait.Read.html).

`Read` trait is object safe, thus we can erase types when constructing it.

Then due to inherentance through [super traits](https://doc.rust-lang.org/reference/items/traits.html#supertraits),
we can call async fns defined in sub traits:

```rust
// This is object safe:
pub trait Read {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>;
}

// Extension trait as a user interface:
pub trait /* not object safe */ AsyncRead: Read {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFuture<'a, Self> where Self: Unpin { ... }
}

// Glue code and poll the future to read data:
impl<T: Read + Unpin + ?Sized> Future for ReadFuture<'_, T> { ... }

// Blanket impl to make async call.
impl<T: Read + ?Sized> AsyncRead for T {}

pub async fn call(file: &mut (dyn Read + Unpin)) -> io::Result<usize> {
    // calling async read is a static dispatch on Read trait object
    file.read(&mut []).await
}
```

So the pattern is 
* a Future-API-styled base trait object
* extension subtrait APIs over the trait object, and a return Future (generic but not heap allocated)
* a blanket impl for the base trait object
* in the async fn call, statically call extension APIs on the trait object

## Workaround 4: AFIT on trait objects

Since AFIT static dispatch works well, we can reduce some boilerplate code above to this:

```rust
pub trait Read {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>;
}

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
```

## Workaround 5: `#[dynosaur]`

[`#[dynosaur]`](https://docs.rs/dynosaur/latest/dynosaur/attr.dynosaur.html) is a proc macro to
generate a dynamic dispatch adaptor type `DynTrait` for the implemented `Trait`.

You should use the generated type `DynTrait` as `dyn Trait`.

```rust
#[dynosaur::dynosaur(DynAsyncRead)]
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

pub async fn call(file: &mut DynAsyncRead<'_> /* 👈 not dyn AsyncRead */) -> io::Result<usize> {
    file.read(&mut []).await
}
```

The interesting part on `DynTrait` is the caller can choose creating a boxed erased type or 
a referenced erased type when instantiating `DynTrait`. 

<details>

<summary>Click to see the (partial) macro expansion.</summary>


```rust
pub trait ErasedAsyncRead {
    fn read<'life0, 'life1, 'dynosaur>(
        &'life0 mut self,
        buf: &'life1 mut [u8],
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = io::Result<usize>> + 'dynosaur>,
    >
    where
        'life0: 'dynosaur,
        'life1: 'dynosaur,
        Self: 'dynosaur;
}

#[repr(transparent)]
pub struct DynAsyncRead<'dynosaur_struct> {
    ptr: dyn ErasedAsyncRead + 'dynosaur_struct,
}

impl<'dynosaur_struct> AsyncRead for DynAsyncRead<'dynosaur_struct> {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> impl ::core::future::Future<Output = io::Result<usize>> {
        let fut: ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = io::Result<usize>> + '_>,
        > = self.ptr.read(buf);
        let fut: ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = io::Result<usize>> + 'static>,
        > = unsafe { ::core::mem::transmute(fut) };
        fut
    }
}

impl<'dynosaur_struct> DynAsyncRead<'dynosaur_struct> {
    pub fn new(
        value: Box<impl AsyncRead + 'dynosaur_struct>,
    ) -> Box<DynAsyncRead<'dynosaur_struct>> {
        let value: Box<dyn ErasedAsyncRead + 'dynosaur_struct> = value;
        unsafe { ::core::mem::transmute(value) }
    }

    pub fn boxed(
        value: impl AsyncRead + 'dynosaur_struct,
    ) -> Box<DynAsyncRead<'dynosaur_struct>> {
        Self::new(Box::new(value))
    }

    pub fn from_ref(
        value: &(impl AsyncRead + 'dynosaur_struct),
    ) -> &DynAsyncRead<'dynosaur_struct> {
        let value: &(dyn ErasedAsyncRead + 'dynosaur_struct) = &*value;
        unsafe { ::core::mem::transmute(value) }
    }

    pub fn from_mut(
        value: &mut (impl AsyncRead + 'dynosaur_struct),
    ) -> &mut DynAsyncRead<'dynosaur_struct> {
        let value: &mut (dyn ErasedAsyncRead + 'dynosaur_struct) = &mut *value;
        unsafe { ::core::mem::transmute(value) }
    }
}
```

</details>

Note: this approach means
* `AsyncRead` we defined is still static dispatchable only
* there is a heap allocation cost when calling read on DynAsyncRead, which is the same as `#[async_trait]`
  * in a sense, `#[async_trait]` is more recommended over this

```rust
// dynamic dispatch: with boxing overhead once (in calling read on DynAsyncRead)
let dyn_async_read = DynAsyncRead::from_mut(&mut file);
_ = dbg!(call(dyn_async_read).await);

// dynamic dispatch: with boxing overhead twice in
// * creating a Boxed DynAsyncRead value
// * and calling read on DynAsyncRead
let mut box_dyn_async_read = DynAsyncRead::boxed(file);
_ = dbg!(call(&mut box_dyn_async_read).await);
```

## Summary

The code is in `examples` folder:

| \# | file name                                    | is directly dispatchable | no head allocated return value | extra description                                               |
|:--:|----------------------------------------------|:------------------------:|:------------------------------:|-----------------------------------------------------------------|
|  1 | `returns-box-trait-object.rs`                |            ✅            |               ❌               | widely used; simple                                             |
|  2 | `returns-stack-future.rs`                    |            ✅            |               ✅               | simple; fixed allocation size but heap allocation as a fallback |
|  3 | `returns-future-in-trait-with-supertrait.rs` |            ❌            |               ✅               | used in `async-std`; the pattern is an inspiration              |
|  4 | `afit-with-supertrait.rs`                    |            ❌            |               ✅               | takes the inspiration above with AFIT                           |
|  5 | `dynosaur.rs`                                |            ❌            |               ❌               | promising idea and APIs to support referenced erased types      |

[I made a post about this on URLO.](https://users.rust-lang.org/t/dynamic-dispatch-on-async-fn/119879)

## Ongoing Language Designs on this topic

`dyn-star` / `dynx` types:

* [dyn*: can we make dyn sized?](https://smallcultfollowing.com/babysteps/blog/2022/03/29/dyn-can-we-make-dyn-sized/)
  | [我的翻译](https://zjp-cn.github.io/translation/dyn-async-traits/2022-03-29-dyn-can-we-make-dyn-sized.html)
* [Async fn in dyn trait](https://rust-lang.github.io/async-fundamentals-initiative/explainer/async_fn_in_dyn_trait.html)
