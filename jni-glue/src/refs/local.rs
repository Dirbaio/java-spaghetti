use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;

use jni_sys::*;

use crate::{Env, Global, Ref, ReferenceType};

/// A [Local](https://www.ibm.com/support/knowledgecenter/en/SSYKE2_8.0.0/com.ibm.java.vm.80.doc/docs/jni_refs.html),
/// non-null, reference to a Java object (+ &[Env]) limited to the current thread/stack.
///
/// Including the env allows for the convenient execution of methods without having to individually pass the env as an
/// argument to each and every one.  Since this is limited to the current thread/stack, these cannot be sanely stored
/// in any kind of static storage, nor shared between threads - instead use a [Global] if you need to do either.
///
/// Will DeleteLocalRef when dropped, invalidating the jobject but ensuring threads that rarely or never return to
/// Java may run without being guaranteed to eventually exhaust their local reference limit.  If this is not desired,
/// convert to a plain Ref with:
///
/// ```rust,no_run
/// # use jni_glue::*;
/// # fn example<T: ReferenceType>(local: Local<T>) {
/// let local = Local::leak(local);
/// # }
/// ```
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
#[repr(transparent)]
pub struct Local<'env, T: ReferenceType> {
    ref_: Ref<'env, T>,
}

// Could implement clone if necessary via NewLocalRef
// Do *not* implement Copy, cannot be safely done.

impl<'env, T: ReferenceType> Local<'env, T> {
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            ref_: Ref::from_raw(env, object),
        }
    }

    pub fn env(&self) -> Env<'env> {
        self.ref_.env()
    }

    pub fn as_raw(&self) -> jobject {
        self.ref_.as_raw()
    }

    pub fn into_raw(self) -> jobject {
        let object = self.ref_.as_raw();
        std::mem::forget(self); // Don't allow local to DeleteLocalRef the jobject
        object
    }

    pub fn leak(self) -> Ref<'env, T> {
        let result = self.ref_;
        std::mem::forget(self); // Don't allow local to DeleteLocalRef the jobject
        result
    }

    pub fn as_global(&self) -> Global<T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.ref_.as_raw()) };
        Global {
            object,
            vm: env.vm(),
            pd: PhantomData,
        }
    }

    pub fn cast<U: ReferenceType>(&self) -> Result<Local<'env, U>, crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.as_raw()) };
        let class2 = U::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        Ok(unsafe { Local::from_raw(env, object) })
    }
}

impl<'env, T: ReferenceType> Deref for Local<'env, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl<'env, T: ReferenceType> Clone for Local<'env, T> {
    fn clone(&self) -> Self {
        let env = self.env().as_raw();
        let object = unsafe { ((**env).v1_2.NewLocalRef)(env, self.as_raw()) };
        unsafe { Self::from_raw(self.env(), object) }
    }
}

impl<'env, T: ReferenceType> Drop for Local<'env, T> {
    fn drop(&mut self) {
        let env = self.env().as_raw();
        unsafe { ((**env).v1_2.DeleteLocalRef)(env, self.as_raw()) }
    }
}

impl<'env, T: ReferenceType + Debug> Debug for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'env, T: ReferenceType + Display> Display for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
