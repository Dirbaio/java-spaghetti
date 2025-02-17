//! New types for `jfieldID` and `jmethodID` that implement `Send` and `Sync`.
//!
//! Inspired by: <https://docs.rs/jni/0.21.1/jni/objects/struct.JMethodID.html>.
//!
//! According to the JNI spec field IDs may be invalidated when the corresponding class is unloaded:
//! <https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/design.html#accessing_fields_and_methods>
//!
//! You should generally not be interacting with these types directly, but it must be public for codegen.

use crate::sys::{jfieldID, jmethodID};

#[doc(hidden)]
#[repr(transparent)]
pub struct JFieldID {
    internal: jfieldID,
}

// Field IDs are valid across threads (not tied to a JNIEnv)
unsafe impl Send for JFieldID {}
unsafe impl Sync for JFieldID {}

impl JFieldID {
    /// Creates a [`JFieldID`] that wraps the given `raw` [`jfieldID`].
    ///
    /// # Safety
    ///
    /// Expects a valid, non-`null` ID.
    pub unsafe fn from_raw(raw: jfieldID) -> Self {
        debug_assert!(!raw.is_null(), "from_raw fieldID argument");
        Self { internal: raw }
    }

    pub fn as_raw(&self) -> jfieldID {
        self.internal
    }
}

#[doc(hidden)]
#[repr(transparent)]
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
    /// Expects a valid, non-`null` ID.
    pub unsafe fn from_raw(raw: jmethodID) -> Self {
        debug_assert!(!raw.is_null(), "from_raw methodID argument");
        Self { internal: raw }
    }

    pub fn as_raw(&self) -> jmethodID {
        self.internal
    }
}
