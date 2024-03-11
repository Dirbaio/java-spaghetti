use std::ffi::c_void;
use std::os::raw::c_char;
use std::ptr::null_mut;

use jni_sys::*;

use crate::{jchar, AsJValue, AsValidJObjectAndEnv, Local, ThrowableType, VM};

/// FFI:  Use **&Env** instead of \*const JNIEnv.  This represents a per-thread Java exection environment.
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
///     _env:       &Env,
///     _this:      jobject, // TODO: Replace with safer equivalent
///     _key_event: jobject  // TODO: Replace with safer equivalent
/// ) -> jboolean {
///     // ...
///     JNI_TRUE
/// }
/// ```
#[repr(transparent)]
pub struct Env(JNIEnv);

impl Env {
    pub unsafe fn from_ptr<'env>(ptr: *const JNIEnv) -> &'env Env {
        &*(ptr as *const Env)
    }

    pub fn as_jni_env(&self) -> *mut JNIEnv {
        &self.0 as *const _ as *mut _
    }
    pub(crate) unsafe fn from_jni_local(env: &JNIEnv) -> &Env {
        &*(env as *const JNIEnv as *const Env)
    }
    pub(crate) unsafe fn from_jni_void_ref(ptr: &*mut c_void) -> &Env {
        Self::from_jni_local(&*(*ptr as *const c_void as *const JNIEnv))
    }

    pub(crate) fn get_vm(&self) -> VM {
        let jni_env = self.as_jni_env();
        let mut vm = null_mut();
        let err = unsafe { ((**jni_env).v1_2.GetJavaVM)(jni_env, &mut vm) };
        assert_eq!(err, JNI_OK);
        assert_ne!(vm, null_mut());
        unsafe { VM::from_raw(vm) }
    }

    // String methods

    pub unsafe fn new_string(&self, chars: *const jchar, len: jsize) -> jstring {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.NewString)(env, chars as *const _, len)
    }

    pub unsafe fn get_string_length(&self, string: jstring) -> jsize {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStringLength)(env, string)
    }

    pub unsafe fn get_string_chars(&self, string: jstring) -> *const jchar {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStringChars)(env, string, null_mut()) as *const _
    }

    pub unsafe fn release_string_chars(&self, string: jstring, chars: *const jchar) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.ReleaseStringChars)(env, string, chars as *const _)
    }

    // Query Methods

    pub unsafe fn require_class(&self, class: &str) -> jclass {
        debug_assert!(class.ends_with('\0'));
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let class = ((**env).v1_2.FindClass)(env, class.as_ptr() as *const c_char);
        assert!(!class.is_null());
        class
    }

    pub unsafe fn require_method(&self, class: jclass, method: &str, descriptor: &str) -> jmethodID {
        debug_assert!(method.ends_with('\0'));
        debug_assert!(descriptor.ends_with('\0'));

        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let method = ((**env).v1_2.GetMethodID)(
            env,
            class,
            method.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!method.is_null());
        method
    }

    pub unsafe fn require_static_method(&self, class: jclass, method: &str, descriptor: &str) -> jmethodID {
        debug_assert!(method.ends_with('\0'));
        debug_assert!(descriptor.ends_with('\0'));

        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let method = ((**env).v1_2.GetStaticMethodID)(
            env,
            class,
            method.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!method.is_null());
        method
    }

    pub unsafe fn require_field(&self, class: jclass, field: &str, descriptor: &str) -> jfieldID {
        debug_assert!(field.ends_with('\0'));
        debug_assert!(field.ends_with('\0'));

        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let field = ((**env).v1_2.GetFieldID)(
            env,
            class,
            field.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!field.is_null());
        field
    }

    pub unsafe fn require_static_field(&self, class: jclass, field: &str, descriptor: &str) -> jfieldID {
        debug_assert!(field.ends_with('\0'));
        debug_assert!(field.ends_with('\0'));

        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let field = ((**env).v1_2.GetStaticFieldID)(
            env,
            class,
            field.as_ptr() as *const c_char,
            descriptor.as_ptr() as *const c_char,
        );
        assert!(!field.is_null());
        field
    }

    // Multi-Query Methods

    pub unsafe fn require_class_method(&self, class: &str, method: &str, descriptor: &str) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_method(class, method, descriptor))
    }

    pub unsafe fn require_class_static_method(
        &self,
        class: &str,
        method: &str,
        descriptor: &str,
    ) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_static_method(class, method, descriptor))
    }

    pub unsafe fn require_class_field(&self, class: &str, method: &str, descriptor: &str) -> (jclass, jfieldID) {
        let class = self.require_class(class);
        (class, self.require_field(class, method, descriptor))
    }

    pub unsafe fn require_class_static_field(&self, class: &str, method: &str, descriptor: &str) -> (jclass, jfieldID) {
        let class = self.require_class(class);
        (class, self.require_static_field(class, method, descriptor))
    }

    // Constructor Methods

    pub unsafe fn new_object_a<R: AsValidJObjectAndEnv, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Local<'_, R>, Local<'_, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.NewObjectA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            assert!(!result.is_null());
            Ok(Local::from_env_object(env, result))
        }
    }

    // Instance Methods

    pub unsafe fn call_object_method_a<R: AsValidJObjectAndEnv, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'_, R>>, Local<'_, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallObjectMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(Local::from_env_object(env, result)))
        }
    }

    pub unsafe fn call_boolean_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<bool, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallBooleanMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result != JNI_FALSE)
        }
    }

    pub unsafe fn call_byte_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallByteMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_char_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallCharMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(jchar(result))
        }
    }

    pub unsafe fn call_short_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallShortMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_int_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallIntMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_long_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallLongMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_float_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallFloatMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_double_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallDoubleMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_void_method_a<'env, E: ThrowableType>(
        &self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.CallVoidMethodA)(env, this, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(())
        }
    }

    // Static Methods

    pub unsafe fn call_static_object_method_a<R: AsValidJObjectAndEnv, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'_, R>>, Local<'_, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticObjectMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(Local::from_env_object(env, result)))
        }
    }

    pub unsafe fn call_static_boolean_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<bool, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticBooleanMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result != JNI_FALSE)
        }
    }

    pub unsafe fn call_static_byte_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticByteMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_char_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticCharMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(jchar(result))
        }
    }

    pub unsafe fn call_static_short_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticShortMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_int_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticIntMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_long_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticLongMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_float_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticFloatMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_double_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.CallStaticDoubleMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(result)
        }
    }

    pub unsafe fn call_static_void_method_a<'env, E: ThrowableType>(
        &self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.CallStaticVoidMethodA)(env, class, method, args);
        let exception = ((**env).v1_2.ExceptionOccurred)(env);
        if !exception.is_null() {
            ((**env).v1_2.ExceptionClear)(env);
            Err(Local::from_env_object(env, exception))
        } else {
            Ok(())
        }
    }

    // Instance Fields

    pub unsafe fn get_object_field<R: AsValidJObjectAndEnv>(
        &self,
        this: jobject,
        field: jfieldID,
    ) -> Option<Local<'_, R>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetObjectField)(env, this, field);
        if result.is_null() {
            None
        } else {
            Some(Local::from_env_object(env, result))
        }
    }

    pub unsafe fn get_boolean_field(&self, this: jobject, field: jfieldID) -> bool {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetBooleanField)(env, this, field);
        result != JNI_FALSE
    }

    pub unsafe fn get_byte_field(&self, this: jobject, field: jfieldID) -> jbyte {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetByteField)(env, this, field)
    }

    pub unsafe fn get_char_field(&self, this: jobject, field: jfieldID) -> jchar {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetCharField)(env, this, field);
        jchar(result)
    }

    pub unsafe fn get_short_field(&self, this: jobject, field: jfieldID) -> jshort {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetShortField)(env, this, field)
    }

    pub unsafe fn get_int_field(&self, this: jobject, field: jfieldID) -> jint {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetIntField)(env, this, field)
    }

    pub unsafe fn get_long_field(&self, this: jobject, field: jfieldID) -> jlong {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetLongField)(env, this, field)
    }

    pub unsafe fn get_float_field(&self, this: jobject, field: jfieldID) -> jfloat {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetFloatField)(env, this, field)
    }

    pub unsafe fn get_double_field(&self, this: jobject, field: jfieldID) -> jdouble {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetDoubleField)(env, this, field)
    }

    pub unsafe fn set_object_field<'env, 'obj, R: 'obj + AsValidJObjectAndEnv>(
        &'env self,
        this: jobject,
        field: jfieldID,
        value: impl Into<Option<&'obj R>>,
    ) {
        let value = value.into().map(|v| AsJValue::as_jvalue(v).l).unwrap_or(null_mut());
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetObjectField)(env, this, field, value);
    }

    pub unsafe fn set_boolean_field(&self, this: jobject, field: jfieldID, value: bool) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetBooleanField)(env, this, field, if value { JNI_TRUE } else { JNI_FALSE });
    }

    pub unsafe fn set_byte_field(&self, this: jobject, field: jfieldID, value: jbyte) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetByteField)(env, this, field, value);
    }

    pub unsafe fn set_char_field(&self, this: jobject, field: jfieldID, value: jchar) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetCharField)(env, this, field, value.0);
    }

    pub unsafe fn set_short_field(&self, this: jobject, field: jfieldID, value: jshort) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetShortField)(env, this, field, value);
    }

    pub unsafe fn set_int_field(&self, this: jobject, field: jfieldID, value: jint) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetIntField)(env, this, field, value);
    }

    pub unsafe fn set_long_field(&self, this: jobject, field: jfieldID, value: jlong) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetLongField)(env, this, field, value);
    }

    pub unsafe fn set_float_field(&self, this: jobject, field: jfieldID, value: jfloat) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetFloatField)(env, this, field, value);
    }

    pub unsafe fn set_double_field(&self, this: jobject, field: jfieldID, value: jdouble) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetDoubleField)(env, this, field, value);
    }

    // Static Fields

    pub unsafe fn get_static_object_field<R: AsValidJObjectAndEnv>(
        &self,
        class: jclass,
        field: jfieldID,
    ) -> Option<Local<'_, R>> {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetStaticObjectField)(env, class, field);
        if result.is_null() {
            None
        } else {
            Some(Local::from_env_object(env, result))
        }
    }

    pub unsafe fn get_static_boolean_field(&self, class: jclass, field: jfieldID) -> bool {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetStaticBooleanField)(env, class, field);
        result != JNI_FALSE
    }

    pub unsafe fn get_static_byte_field(&self, class: jclass, field: jfieldID) -> jbyte {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticByteField)(env, class, field)
    }

    pub unsafe fn get_static_char_field(&self, class: jclass, field: jfieldID) -> jchar {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        let result = ((**env).v1_2.GetStaticCharField)(env, class, field);
        jchar(result)
    }

    pub unsafe fn get_static_short_field(&self, class: jclass, field: jfieldID) -> jshort {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticShortField)(env, class, field)
    }

    pub unsafe fn get_static_int_field(&self, class: jclass, field: jfieldID) -> jint {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticIntField)(env, class, field)
    }

    pub unsafe fn get_static_long_field(&self, class: jclass, field: jfieldID) -> jlong {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticLongField)(env, class, field)
    }

    pub unsafe fn get_static_float_field(&self, class: jclass, field: jfieldID) -> jfloat {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticFloatField)(env, class, field)
    }

    pub unsafe fn get_static_double_field(&self, class: jclass, field: jfieldID) -> jdouble {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.GetStaticDoubleField)(env, class, field)
    }

    pub unsafe fn set_static_object_field<'env, 'obj, R: 'obj + AsValidJObjectAndEnv>(
        &'env self,
        class: jclass,
        field: jfieldID,
        value: impl Into<Option<&'obj R>>,
    ) {
        let value = value.into().map(|v| AsJValue::as_jvalue(v).l).unwrap_or(null_mut());
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticObjectField)(env, class, field, value);
    }

    pub unsafe fn set_static_boolean_field(&self, class: jclass, field: jfieldID, value: bool) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticBooleanField)(env, class, field, if value { JNI_TRUE } else { JNI_FALSE });
    }

    pub unsafe fn set_static_byte_field(&self, class: jclass, field: jfieldID, value: jbyte) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticByteField)(env, class, field, value);
    }

    pub unsafe fn set_static_char_field(&self, class: jclass, field: jfieldID, value: jchar) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticCharField)(env, class, field, value.0);
    }

    pub unsafe fn set_static_short_field(&self, class: jclass, field: jfieldID, value: jshort) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticShortField)(env, class, field, value);
    }

    pub unsafe fn set_static_int_field(&self, class: jclass, field: jfieldID, value: jint) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticIntField)(env, class, field, value);
    }

    pub unsafe fn set_static_long_field(&self, class: jclass, field: jfieldID, value: jlong) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticLongField)(env, class, field, value);
    }

    pub unsafe fn set_static_float_field(&self, class: jclass, field: jfieldID, value: jfloat) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticFloatField)(env, class, field, value);
    }

    pub unsafe fn set_static_double_field(&self, class: jclass, field: jfieldID, value: jdouble) {
        let env = &self.0 as *const JNIEnv as *mut JNIEnv;
        ((**env).v1_2.SetStaticDoubleField)(env, class, field, value);
    }
}
