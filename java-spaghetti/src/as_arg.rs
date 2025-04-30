use std::ptr::null_mut;

use jni_sys::jobject;

use crate::{AsJValue, AssignableTo, Global, Local, Null, Ref, ReferenceType};

/// A marker trait indicating this is a valid JNI reference type for Java method argument
/// type `T`, this can be null.
///
/// # Safety
///
/// It should be implemented automatically by `java_spaghetti`.
pub unsafe trait AsArg<T>: Sized + AsJValue {
    fn as_arg(&self) -> jobject;
}

unsafe impl<T: ReferenceType, U: AsArg<T>> AsArg<T> for &U {
    fn as_arg(&self) -> jobject {
        U::as_arg(self)
    }
}

unsafe impl<T: ReferenceType, U: AsArg<T>> AsArg<T> for &mut U {
    fn as_arg(&self) -> jobject {
        U::as_arg(self)
    }
}

unsafe impl<T: ReferenceType> AsArg<T> for Null {
    fn as_arg(&self) -> jobject {
        null_mut()
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Ref<'_, U> {
    fn as_arg(&self) -> jobject {
        self.as_raw()
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<Ref<'_, U>> {
    fn as_arg(&self) -> jobject {
        self.as_ref().map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<&Ref<'_, U>> {
    fn as_arg(&self) -> jobject {
        self.map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Local<'_, U> {
    fn as_arg(&self) -> jobject {
        self.as_raw()
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<Local<'_, U>> {
    fn as_arg(&self) -> jobject {
        self.as_ref().map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<&Local<'_, U>> {
    fn as_arg(&self) -> jobject {
        self.map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Global<U> {
    fn as_arg(&self) -> jobject {
        self.as_raw()
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<Global<U>> {
    fn as_arg(&self) -> jobject {
        self.as_ref().map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}

unsafe impl<T: ReferenceType, U: AssignableTo<T>> AsArg<T> for Option<&Global<U>> {
    fn as_arg(&self) -> jobject {
        self.map(|r| r.as_raw()).unwrap_or(null_mut())
    }
}
