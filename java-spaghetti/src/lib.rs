//! Common glue code between Rust and JNI, used in auto-generated `java-spaghetti` glue code.
//!
//! See also the [Android JNI tips](https://developer.android.com/training/articles/perf-jni) documentation as well as the
//! [Java Native Interface Specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/jniTOC.html).
//!
//! Just like [jni-rs](https://docs.rs/jni/latest/jni/), thread safety of accessing Java objects are not guaranteed, unless
//! they are thread-safe by themselves.

#![feature(arbitrary_self_types)]

use std::fmt;

/// public jni-sys reexport.
pub use ::jni_sys as sys;

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
mod as_arg;
mod as_jvalue;
mod env;
mod id_cache;
mod jni_type;
mod string_chars;
mod vm;

pub use array::*;
pub use as_arg::*;
pub use as_jvalue::*;
pub use env::*;
pub use id_cache::*;
pub use jni_type::JniType;
pub use refs::*;
pub use string_chars::*;
pub use vm::*;

/// Error returned on failed `.cast()`.`
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CastError;

impl std::error::Error for CastError {}
impl fmt::Display for CastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cast failed")
    }
}

/// A marker type indicating this is a valid exception type that all exceptions thrown by java should be compatible with
pub trait ThrowableType: ReferenceType {}

/// You should generally not be interacting with this type directly, but it must be public for codegen.
#[doc(hidden)]
#[warn(clippy::missing_safety_doc)]
pub unsafe trait ReferenceType: JniType + Sized + 'static {}

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

/// Represents a Java `null` value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Null;
