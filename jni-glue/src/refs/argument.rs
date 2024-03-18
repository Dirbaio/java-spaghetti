use std::marker::PhantomData;

use jni_sys::*;

use crate::{Env, Global, Ref, ReferenceType};

/// FFI: Use **Argument\<java::lang::Object\>** instead of jobject.  This represents a (null?) function argument.
///
/// Unlike most Java reference types from this library, this *can* be null.
///
/// FFI safe where a jobject is safe, assuming you match your types correctly.  Using the wrong type may result in
/// soundness issues, but at least on Android mostly seems to just result in JNI aborting execution for the current
/// process when calling methods on an instance of the wrong type.
#[repr(transparent)]
pub struct Argument<T: ReferenceType> {
    object: jobject,
    _class: PhantomData<T>,
}

impl<T: ReferenceType> Argument<T> {
    /// **unsafe**:  There's no guarantee the jobject being passed is valid or null, nor any means of checking it.
    pub unsafe fn from_raw(object: jobject) -> Self {
        Self {
            object,
            _class: PhantomData,
        }
    }

    pub fn as_raw(&self) -> jobject {
        self.object
    }

    /// **unsafe**:  This assumes the argument belongs to the given Env/VM, which is technically unsound.  However, the
    /// intended use case of immediately converting any Argument s into ArgumentRef s at the start of a JNI callback,
    /// where Java directly invoked your function with an Env + arguments, is sound.
    pub unsafe fn with_unchecked<'env>(&'env self, env: Env<'env>) -> Option<Ref<'env, T>> {
        if self.object.is_null() {
            None
        } else {
            Some(Ref::from_raw(env, self.object))
        }
    }

    /// **unsafe**:  This assumes the argument belongs to the given Env/VM, which is technically unsound.  However, the
    /// intended use case of immediately converting any Argument s into ArgumentRef s at the start of a JNI callback,
    /// where Java directly invoked your function with an Env + arguments, is sound.
    pub unsafe fn into_global(self, env: Env) -> Option<Global<T>> {
        if self.object.is_null() {
            None
        } else {
            let jnienv = env.as_raw();
            let object = ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.object);
            Some(Global {
                object,
                vm: env.vm(),
                pd: PhantomData,
            })
        }
    }
}
