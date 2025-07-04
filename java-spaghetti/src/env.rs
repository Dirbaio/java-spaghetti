use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr::{self, null_mut};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use jni_sys::*;

use crate::{AsArg, Local, Ref, ReferenceType, StringChars, ThrowableType, VM};

/// FFI:  Use **Env** instead of `*const JNIEnv`.  This represents a per-thread Java exection environment.
///
/// A "safe" alternative to `jni_sys::JNIEnv` raw pointers, with the following caveats:
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
/// Most methods of `Env` are supposed to be used by generated bindings.
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
/// ```ignore
/// use java_spaghetti::{Env, Arg};
/// use java_spaghetti::sys::{jboolean, JNI_TRUE};
/// use bindings::java::lang::Object;
/// use bindings::android::view::KeyEvent;
///
/// mod bindings; // Generated by `java-spaghetti-gen`
///
/// #[unsafe(no_mangle)] pub extern "system"
/// fn Java_com_maulingmonkey_example_MainActivity_dispatchKeyEvent<'env>(
///     env:       Env<'env>,
///     _this:     Arg<Object>,
///     key_event: Arg<KeyEvent>,
/// ) -> jboolean {
///     let key_event = unsafe { key_event.into_ref(env) };
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

static CLASS_LOADER: AtomicPtr<_jobject> = AtomicPtr::new(null_mut());

#[allow(clippy::missing_safety_doc)]
#[allow(unsafe_op_in_unsafe_fn)]
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
        let result = ((**self.env).v1_2.NewString)(self.env, chars as *const _, len);
        assert!(!result.is_null());
        result
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

    /// Set a custom class loader to use instead of JNI `FindClass` calls.
    ///
    /// When calling Java methods, `java-spaghetti` may need to resolve class names (as strings)
    /// into `jclass` pointers. The JNI API provides `FindClass` to do it. However, it is
    /// hardcoded to use the class loader for the class that called the currently-running native method.
    ///
    /// This works fine most of the time, except:
    ///
    /// - On a thread created by native code (such as with `std::thread::spawn()`), there is no
    ///   "class that called a native method" in the call stack, since the execution already started
    ///   in native code. In this case, `FindClass` falls back to the system class loader.
    /// - On Android, the system class loader can't find classes for your application, it can only find
    ///   classes from the Android frameworks.
    ///
    /// `set_class_loader` allows you to set a `ClassLoader` instance that `java-spaghetti` will use to
    /// resolve class names, by calling the `loadClass` method, instead of doing JNI `FindClass` calls.
    ///
    /// Calling this with a null `classloader` reverts back to using JNI `FindClass`.
    ///
    /// # Safety
    ///
    /// - `classloader` must be a global reference to a `java.lang.ClassLoader` instance.
    /// - The library does not take ownership of the global reference. I.e. it will not delete it if you
    ///   call `set_class_loader` with another class loader, or with null.
    pub unsafe fn set_class_loader(classloader: jobject) {
        CLASS_LOADER.store(classloader, Ordering::Relaxed);
    }

    /// Checks if an exception has occurred; if occurred, it clears the exception to make the next
    /// JNI call possible, then it returns the exception as an `Err`.
    ///
    /// XXX: Make this method public after making sure that it has a proper name.
    /// Note that there is `ExceptionCheck` in JNI functions, which does not create a
    /// local reference to the exception object.
    pub(crate) fn exception_check<E: ThrowableType>(self) -> Result<(), Local<'env, E>> {
        unsafe {
            let exception = ((**self.env).v1_2.ExceptionOccurred)(self.env);
            if exception.is_null() {
                Ok(())
            } else {
                ((**self.env).v1_2.ExceptionClear)(self.env);
                Err(Local::from_raw(self, exception))
            }
        }
    }

    unsafe fn exception_to_string(self, exception: jobject) -> String {
        static METHOD_GET_MESSAGE: OnceLock<usize> = OnceLock::new();
        let throwable_get_message = *METHOD_GET_MESSAGE.get_or_init(|| {
            // use JNI FindClass to avoid infinte recursion.
            let throwable_class = self.require_class_jni(c"java/lang/Throwable");
            let method = self.require_method(throwable_class, c"getMessage", c"()Ljava/lang/String;");
            ((**self.env).v1_2.DeleteLocalRef)(self.env, throwable_class);
            method.addr()
        }) as jmethodID; // it is a global ID

        let message =
            ((**self.env).v1_2.CallObjectMethodA)(self.env, exception, throwable_get_message, ptr::null_mut());
        let e2: *mut _jobject = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !e2.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            panic!("exception happened calling Throwable.getMessage()");
        }

        StringChars::from_env_jstring(self, message).to_string_lossy()
    }

    /// Note: the returned `jclass` is actually a new local reference of the class object.
    pub unsafe fn require_class(self, class: &CStr) -> jclass {
        // First try with JNI FindClass.
        let c = ((**self.env).v1_2.FindClass)(self.env, class.as_ptr());
        let exception: *mut _jobject = ((**self.env).v1_2.ExceptionOccurred)(self.env);
        if !exception.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
        }
        if !c.is_null() {
            return c;
        }

        // If class is not found and we have a classloader set, try that.
        let classloader = CLASS_LOADER.load(Ordering::Relaxed);
        if !classloader.is_null() {
            let chars = class
                .to_str()
                .unwrap()
                .replace('/', ".")
                .encode_utf16()
                .collect::<Vec<_>>();
            let string = unsafe { self.new_string(chars.as_ptr(), chars.len() as jsize) };

            static CL_METHOD: OnceLock<usize> = OnceLock::new();
            let cl_method = *CL_METHOD.get_or_init(|| {
                // We still use JNI FindClass for this, to avoid a chicken-and-egg situation.
                // If the system class loader cannot find java.lang.ClassLoader, things are pretty broken!
                let cl_class = self.require_class_jni(c"java/lang/ClassLoader");
                let cl_method = self.require_method(cl_class, c"loadClass", c"(Ljava/lang/String;)Ljava/lang/Class;");
                ((**self.env).v1_2.DeleteLocalRef)(self.env, cl_class);
                cl_method.addr()
            }) as jmethodID; // it is a global ID

            let args = [jvalue { l: string }];
            let result: *mut _jobject =
                ((**self.env).v1_2.CallObjectMethodA)(self.env, classloader, cl_method, args.as_ptr());
            let exception: *mut _jobject = ((**self.env).v1_2.ExceptionOccurred)(self.env);
            if !exception.is_null() {
                ((**self.env).v1_2.ExceptionClear)(self.env);
                panic!(
                    "exception happened calling loadClass(): {}",
                    self.exception_to_string(exception)
                );
            } else if result.is_null() {
                panic!("loadClass() returned null");
            }

            ((**self.env).v1_2.DeleteLocalRef)(self.env, string);

            return result as jclass;
        }

        // If neither found the class, panic.
        panic!("couldn't load class {class:?}");
    }

    unsafe fn require_class_jni(self, class: &CStr) -> jclass {
        let res = ((**self.env).v1_2.FindClass)(self.env, class.as_ptr());
        if res.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            panic!("could not find class {class:?}");
        }
        res
    }

    // used only for debugging
    unsafe fn get_class_name(self, class: jclass) -> String {
        let classclass = self.require_class_jni(c"java/lang/Class");

        // don't use self.require_method() here to avoid recursion!
        let method = ((**self.env).v1_2.GetMethodID)(
            self.env,
            classclass,
            c"getName".as_ptr(),
            c"()Ljava/lang/String;".as_ptr(),
        );
        if method.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            ((**self.env).v1_2.DeleteLocalRef)(self.env, classclass);
            return "??? (couldn't get class getName method)".to_string();
        }

        let string = ((**self.env).v1_2.CallObjectMethod)(self.env, class, method);
        if string.is_null() {
            return "??? (getName returned null string)".to_string();
        }
        let chars = ((**self.env).v1_2.GetStringUTFChars)(self.env, string, ptr::null_mut());
        if chars.is_null() {
            ((**self.env).v1_2.DeleteLocalRef)(self.env, string);
            ((**self.env).v1_2.DeleteLocalRef)(self.env, classclass);
            return "??? (GetStringUTFChars returned null chars)".to_string();
        }

        let cchars = CStr::from_ptr(chars);
        let res = cchars.to_string_lossy().to_string();

        ((**self.env).v1_2.ReleaseStringUTFChars)(self.env, string, chars);
        ((**self.env).v1_2.DeleteLocalRef)(self.env, string);
        ((**self.env).v1_2.DeleteLocalRef)(self.env, classclass);

        res
    }

    pub unsafe fn require_method(self, class: jclass, method: &CStr, descriptor: &CStr) -> jmethodID {
        let res = ((**self.env).v1_2.GetMethodID)(self.env, class, method.as_ptr(), descriptor.as_ptr());
        if res.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            let class_name = self.get_class_name(class);
            panic!("could not find method {method:?} {descriptor:?} on class {class_name:?}");
        }
        res
    }

    pub unsafe fn require_static_method(self, class: jclass, method: &CStr, descriptor: &CStr) -> jmethodID {
        let res = ((**self.env).v1_2.GetStaticMethodID)(self.env, class, method.as_ptr(), descriptor.as_ptr());
        if res.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            let class_name = self.get_class_name(class);
            panic!("could not find static method {method:?} {descriptor:?} on class {class_name:?}");
        }
        res
    }

    pub unsafe fn require_field(self, class: jclass, field: &CStr, descriptor: &CStr) -> jfieldID {
        let res = ((**self.env).v1_2.GetFieldID)(self.env, class, field.as_ptr(), descriptor.as_ptr());
        if res.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            let class_name = self.get_class_name(class);
            panic!("could not find field {field:?} {descriptor:?} on class {class_name:?}");
        }
        res
    }

    pub unsafe fn require_static_field(self, class: jclass, field: &CStr, descriptor: &CStr) -> jfieldID {
        let res = ((**self.env).v1_2.GetStaticFieldID)(self.env, class, field.as_ptr(), descriptor.as_ptr());
        if res.is_null() {
            ((**self.env).v1_2.ExceptionClear)(self.env);
            let class_name = self.get_class_name(class);
            panic!("could not find static field {field:?} {descriptor:?} on class {class_name:?}");
        }
        res
    }

    // Multi-Query Methods
    // XXX: Remove these unused functions.

    pub unsafe fn require_class_method(self, class: &CStr, method: &CStr, descriptor: &CStr) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_method(class, method, descriptor))
    }

    pub unsafe fn require_class_static_method(
        self,
        class: &CStr,
        method: &CStr,
        descriptor: &CStr,
    ) -> (jclass, jmethodID) {
        let class = self.require_class(class);
        (class, self.require_static_method(class, method, descriptor))
    }

    pub unsafe fn require_class_field(self, class: &CStr, method: &CStr, descriptor: &CStr) -> (jclass, jfieldID) {
        let class = self.require_class(class);
        (class, self.require_field(class, method, descriptor))
    }

    pub unsafe fn require_class_static_field(
        self,
        class: &CStr,
        method: &CStr,
        descriptor: &CStr,
    ) -> (jclass, jfieldID) {
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
        self.exception_check()?;
        assert!(!result.is_null());
        Ok(Local::from_raw(self, result))
    }

    // Instance Methods

    pub unsafe fn call_object_method_a<R: ReferenceType, E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'env, R>>, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallObjectMethodA)(self.env, this, method, args);
        self.exception_check()?;
        if result.is_null() {
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
        self.exception_check()?;
        Ok(result != JNI_FALSE)
    }

    pub unsafe fn call_byte_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallByteMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_char_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallCharMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_short_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallShortMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_int_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallIntMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_long_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallLongMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_float_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallFloatMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_double_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallDoubleMethodA)(self.env, this, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_void_method_a<E: ThrowableType>(
        self,
        this: jobject,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        ((**self.env).v1_2.CallVoidMethodA)(self.env, this, method, args);
        self.exception_check()
    }

    // Static Methods

    pub unsafe fn call_static_object_method_a<R: ReferenceType, E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<Option<Local<'env, R>>, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticObjectMethodA)(self.env, class, method, args);
        self.exception_check()?;
        if result.is_null() {
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
        self.exception_check()?;
        Ok(result != JNI_FALSE)
    }

    pub unsafe fn call_static_byte_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jbyte, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticByteMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_char_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jchar, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticCharMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_short_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jshort, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticShortMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_int_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jint, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticIntMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_long_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jlong, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticLongMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_float_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jfloat, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticFloatMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_double_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<jdouble, Local<'env, E>> {
        let result = ((**self.env).v1_2.CallStaticDoubleMethodA)(self.env, class, method, args);
        self.exception_check()?;
        Ok(result)
    }

    pub unsafe fn call_static_void_method_a<E: ThrowableType>(
        self,
        class: jclass,
        method: jmethodID,
        args: *const jvalue,
    ) -> Result<(), Local<'env, E>> {
        ((**self.env).v1_2.CallStaticVoidMethodA)(self.env, class, method, args);
        self.exception_check()
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

    pub unsafe fn set_object_field<R: ReferenceType>(self, this: jobject, field: jfieldID, value: impl AsArg<R>) {
        ((**self.env).v1_2.SetObjectField)(self.env, this, field, value.as_arg());
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

    pub unsafe fn set_static_object_field<R: ReferenceType>(
        self,
        class: jclass,
        field: jfieldID,
        value: impl AsArg<R>,
    ) {
        ((**self.env).v1_2.SetStaticObjectField)(self.env, class, field, value.as_arg());
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

    pub fn throw<T: ReferenceType>(self, throwable: &Ref<T>) {
        let res = unsafe { ((**self.env).v1_2.Throw)(self.env, throwable.as_raw()) };
        assert_eq!(res, 0);
    }
}
