//! XXX: This type came from the original [jni-glue](https://docs.rs/jni-glue/0.0.10/src/jni_glue/jni_type.rs.html),
//! I'm not sure of its possible funcationality in the future, but it's currently preserved.
//!
//! Side note: While primitive array type signatures like c"[I" can be passed to the JNI `FindClass`, a primitive "class"
//! like `int.class` cannot be obtained by passing c"I" to `FindClass`. Primitive "classes" might be obtained from
//! [java.lang.reflect.Method](https://docs.oracle.com/javase/8/docs/api/java/lang/reflect/Method.html#getParameterTypes).

use std::borrow::Cow;
use std::ffi::{CStr, CString};

use jni_sys::*;

use crate::ReferenceType;

#[doc(hidden)]
pub unsafe trait JniType {
    fn jni_type_name() -> Cow<'static, CStr>;
}

unsafe impl JniType for () {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"V")
    }
}
unsafe impl JniType for bool {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"Z")
    }
}
unsafe impl JniType for jbyte {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"B")
    }
}
unsafe impl JniType for jchar {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"C")
    }
}
unsafe impl JniType for jshort {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"S")
    }
}
unsafe impl JniType for jint {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"I")
    }
}
unsafe impl JniType for jlong {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"J")
    }
}
unsafe impl JniType for jfloat {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"F")
    }
}
unsafe impl JniType for jdouble {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"D")
    }
}
unsafe impl JniType for &CStr {
    fn jni_type_name() -> Cow<'static, CStr> {
        Cow::Borrowed(c"Ljava/lang/String;")
    }
}

unsafe impl<T: ReferenceType> JniType for T {
    fn jni_type_name() -> Cow<'static, CStr> {
        let type_name = Self::jni_reference_type_name();
        if type_name.to_bytes()[0] != b'[' {
            Cow::Owned(CString::new(format!("L{};", type_name.to_string_lossy())).unwrap())
        } else {
            type_name
        }
    }
}
