use std::marker::PhantomData;

use jni_sys::*;

use crate::{Env, Local, Ref, ReferenceType, VM};

/// A [Global](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=collector-overview-jni-object-references),
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
    object: jobject,
    vm: VM,
    pd: PhantomData<T>,
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

    pub fn as_local<'env>(&self, env: Env<'env>) -> Local<'env, T> {
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Local::from_raw(env, object) }
    }

    pub fn as_ref<'env>(&'env self, env: Env<'env>) -> Ref<'env, T> {
        unsafe { Ref::from_raw(env, self.object) }
    }
}

impl<'env, T: ReferenceType> From<Local<'env, T>> for Global<T> {
    fn from(x: Local<'env, T>) -> Self {
        x.as_global()
    }
}

impl<'env, T: ReferenceType> From<Ref<'env, T>> for Global<T> {
    fn from(x: Ref<'env, T>) -> Self {
        x.as_global()
    }
}

impl<'env, T: ReferenceType> From<&Local<'env, T>> for Global<T> {
    fn from(x: &Local<'env, T>) -> Self {
        x.as_global()
    }
}

impl<'env, T: ReferenceType> From<&Ref<'env, T>> for Global<T> {
    fn from(x: &Ref<'env, T>) -> Self {
        x.as_global()
    }
}

impl<T: ReferenceType> Clone for Global<T> {
    fn clone(&self) -> Self {
        self.vm.with_env(|env| {
            let env = env.as_raw();
            let object = unsafe { ((**env).v1_2.NewGlobalRef)(env, self.object) };
            assert!(!object.is_null());
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
