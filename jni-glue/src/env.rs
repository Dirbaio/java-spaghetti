use std::marker::PhantomData;
use std::os::raw::c_char;
use std::ptr::null_mut;

use jni_sys::*;

use crate::{AsJValue, Local, ReferenceType, ThrowableType, VM};

/// FFI:  Use **Env** instead of \*const JNIEnv.  This represents a per-thread Java exection environment.
///
/// A "safe" alternative to jni_sys::JNIEnv raw pointers, with the following caveats:
///
/// 1)  A null env will result in **undefined behavior**.  Java should not be invoking your native functions with a null
///     *mut JNIEnv, however, so I don't believe this is a problem in practice unless you've bindgened the C header
///     definitions elsewhere, calling them (requiring `unsafe`), and passing null pointers (generally UB for JNI
///     functions anyways, so can be seen as a caller soundness issue.)
///
/// 2)  Allowing the underlying JNIEnv to be modified is **undefined behavior**.  I don't believe the JNI libraries
///     modify the JNIEnv, so as long as you're not accepting a *mut JNIEnv elsewhere, using unsafe to dereference it,
///     and mucking with the methods on it yourself, I believe this "should" be fine.
///
/// # Example
///
/// ### MainActivity.java
///
/// ```java
/// package com.maulingmonkey.example;
///
/// public class MainActivity extends androidx.appcompat.app.AppCompatActivity {
///     @Override
///     public native boolean dispatchKeyEvent(android.view.KeyEvent keyEvent);
///
///     // ...
/// }
/// ```
///
/// ### main_activity.rs
///
/// ```rust
/// use jni_sys::{jboolean, jobject, JNI_TRUE}; // TODO: Replace with safer equivalent
/// use jni_glue::Env;
///
/// #[no_mangle] pub extern "system"
/// fn Java_com_maulingmonkey_example_MainActivity_dispatchKeyEvent<'env>(
///     _env:       Env<'env>,
///     _this:      jobject, // TODO: Replace with safer equivalent
///     _key_event: jobject  // TODO: Replace with safer equivalent
/// ) -> jboolean {
///     // ...
///     JNI_TRUE
/// }
/// ```
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Env<'env> {
    env: *mut JNIEnv,
    pd: PhantomData<&'env mut JNIEnv>,
}

impl<'env> Env<'env> {
    pub unsafe fn from_raw(ptr: *mut JNIEnv) -> Self {
        Self {
            env: ptr,
            pd: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *mut JNIEnv {
        self.env
    }

    pub fn vm(&self) -> VM {
        let jni_env = self.as_raw();
        let mut vm = null_mut();
        let err = unsafe { ((**jni_env).v1_2.GetJavaVM)(jni_env, &mut vm) };
        assert_eq!(err, JNI_OK);
        assert_ne!(vm, null_mut());
        unsafe { VM::from_raw(vm) }
    }

    // String methods

    pub unsafe fn new_string(self, chars: *const jchar, len: jsize) -> jstring {
        ((**self.env).v1_2.NewString)(self.env, chars as *const _, len)
    }

    pub unsafe fn get_string_length(self, string: jstring) -> jsize {
        ((**self.env).v1_2.GetStringLength)(self.env, string)
    }

    pub unsafe fn get_string_chars(self, string: jstring) -> *const jchar {
        ((**self.env).v1_2.GetStringChars)(self.env, string, null_mut()) as *const _
    }

    pub unsafe fn release_string_chars(self, string: jstring, chars: *const jchar) {
        ((**self.env).v1_2.ReleaseStringChars)(self.env, string, chars as *const _)
    }

    // Query Methods

    pub unsafe fn require_class(self, class: &str) -> jclass {
        debug_assert!(class.ends_with('\0'));
        let class = ((**self.env).v1_2.FindClass)(self.env, class.as_ptr() as *const c_char);
        assert!(!class.is_null());
        class
    }

    pub unsafe fn require_method(self, class: jclass, method: &str, descriptor: &str) -> jmethodID {
        debug_assert!(method.ends_with('\0'));
        debug_assert!(descriptor.ends_with('\0'));

        let method = ((**self.env).v1_2.GetMethodID)(
            self.env,
            class,
            method.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!method.is_null());
        method
    }

    pub unsafe fn require_static_method(self, class: jclass, method: &str, descriptor: &str) -> jmethodID {
        debug_assert!(method.ends_with('\0'));
        debug_assert!(descriptor.ends_with('\0'));

        let method = ((**self.env).v1_2.GetStaticMethodID)(
            self.env,
            class,
            method.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!method.is_null());
        method
    }

    pub unsafe fn require_field(self, class: jclass, field: &str, descriptor: &str) -> jfieldID {
        debug_assert!(field.ends_with('\0'));
        debug_assert!(field.ends_with('\0'));

        let field = ((**self.env).v1_2.GetFieldID)(
            self.env,
            class,
            field.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!field.is_null());
        field
    }

    pub unsafe fn require_static_field(self, class: jclass, field: &str, descriptor: &str) -> jfieldID {
        debug_assert!(field.ends_with('\0'));
        debug_assert!(field.ends_with('\0'));

        let field = ((**self.env).v1_2.GetStaticFieldID)(
            self.env,
            class,
            field.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!field.is_null());
        field
    }

    // Multi-Query Methods

    pub unsafe fn require_class_method(self, class: &str, method: &str, descriptor: &str) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_method(class, method, descriptor))
    }

    pub unsafe fn require_class_static_method(
        self,
        class: &str,
        method: &str,
        descriptor: &str,
    ) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_static_method(class, method, descriptor))
    }

    pub unsafe fn require_class_field(self, class: &str, method: &str, descriptor: &str) -> (jclass, jfieldID) {
        let class = self.require_class(class);
        (class, self.require_field(class, method, descriptor))
    }

    pub unsafe fn require_class_static_field(self, class: &str, method: &str, descriptor: &str) -> (jclass, jfieldID) {
        let class = self.require_class(class);
        (class, self.require_static_field(class, method, descriptor))
    }

    // Constructor Methods

    pub unsafe fn new_object_a<R: ReferenceType, E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Local<'env, R>, Local<'env, E>> {
        let result = ((**self.env).v1_2.NewObjectA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            assert!(!result.is_null());
            Ok(Local::from_raw(self, result))
        }
    }

    // Instance Methods

    pub unsafe fn call_object_method_a<R: ReferenceType, E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'env, R>>, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallObjectMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(Local::from_raw(self, result)))
        }
    }

    pub unsafe fn call_boolean_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<bool, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallBooleanMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result != JNI_FALSE)
        }
    }

    pub unsafe fn call_byte_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallByteMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_char_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallCharMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_short_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallShortMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_int_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallIntMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_long_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallLongMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_float_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallFloatMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_double_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallDoubleMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_void_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        ((**self.env).v1_2.CallVoidMethodA)(self.env, this, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(())
        }
    }

    // Static Methods

    pub unsafe fn call_static_object_method_a<R: ReferenceType, E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'env, R>>, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticObjectMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(Local::from_raw(self, result)))
        }
    }

    pub unsafe fn call_static_boolean_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<bool, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticBooleanMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result != JNI_FALSE)
        }
    }

    pub unsafe fn call_static_byte_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticByteMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_char_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticCharMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_short_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticShortMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_int_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticIntMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_long_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticLongMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_float_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticFloatMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_double_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticDoubleMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_void_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        ((**self.env).v1_2.CallStaticVoidMethodA)(self.env, class, method, args);
        let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            Err(Local::from_raw(self, exception))
        } else {
            Ok(())
        }
    }

    // Instance Fields

    pub unsafe fn get_object_field<R: ReferenceType>(self, this: jobject, field: jfieldID) -> Option<Local<'env, R>> {
        let result = ((**self.env).v1_2.GetObjectField)(self.env, this, field);
        if result.is_null() {
            None
        } else {
            Some(Local::from_raw(self, result))
        }
    }

    pub unsafe fn get_boolean_field(self, this: jobject, field: jfieldID) -> bool {
        let result = ((**self.env).v1_2.GetBooleanField)(self.env, this, field);
        result != JNI_FALSE
    }

    pub unsafe fn get_byte_field(self, this: jobject, field: jfieldID) -> jbyte {
        ((**self.env).v1_2.GetByteField)(self.env, this, field)
    }

    pub unsafe fn get_char_field(self, this: jobject, field: jfieldID) -> jchar {
        ((**self.env).v1_2.GetCharField)(self.env, this, field)
    }

    pub unsafe fn get_short_field(self, this: jobject, field: jfieldID) -> jshort {
        ((**self.env).v1_2.GetShortField)(self.env, this, field)
    }

    pub unsafe fn get_int_field(self, this: jobject, field: jfieldID) -> jint {
        ((**self.env).v1_2.GetIntField)(self.env, this, field)
    }

    pub unsafe fn get_long_field(self, this: jobject, field: jfieldID) -> jlong {
        ((**self.env).v1_2.GetLongField)(self.env, this, field)
    }

    pub unsafe fn get_float_field(self, this: jobject, field: jfieldID) -> jfloat {
        ((**self.env).v1_2.GetFloatField)(self.env, this, field)
    }

    pub unsafe fn get_double_field(self, this: jobject, field: jfieldID) -> jdouble {
        ((**self.env).v1_2.GetDoubleField)(self.env, this, field)
    }

    pub unsafe fn set_object_field<'obj, R: 'obj + ReferenceType>(
        self,
        this: jobject,
        field: jfieldID,
        value: impl Into<Option<&'obj R>>,
    ) {
        let value = value.into().map(|v| AsJValue::as_jvalue(v).l).unwrap_or(null_mut());
        ((**self.env).v1_2.SetObjectField)(self.env, this, field, value);
    }

    pub unsafe fn set_boolean_field(self, this: jobject, field: jfieldID, value: bool) {
        ((**self.env).v1_2.SetBooleanField)(self.env, this, field, if value { JNI_TRUE } else { JNI_FALSE });
    }

    pub unsafe fn set_byte_field(self, this: jobject, field: jfieldID, value: jbyte) {
        ((**self.env).v1_2.SetByteField)(self.env, this, field, value);
    }

    pub unsafe fn set_char_field(self, this: jobject, field: jfieldID, value: jchar) {
        ((**self.env).v1_2.SetCharField)(self.env, this, field, value);
    }

    pub unsafe fn set_short_field(self, this: jobject, field: jfieldID, value: jshort) {
        ((**self.env).v1_2.SetShortField)(self.env, this, field, value);
    }

    pub unsafe fn set_int_field(self, this: jobject, field: jfieldID, value: jint) {
        ((**self.env).v1_2.SetIntField)(self.env, this, field, value);
    }

    pub unsafe fn set_long_field(self, this: jobject, field: jfieldID, value: jlong) {
        ((**self.env).v1_2.SetLongField)(self.env, this, field, value);
    }

    pub unsafe fn set_float_field(self, this: jobject, field: jfieldID, value: jfloat) {
        ((**self.env).v1_2.SetFloatField)(self.env, this, field, value);
    }

    pub unsafe fn set_double_field(self, this: jobject, field: jfieldID, value: jdouble) {
        ((**self.env).v1_2.SetDoubleField)(self.env, this, field, value);
    }

    // Static Fields

    pub unsafe fn get_static_object_field<R: ReferenceType>(
        self,
        class: jclass,
        field: jfieldID,
    ) -> Option<Local<'env, R>> {
        let result = ((**self.env).v1_2.GetStaticObjectField)(self.env, class, field);
        if result.is_null() {
            None
        } else {
            Some(Local::from_raw(self, result))
        }
    }

    pub unsafe fn get_static_boolean_field(self, class: jclass, field: jfieldID) -> bool {
        let result = ((**self.env).v1_2.GetStaticBooleanField)(self.env, class, field);
        result != JNI_FALSE
    }

    pub unsafe fn get_static_byte_field(self, class: jclass, field: jfieldID) -> jbyte {
        ((**self.env).v1_2.GetStaticByteField)(self.env, class, field)
    }

    pub unsafe fn get_static_char_field(self, class: jclass, field: jfieldID) -> jchar {
        ((**self.env).v1_2.GetStaticCharField)(self.env, class, field)
    }

    pub unsafe fn get_static_short_field(self, class: jclass, field: jfieldID) -> jshort {
        ((**self.env).v1_2.GetStaticShortField)(self.env, class, field)
    }

    pub unsafe fn get_static_int_field(self, class: jclass, field: jfieldID) -> jint {
        ((**self.env).v1_2.GetStaticIntField)(self.env, class, field)
    }

    pub unsafe fn get_static_long_field(self, class: jclass, field: jfieldID) -> jlong {
        ((**self.env).v1_2.GetStaticLongField)(self.env, class, field)
    }

    pub unsafe fn get_static_float_field(self, class: jclass, field: jfieldID) -> jfloat {
        ((**self.env).v1_2.GetStaticFloatField)(self.env, class, field)
    }

    pub unsafe fn get_static_double_field(self, class: jclass, field: jfieldID) -> jdouble {
        ((**self.env).v1_2.GetStaticDoubleField)(self.env, class, field)
    }

    pub unsafe fn set_static_object_field<'obj, R: 'obj + ReferenceType>(
        self,
        class: jclass,
        field: jfieldID,
        value: impl Into<Option<&'obj R>>,
    ) {
        let value = value.into().map(|v| AsJValue::as_jvalue(v).l).unwrap_or(null_mut());
        ((**self.env).v1_2.SetStaticObjectField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_boolean_field(self, class: jclass, field: jfieldID, value: bool) {
        ((**self.env).v1_2.SetStaticBooleanField)(self.env, class, field, if value { JNI_TRUE } else { JNI_FALSE });
    }

    pub unsafe fn set_static_byte_field(self, class: jclass, field: jfieldID, value: jbyte) {
        ((**self.env).v1_2.SetStaticByteField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_char_field(self, class: jclass, field: jfieldID, value: jchar) {
        ((**self.env).v1_2.SetStaticCharField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_short_field(self, class: jclass, field: jfieldID, value: jshort) {
        ((**self.env).v1_2.SetStaticShortField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_int_field(self, class: jclass, field: jfieldID, value: jint) {
        ((**self.env).v1_2.SetStaticIntField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_long_field(self, class: jclass, field: jfieldID, value: jlong) {
        ((**self.env).v1_2.SetStaticLongField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_float_field(self, class: jclass, field: jfieldID, value: jfloat) {
        ((**self.env).v1_2.SetStaticFloatField)(self.env, class, field, value);
    }

    pub unsafe fn set_static_double_field(self, class: jclass, field: jfieldID, value: jdouble) {
        ((**self.env).v1_2.SetStaticDoubleField)(self.env, class, field, value);
    }
}
