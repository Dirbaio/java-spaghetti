//! New type for cached class objects as JNI global references; new types for `jfieldID` and `jmethodID` that
//! implement `Send` and `Sync`.
//!
//! Inspired by: <https://docs.rs/jni/0.21.1/jni/objects/struct.JMethodID.html>.

use crate::sys::{jclass, jfieldID, jmethodID, jobject};
use crate::{Env, VM};

/// New type for cached class objects as JNI global references.
///
/// Holding a `JClass` global reference prevents the corresponding Java class from being unloaded.
#[derive(Debug)]
pub struct JClass {
    class: jclass,
    vm: VM,
}

unsafe impl Send for JClass {}
unsafe impl Sync for JClass {}

impl JClass {
    /// Creates a `JClass` from an owned JNI local reference of a class object and *deletes* the
    /// local reference.
    ///
    /// # Safety
    ///
    /// `class` must be a valid JNI local reference to a `java.lang.Class` object.
    /// Do not use the passed `class` local reference after calling this function.
    ///
    /// It is safe to pass the returned value of JNI `FindClass` to it if no exeception occurred.
    pub unsafe fn from_raw<'env>(env: Env<'env>, class: jclass) -> Self {
        assert!(!class.is_null(), "from_raw jclass argument is null");
        let jnienv = env.as_raw();
        let class_global = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, class) };
        unsafe { ((**jnienv).v1_2.DeleteLocalRef)(jnienv, class) }
        unsafe { Self::from_raw_global(env.vm(), class_global) }
    }

    /// Wraps an owned raw JNI global reference of a class object.
    ///
    /// # Safety
    ///
    /// `class` must be a valid JNI global reference to a `java.lang.Class` object.
    pub unsafe fn from_raw_global(vm: VM, class: jobject) -> Self {
        assert!(!class.is_null(), "from_raw_global jclass argument is null");
        Self {
            class: class as jclass,
            vm,
        }
    }

    /// Returns the raw JNI reference pointer.
    pub fn as_raw(&self) -> jclass {
        self.class
    }

    /// Turns it into a raw global reference; prevents `DeleteGlobalRef` from being called on dropping.
    pub fn into_raw(self) -> jclass {
        let class = self.class;
        std::mem::forget(self); // Don't delete the object.
        class
    }
}

impl Clone for JClass {
    fn clone(&self) -> Self {
        self.vm.with_env(|env| {
            let env = env.as_raw();
            let class_global = unsafe { ((**env).v1_2.NewGlobalRef)(env, self.class) };
            assert!(!class_global.is_null());
            unsafe { Self::from_raw_global(self.vm, class_global) }
        })
    }
}

// XXX: Unfortunately, static items (e.g. `OnceLock`) may not call drop() at the end of the Rust program:
// JNI global references may be leaked if `java-spaghetti`-based libraries are unloaded and reloaded by the VM.
impl Drop for JClass {
    fn drop(&mut self) {
        self.vm.with_env(|env| {
            let env = env.as_raw();
            unsafe { ((**env).v1_2.DeleteGlobalRef)(env, self.class) }
        });
    }
}

/// New type for `jfieldID`, implements `Send` and `Sync`.
///
/// According to the JNI spec, field IDs may be invalidated when the corresponding class is unloaded:
/// <https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/design.html#accessing_fields_and_methods>.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JFieldID {
    internal: jfieldID,
}

// Field IDs are valid across threads (not tied to a JNIEnv)
unsafe impl Send for JFieldID {}
unsafe impl Sync for JFieldID {}

impl JFieldID {
    /// Creates a [`JFieldID`] that wraps the given raw [`jfieldID`].
    ///
    /// # Safety
    ///
    /// Expects a valid, non-null ID.
    pub unsafe fn from_raw(raw: jfieldID) -> Self {
        assert!(!raw.is_null(), "from_raw jfieldID argument is null");
        Self { internal: raw }
    }

    pub fn as_raw(&self) -> jfieldID {
        self.internal
    }
}

/// New type for `jmethodID`, implements `Send` and `Sync`.
///
/// According to the JNI spec, method IDs may be invalidated when the corresponding class is unloaded:
/// <https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/design.html#accessing_fields_and_methods>.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JMethodID {
    internal: jmethodID,
}

// Method IDs are valid across threads (not tied to a JNIEnv)
unsafe impl Send for JMethodID {}
unsafe impl Sync for JMethodID {}

impl JMethodID {
    /// Creates a [`JMethodID`] that wraps the given `raw` [`jmethodID`].
    ///
    /// # Safety
    ///
    /// Expects a valid, non-null ID.
    pub unsafe fn from_raw(raw: jmethodID) -> Self {
        assert!(!raw.is_null(), "from_raw jmethodID argument is null");
        Self { internal: raw }
    }

    pub fn as_raw(&self) -> jmethodID {
        self.internal
    }
}
