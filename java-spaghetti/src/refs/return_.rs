use std::marker::PhantomData;
use std::ptr::null_mut;

use jni_sys::jobject;

use crate::ReferenceType;

/// FFI: Use **Return\<java::lang::Object\>** instead of jobject.  This represents a (null?) JNI function call return value.
///
/// Unlike most Java reference types from this library, this *can* be null.
///
/// FFI safe where a jobject is safe, assuming you match your types correctly.
#[repr(transparent)]
pub struct Return<'env, T: ReferenceType> {
    object: jobject,
    _class: PhantomData<&'env T>,
}

impl<'env, T: ReferenceType> Return<'env, T> {
    pub unsafe fn from_raw(object: jobject) -> Self {
        Self {
            object,
            _class: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self {
            object: null_mut(),
            _class: PhantomData,
        }
    }

    pub fn as_raw(&self) -> jobject {
        self.object
    }
}
