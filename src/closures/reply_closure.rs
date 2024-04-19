use crate::z_reply_t;
use libc::c_void;
/// A closure is a structure that contains all the elements for stateful, memory-leak-free callbacks:
///
/// Members:
///   void *context: a pointer to an arbitrary state.
///   void *call(const struct z_owned_reply_t*, const void *context): the typical callback function. `context` will be passed as its last argument.
///   void *drop(void*): allows the callback's state to be freed.
///
/// Closures are not guaranteed not to be called concurrently.
///
/// It is guaranteed that:
///
///   - `call` will never be called once `drop` has started.
///   - `drop` will only be called **once**, and **after every** `call` has ended.
///   - The two previous guarantees imply that `call` and `drop` are never called concurrently.
#[repr(C)]
pub struct z_owned_closure_reply_t {
    context: *mut c_void,
    call: Option<extern "C" fn(z_reply_t, *mut c_void)>,
    drop: Option<extern "C" fn(*mut c_void)>,
}

impl z_owned_closure_reply_t {
    pub(crate) fn empty() -> Self {
        z_owned_closure_reply_t {
            context: std::ptr::null_mut(),
            call: None,
            drop: None,
        }
    }
}
unsafe impl Send for z_owned_closure_reply_t {}
unsafe impl Sync for z_owned_closure_reply_t {}
impl Drop for z_owned_closure_reply_t {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            drop(self.context)
        }
    }
}
/// Constructs a null safe-to-drop value of 'z_owned_closure_reply_t' type
#[no_mangle]
pub extern "C" fn z_closure_reply_null() -> z_owned_closure_reply_t {
    z_owned_closure_reply_t::empty()
}
/// Calls the closure. Calling an uninitialized closure is a no-op.
#[no_mangle]
pub extern "C" fn z_closure_reply_call(
    closure: &z_owned_closure_reply_t,
    reply: z_reply_t,
) {
    match closure.call {
        Some(call) => call(reply, closure.context),
        None => {
            log::error!("Attempted to call an uninitialized closure!");
        }
    }
}
/// Drops the closure. Droping an uninitialized closure is a no-op.
#[no_mangle]
pub extern "C" fn z_closure_reply_drop(closure: &mut z_owned_closure_reply_t) {
    let mut empty_closure = z_owned_closure_reply_t::empty();
    std::mem::swap(&mut empty_closure, closure);
}
impl<F: Fn(z_reply_t)> From<F> for z_owned_closure_reply_t {
    fn from(f: F) -> Self {
        let this = Box::into_raw(Box::new(f)) as _;
        extern "C" fn call<F: Fn(z_reply_t)>(
            response: z_reply_t,
            this: *mut c_void,
        ) {
            let this = unsafe { &*(this as *const F) };
            this(response)
        }
        extern "C" fn drop<F>(this: *mut c_void) {
            std::mem::drop(unsafe { Box::from_raw(this as *mut F) })
        }
        z_owned_closure_reply_t {
            context: this,
            call: Some(call::<F>),
            drop: Some(drop::<F>),
        }
    }
}
