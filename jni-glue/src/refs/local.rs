use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;

use jni_sys::*;

use crate::{Env, Global, ObjectAndEnv, Ref, ReferenceType};

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
/// # fn example<Class: ReferenceType>(local: Local<Class>) {
/// let local = Local::leak(local);
/// # }
/// ```
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
///
/// [Env]:    struct.Env.html
/// [Global]: struct.Global.html
pub struct Local<'env, Class: ReferenceType> {
    pub(crate) oae: ObjectAndEnv,
    pub(crate) _env: PhantomData<Env<'env>>,
    pub(crate) _class: PhantomData<&'env Class>,
}

// Could implement clone if necessary via NewLocalRef
// Do *not* implement Copy, cannot be safely done.

impl<'env, Class: ReferenceType> Local<'env, Class> {
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            oae: ObjectAndEnv {
                object,
                env: env.as_raw(),
            },
            _env: PhantomData,
            _class: PhantomData,
        }
    }

    pub fn env(&self) -> Env<'env> {
        unsafe { Env::from_raw(self.oae.env) }
    }

    pub fn as_raw(&self) -> jobject {
        self.oae.object
    }

    pub fn into_raw(self) -> jobject {
        let object = self.oae.object;
        std::mem::forget(self); // Don't allow local to DeleteLocalRef the jobject
        object
    }

    pub fn leak(self) -> Ref<'env, Class> {
        let result = Ref {
            oae: ObjectAndEnv {
                object: self.oae.object,
                env: self.oae.env,
            },
            _env: PhantomData,
            _class: PhantomData,
        };
        std::mem::forget(self); // Don't allow local to DeleteLocalRef the jobject
        result
    }

    pub fn as_global(&self) -> Global<Class> {
        let env = unsafe { Env::from_raw(self.oae.env) };
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.oae.object) };
        Global {
            object,
            vm: env.vm(),
            pd: PhantomData,
        }
    }

    pub fn cast<Class2: ReferenceType>(&self) -> Result<Local<'env, Class2>, crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.oae.object) };
        let class2 = Class2::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.oae.object) };
        Ok(unsafe { Local::from_raw(env, object) })
    }
}

impl<'env, Class: ReferenceType> Deref for Local<'env, Class> {
    type Target = Class;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(&self.oae as *const ObjectAndEnv as *const Self::Target) }
    }
}

impl<'env, Class: ReferenceType> Clone for Local<'env, Class> {
    fn clone(&self) -> Self {
        let env = self.oae.env;
        let object = unsafe { ((**env).v1_2.NewLocalRef)(env, self.oae.object) };
        unsafe { Self::from_raw(self.env(), object) }
    }
}

impl<'env, Class: ReferenceType> Drop for Local<'env, Class> {
    fn drop(&mut self) {
        let env = self.oae.env;
        unsafe { ((**env).v1_2.DeleteLocalRef)(env, self.oae.object) }
    }
}

impl<'env, Class: ReferenceType + Debug> Debug for Local<'env, Class> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'env, Class: ReferenceType + Display> Display for Local<'env, Class> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
