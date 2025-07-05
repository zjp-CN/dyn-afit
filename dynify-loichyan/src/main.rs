use dynify::*;
use std::{
    future::Future,
    mem::MaybeUninit,
    pin::{Pin, pin},
};

pub trait UserCommunication {
    #[allow(async_fn_in_trait)]
    async fn send_sms(&self, phone: &str, code: &str);
}

trait DynUserCommunication {
    fn send_sms<'this: 'ret, 'phone: 'ret, 'code: 'ret, 'ret>(
        &'this self,
        phone: &'phone str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'phone str, &'code str => dyn 'ret + Future<Output = ()>);
}
impl<T: UserCommunication> DynUserCommunication for T {
    fn send_sms<'this: 'ret, 'phone: 'ret, 'code: 'ret, 'ret>(
        &'this self,
        phone: &'phone str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'phone str, &'code str => dyn 'ret + Future<Output = ()>) {
        from_fn!(T::send_sms, self, phone, code)
    }
}

pub struct AuthenticationService {
    communicator: Box<dyn DynUserCommunication>,
}

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let auth = AuthenticationService {
        communicator: Box::new(Test { _a: 0 }),
    };
    auth.communicator.send_sms("abc", "123").pin_boxed().await;

    let stack = pin!(StackedBuf::<FUT_STACK_LEN>::new_uninit_buf());
    auth.communicator
        .send_sms("abc", "123")
        .pin_init(StackedBuf::new(stack))
        .await;
}

const FUT_STACK_LEN: usize = 128;

pub struct StackedBuf<'a, const LEN: usize>(Pin<&'a mut [MaybeUninit<u8>; LEN]>);

impl<const LEN: usize> StackedBuf<'_, LEN> {
    pub fn new_uninit_buf() -> [MaybeUninit<u8>; LEN] {
        [MaybeUninit::uninit(); LEN]
    }

    pub fn new(val: Pin<&'_ mut [MaybeUninit<u8>; LEN]>) -> StackedBuf<'_, LEN> {
        StackedBuf(val)
    }
}

unsafe impl<'a, T: 'a + ?Sized, const LEN: usize> Emplace<T> for StackedBuf<'a, LEN> {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: Construct<Object = T>,
    {
        Pin::into_inner(self.0).emplace(constructor)
    }
}
unsafe impl<'a, T: 'a + ?Sized, const LEN: usize> PinEmplace<T> for StackedBuf<'a, LEN> {}

#[derive(Debug)]
struct Test {
    _a: u8,
}
impl UserCommunication for Test {
    async fn send_sms(&self, phone: &str, code: &str) {
        println!("[{self:?}] {phone}: {code}")
    }
}
