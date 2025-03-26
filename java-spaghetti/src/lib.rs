//! Common glue code between Rust and JNI, used in auto-generated `java-spaghetti` glue code.
//!
//! See also the [Android JNI tips](https://developer.android.com/training/articles/perf-jni) documentation as well as the
//! [Java Native Interface Specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/jniTOC.html).
//!
//! Just like [jni-rs](https://docs.rs/jni/latest/jni/), thread safety of accessing Java objects are not guaranteed, unless
//! they are thread-safe by themselves.

#![feature(arbitrary_self_types)]

use std::borrow::Cow;
use std::fmt;
use std::ptr::null_mut;

/// public jni-sys reexport.
pub use ::jni_sys as sys;
use sys::{jobject, jvalue};

mod refs {
    mod arg;
    mod global;
    mod local;
    mod ref_;
    mod return_;

    pub use arg::*;
    pub use global::*;
    pub use local::*;
    pub use ref_::*;
    pub use return_::*;
}

mod array;
mod as_jvalue;
mod env;
mod id_cache;
mod jni_type;
mod string_chars;
mod vm;

pub use array::*;
pub use as_jvalue::*;
pub use env::*;
pub use id_cache::*;
pub use jni_type::JniType;
pub use refs::*;
pub use string_chars::*;
pub use vm::*;

/// Error returned on failed `.cast()`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CastError;

impl std::error::Error for CastError {}
impl fmt::Display for CastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cast failed")
    }
}

/// A marker trait indicating this is a valid exception type that all exceptions thrown by Java
/// should be compatible with.
pub trait ThrowableType: ReferenceType {}

/// JNI bindings rely on this type being accurate.
///
/// You should generally not be interacting with this type directly, but it must be public for codegen.
///
/// # Safety
///
/// **unsafe**:  `jni_reference_type_name` must pass a string terminated by '\0'.  Failing to do so is a soundness bug, as
/// the string is passed directly to JNI as a raw pointer!  Additionally, passing the wrong type may be a soundness bug
/// as although the Android JVM will simply panic and abort, I have no idea if that is a guarantee or not.
#[doc(hidden)]
pub unsafe trait ReferenceType: JniType + Sized + 'static {
    /// Returns a string value compatible with JNI
    /// [FindClass](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#FindClass).
    fn jni_reference_type_name() -> Cow<'static, str>;

    /// Returns a cached `JClass` of the class object for this reference type.
    ///
    /// There could not be a default implementation holding a static `OnceLock`: the compiler may not
    /// generate an independent static item for each generated struct that implements `ReferenceType`.
    fn jni_get_class<'env>(env: Env<'env>) -> &'static JClass;
}

/// Marker trait indicating `Self` can be assigned to `T`.
///
/// # Safety
///
/// `T` is a superclass or superinterface of `Self`.
pub unsafe trait AssignableTo<T: ReferenceType>: ReferenceType {}

/// A type is always assignable to itself.
unsafe impl<T: ReferenceType> AssignableTo<T> for T {}

/// A trait similar to `Display`.
pub trait JavaDisplay: ReferenceType {
    fn fmt(self: &Ref<'_, Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// A trait similar to `Debug`. Currently it is implemented by `Throwable` in generated bindings.
pub trait JavaDebug: ReferenceType {
    fn fmt(self: &Ref<'_, Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// A marker trait indicating this is a valid JNI reference type for Java method argument
/// type `T`, this can be null.
///
/// # Safety
///
/// It should be implemented automatically by `java_spaghetti`.
pub unsafe trait AsArg<T>: Sized {
    fn as_arg(&self) -> jobject;
    fn as_arg_jvalue(&self) -> jvalue {
        jvalue { l: self.as_arg() }
    }
}

/// Represents a Java `null` value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Null;

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
