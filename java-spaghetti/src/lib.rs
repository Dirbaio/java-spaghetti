//! Common glue code between Rust and JNI, used in autogenerated `java-spaghetti` glue code.
//!
//! See also the [Android JNI tips](https://developer.android.com/training/articles/perf-jni) documentation as well as the
//! [Java Native Interface Specification](https://docs.oracle.com/javase/7/docs/technotes/guides/jni/spec/jniTOC.html).

use std::fmt;

/// public jni-sys reexport.
pub use ::jni_sys as sys;
/// Re-export std such that we have a consistent name for them in autogenerated glue code wherever we go.
#[doc(hidden)]
pub use ::std;

mod refs {

    mod arg;
    mod global;
    mod local;
    mod ref_;

    pub use arg::*;
    pub use global::*;
    pub use local::*;
    pub use ref_::*;
}

mod array;
mod as_jvalue;
mod env;
mod jni_type;
mod macros;
mod string_chars;
mod vm;

pub use array::*;
pub use as_jvalue::*;
pub use env::*;
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
/// This is hideously unsafe to implement:
///
/// 1) You assert the type is a #[repr(transparent)] wrapper around ObjectAndEnv.
/// 2) You assert the type cannot exist with a dangling object or env.
///     2.1) Do not implement Copy or Clone.
///     2.2) Do not allow value access.
///     2.3) Do not allow &mut T access.
///     2.4) Only allow &T access, which cannot be moved from.
#[doc(hidden)]
pub unsafe trait ReferenceType: AsJValue + JniType + 'static {}

#[repr(C)] // Given how frequently we transmute to/from this, we'd better keep a consistent layout.
#[doc(hidden)] // You should generally not be interacting with this type directly, but it must be public for codegen.
#[derive(Copy, Clone)]
pub struct ObjectAndEnv {
    pub object: jni_sys::jobject,
    pub env: *mut jni_sys::JNIEnv,
}
