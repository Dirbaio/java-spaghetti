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
/// **Not FFI Safe:**  `#[repr(rust)]`, and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// `*const JavaVM` in static and/or thread local storage instead of lugging it around in every [Global].  Of course, there's
/// no guarantee that's actually an *optimization*...
pub struct Global<T: ReferenceType> {
    object: jobject,
    vm: VM,
    pd: PhantomData<T>,
}

unsafe impl<T: ReferenceType> Send for Global<T> {}
unsafe impl<T: ReferenceType> Sync for Global<T> {}

impl<T: ReferenceType> Global<T> {
    /// Wraps an owned raw JNI global reference, taking the ownership.
    ///
    /// # Safety
    ///
    /// `object` must be an owned non-null JNI global reference to an object of type `T`,
    /// not to be deleted by another wrapper.
    pub unsafe fn from_raw(vm: VM, object: jobject) -> Self {
        Self {
            object,
            vm,
            pd: PhantomData,
        }
    }

    /// Gets the [VM] under which the JNI reference is created.
    pub fn vm(&self) -> VM {
        self.vm
    }

    /// Returns the raw JNI reference pointer.
    pub fn as_raw(&self) -> jobject {
        self.object
    }

    /// Leaks the `Global` and turns it into a raw pointer, preserving the ownership of
    /// one JNI global reference; prevents `DeleteGlobalRef` from being called on dropping.
    pub fn into_raw(self) -> jobject {
        let object = self.object;
        std::mem::forget(self); // Don't delete the object.
        object
    }

    /// Returns a new JNI local reference of the same Java object.
    pub fn as_local<'env>(&self, env: Env<'env>) -> Local<'env, T> {
        // Safety: this `Ref<'env, T>` isn't available outside, and it does nothing on dropping.
        let temp_ref = unsafe { Ref::from_raw(env, self.object) };
        temp_ref.as_local() // creates a new `Local`
    }

    /// Returns a [Ref], with which Java methods from generated bindings can be used.
    /// The lifetime of the returned [Ref] can be the intersection of this `Global`
    /// and a supposed local reference under `env`.
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
