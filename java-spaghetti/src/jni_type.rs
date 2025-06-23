use std::ffi::CStr;

use jni_sys::*;

/// JNI bindings rely on this type being accurate.
///
/// # Safety
///
/// **unsafe**: Passing the wrong type can cause unsoundness, since the code that interacts with JNI blindly trusts it's correct.
///
/// Why the awkward callback style instead of returning `&'static CStr`?  Arrays of arrays may need to dynamically
/// construct their type strings, which would need to leak.  Worse, we can't easily intern those strings via
/// lazy_static without running into:
///
/// ```text
/// error[E0401]: can't use generic parameters from outer function
/// ```
pub unsafe trait JniType {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R;
}

unsafe impl JniType for () {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"V")
    }
}
unsafe impl JniType for bool {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"Z")
    }
}
unsafe impl JniType for jbyte {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"B")
    }
}
unsafe impl JniType for jchar {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"C")
    }
}
unsafe impl JniType for jshort {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"S")
    }
}
unsafe impl JniType for jint {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"I")
    }
}
unsafe impl JniType for jlong {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"J")
    }
}
unsafe impl JniType for jfloat {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"F")
    }
}
unsafe impl JniType for jdouble {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"D")
    }
}
unsafe impl JniType for &CStr {
    fn static_with_jni_type<R>(callback: impl FnOnce(&CStr) -> R) -> R {
        callback(c"Ljava/lang/String;")
    }
}
