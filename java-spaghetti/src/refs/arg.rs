use std::marker::PhantomData;

use jni_sys::*;

use crate::{Env, Global, Local, Ref, ReferenceType};

/// FFI: Use **Arg\<java::lang::Object\>** instead of `jobject`.  This represents a (null?) function argument.
///
/// Unlike most Java reference types from this library, this *can* be null.
///
/// FFI safe where a `jobject` is safe, assuming you match your types correctly.  Using the wrong type may result in
/// soundness issues, but at least on Android mostly seems to just result in JNI aborting execution for the current
/// process when calling methods on an instance of the wrong type.
#[repr(transparent)]
pub struct Arg<T: ReferenceType> {
    object: jobject,
    _class: PhantomData<T>,
}

impl<T: ReferenceType> Arg<T> {
    /// # Safety
    ///
    /// **unsafe**:  There's no guarantee the `jobject` being passed is valid or null, nor any means of checking it.
    pub unsafe fn from_raw(object: jobject) -> Self {
        Self {
            object,
            _class: PhantomData,
        }
    }

    /// Returns the raw JNI reference pointer.
    pub fn as_raw(&self) -> jobject {
        self.object
    }

    /// # Safety
    ///
    /// **unsafe**:  This assumes the argument belongs to the given [Env], which is technically unsound.  However,
    /// the intended use case of immediately converting any [Arg] into [Ref] at the start of a JNI callback,
    /// where Java directly invoked your function with an [Env] + arguments, is sound.
    pub unsafe fn into_ref<'env>(self, env: Env<'env>) -> Option<Ref<'env, T>> {
        if self.object.is_null() {
            None
        } else {
            Some(Ref::from_raw(env, self.object))
        }
    }

    /// # Safety
    ///
    /// **unsafe**:  This assumes the argument belongs to the given [Env], which is technically unsound.  However,
    /// the intended use case of immediately converting any [Arg] into [Local] at the start of a JNI callback,
    /// where Java directly invoked your function with an [Env] + arguments, is sound.
    pub unsafe fn into_local<'env>(self, env: Env<'env>) -> Option<Local<'env, T>> {
        self.into_ref(env).map(|r| r.as_local())
    }

    /// This equals [Arg::into_ref] + [Ref::as_global].
    ///
    /// # Safety
    ///
    /// **unsafe**:  The same as [Arg::into_ref].
    pub unsafe fn into_global(self, env: Env) -> Option<Global<T>> {
        self.into_ref(env).as_ref().map(Ref::as_global)
    }
}
