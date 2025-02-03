use std::cell::{Cell, OnceCell};
use std::ptr::null_mut;

use jni_sys::*;

use crate::Env;

/// FFI: Use **&VM** instead of *const JavaVM.  This represents a global, process-wide Java exection environment.
///
/// On Android, there is only one VM per-process, although on desktop it's possible (if rare) to have multiple VMs
/// within the same process.  This library does not support having multiple VMs active simultaniously.
///
/// This is a "safe" alternative to jni_sys::JavaVM raw pointers, with the following caveats:
///
/// 1)  A null vm will result in **undefined behavior**.  Java should not be invoking your native functions with a null
///     *mut JavaVM, however, so I don't believe this is a problem in practice unless you've bindgened the C header
///     definitions elsewhere, calling them (requiring `unsafe`), and passing null pointers (generally UB for JNI
///     functions anyways, so can be seen as a caller soundness issue.)
///
/// 2)  Allowing the underlying JavaVM to be modified is **undefined behavior**.  I don't believe the JNI libraries
///     modify the JavaVM, so as long as you're not accepting a *mut JavaVM elsewhere, using unsafe to dereference it,
///     and mucking with the methods on it yourself, I believe this "should" be fine.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VM(*mut JavaVM);

impl VM {
    pub fn as_raw(&self) -> *mut JavaVM {
        self.0
    }

    pub unsafe fn from_raw(vm: *mut JavaVM) -> Self {
        Self(vm)
    }

    pub fn with_env<F, R>(&self, callback: F) -> R
    where
        F: for<'env> FnOnce(Env<'env>) -> R,
    {
        let mut env = null_mut();
        let just_attached = match unsafe { ((**self.0).v1_2.GetEnv)(self.0, &mut env, JNI_VERSION_1_2) } {
            JNI_OK => false,
            JNI_EDETACHED => {
                let ret = unsafe { ((**self.0).v1_2.AttachCurrentThread)(self.0, &mut env, null_mut()) };
                if ret != JNI_OK {
                    panic!("AttachCurrentThread returned unknown error: {}", ret)
                }
                if !get_thread_exit_flag() {
                    set_thread_attach_flag(self.0);
                }
                true
            }
            JNI_EVERSION => panic!("GetEnv returned JNI_EVERSION"),
            unexpected => panic!("GetEnv returned unknown error: {}", unexpected),
        };

        let result = callback(unsafe { Env::from_raw(env as _) });

        if just_attached && get_thread_exit_flag() {
            // this is needed in case of `with_env` is used on dropping some thread-local instance.
            unsafe { ((**self.0).v1_2.DetachCurrentThread)(self.0) };
        }

        result
    }
}

unsafe impl Send for VM {}
unsafe impl Sync for VM {}

thread_local! {
    static THREAD_ATTACH_FLAG: Cell<Option<AttachFlag>> = const { Cell::new(None) };
    static THREAD_EXIT_FLAG: OnceCell<()> = const { OnceCell::new() };
}

struct AttachFlag {
    raw_vm: *mut JavaVM,
}

impl Drop for AttachFlag {
    fn drop(&mut self) {
        // avoids the fatal error "Native thread exiting without having called DetachCurrentThread"
        unsafe { ((**self.raw_vm).v1_2.DetachCurrentThread)(self.raw_vm) };
        let _ = THREAD_EXIT_FLAG.try_with(|flag| flag.set(()));
    }
}

fn set_thread_attach_flag(raw_vm: *mut JavaVM) {
    THREAD_ATTACH_FLAG.replace(Some(AttachFlag { raw_vm }));
}

fn get_thread_exit_flag() -> bool {
    THREAD_EXIT_FLAG.try_with(|flag| flag.get().is_some()).unwrap_or(true)
}
