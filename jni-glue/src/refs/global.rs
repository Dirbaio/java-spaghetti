use std::marker::PhantomData;

use jni_sys::*;

use crate::{Env, Local, Ref, ReferenceType, VM};

/// A [Global](https://www.ibm.com/support/knowledgecenter/en/SSYKE2_8.0.0/com.ibm.java.vm.80.doc/docs/jni_refs.html),
/// non-null, reference to a Java object (+ [VM]).
///
/// Unlike Local, this can be stored statically and shared between threads.  This has a few caveats:
/// * You must create a [Ref] before use.
/// * The [Global] can be invalidated if the [VM] is unloaded.
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient [VM], we can likely store the
/// *const JavaVM in static and/or thread local storage instead of lugging it around in every [Local].  Of course, there's
/// no guarantee that's actually an *optimization*...
pub struct Global<T: ReferenceType> {
    pub(crate) object: jobject,
    pub(crate) vm: VM,
    pub(crate) pd: PhantomData<T>,
}

unsafe impl<T: ReferenceType> Send for Global<T> {}
unsafe impl<T: ReferenceType> Sync for Global<T> {}

impl<T: ReferenceType> Global<T> {
    pub unsafe fn from_raw(vm: VM, object: jobject) -> Self {
        Self {
            object,
            vm,
            pd: PhantomData,
        }
    }

    pub fn vm(&self) -> VM {
        self.vm
    }

    pub fn as_raw(&self) -> jobject {
        self.object
    }

    pub fn into_raw(self) -> jobject {
        let object = self.object;
        std::mem::forget(self); // Don't delete the object.
        object
    }

    pub fn with<'env>(&self, env: Env<'env>) -> Ref<'env, T> {
        assert_eq!(self.vm, env.vm()); // Soundness check - env *must* belong to the same VM!
        unsafe { self.with_unchecked(env) }
    }

    pub unsafe fn with_unchecked<'env>(&self, env: Env<'env>) -> Ref<'env, T> {
        Ref::from_raw(env, self.object)
    }
}

impl<'env, T: ReferenceType> From<Local<'env, T>> for Global<T> {
    fn from(local: Local<'env, T>) -> Global<T> {
        local.as_global()
    }
}

impl<T: ReferenceType> Clone for Global<T> {
    fn clone(&self) -> Self {
        self.vm.with_env(|env| {
            let env = env.as_raw();
            let object = unsafe { ((**env).v1_2.NewGlobalRef)(env, self.object) };
            Self {
                object,
                vm: self.vm,
                pd: PhantomData,
            }
        })
    }
}

impl<T: ReferenceType> Drop for Global<T> {
    fn drop(&mut self) {
        self.vm.with_env(|env| {
            let env = env.as_raw();
            unsafe { ((**env).v1_2.DeleteGlobalRef)(env, self.object) }
        });
    }
}
