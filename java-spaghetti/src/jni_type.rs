//! XXX: This type came from the original [jni-glue](https://docs.rs/jni-glue/0.0.10/src/jni_glue/jni_type.rs.html),
//! I'm not sure of its possible funcationality in the future, but it's currently preserved.
//!
//! NOTE: While primitive array type signatures like "[I\0" can be passed to the JNI `FindClass`, a primitive "class"
//! like `int.class` cannot be obtained by passing "I\0" to `FindClass`. Primitive "classes" may be obtained from
//! [java.lang.reflect.Method](https://docs.oracle.com/javase/8/docs/api/java/lang/reflect/Method.html#getParameterTypes).

use std::borrow::Cow;

use jni_sys::*;

use crate::ReferenceType;

#[doc(hidden)]
pub unsafe trait JniType {
    fn jni_type_name() -> Cow<'static, str>;
}

unsafe impl JniType for () {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("V\0")
    }
}
unsafe impl JniType for bool {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("Z\0")
    }
}
unsafe impl JniType for jbyte {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("B\0")
    }
}
unsafe impl JniType for jchar {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("C\0")
    }
}
unsafe impl JniType for jshort {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("S\0")
    }
}
unsafe impl JniType for jint {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("I\0")
    }
}
unsafe impl JniType for jlong {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("J\0")
    }
}
unsafe impl JniType for jfloat {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("F\0")
    }
}
unsafe impl JniType for jdouble {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("D\0")
    }
}
unsafe impl JniType for &str {
    fn jni_type_name() -> Cow<'static, str> {
        Cow::Borrowed("Ljava/lang/String;\0")
    }
}

unsafe impl<T: ReferenceType> JniType for T {
    fn jni_type_name() -> Cow<'static, str> {
        Self::jni_reference_type_name()
    }
}
