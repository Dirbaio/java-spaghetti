use std::marker::PhantomData;
use std::ptr::null_mut;

use jni_sys::jobject;

use crate::ReferenceType;

/// FFI: Use **Return\<java::lang::Object\>** instead of `jobject`.  This represents a (null?) JNI function call return value.
///
/// Unlike most Java reference types from this library, this *can* be null. Recommended constructors are
/// [crate::Local::into_return] and [Return::null].
///
/// FFI safe where a jobject is safe, assuming you match your types correctly.
#[repr(transparent)]
pub struct Return<'env, T: ReferenceType> {
    object: jobject,
    _class: PhantomData<&'env T>,
}

impl<'env, T: ReferenceType> Return<'env, T> {
    /// Wraps a raw JNI reference.
    ///
    /// # Safety
    ///
    /// - If `object` is non-null, it must be a JNI local(?) reference to an instance of type `T`;
    /// - `object` must keep valid for `'env` lifetime; it is not owned by `Local` or any other wrapper
    ///   that deletes the reference on `Drop` before the JNI native method call returns.
    pub unsafe fn from_raw(object: jobject) -> Self {
        Self {
            object,
            _class: PhantomData,
        }
    }

    /// Creates a null value to be returned from the JNI native method.
    pub fn null() -> Self {
        Self {
            object: null_mut(),
            _class: PhantomData,
        }
    }

    /// Returns the raw JNI reference pointer. Generally it should not be used.
    pub fn as_raw(&self) -> jobject {
        self.object
    }
}

impl<'env, T: ReferenceType> Default for Return<'env, T> {
    /// This is a null value.
    fn default() -> Self {
        Self::null()
    }
}
