use std::{char, iter, slice};

use jni_sys::*;

use crate::Env;

/// Represents a JNI `GetStringChars` + `GetStringLength` query.
/// It will call `ReleaseStringChars` automatically when dropped.
pub struct StringChars<'env> {
    env: Env<'env>,
    string: jstring,
    chars: *const jchar,
    length: jsize, // in characters
}

impl<'env> StringChars<'env> {
    /// Construct a `StringChars` from an [Env] + [jstring].
    ///
    /// # Safety
    ///
    /// The Java string object referenced by `string` must remain available before the created
    /// `StringChars` is dropped. This should be true if the JNI reference `string` is not deleted.
    ///
    /// This function is supposed to be used in generated bindings.
    pub unsafe fn from_env_jstring(env: Env<'env>, string: jstring) -> Self {
        debug_assert!(!string.is_null());

        let chars = unsafe { env.get_string_chars(string) };
        let length = unsafe { env.get_string_length(string) };

        debug_assert!(!chars.is_null() || length == 0);

        Self {
            env,
            string,
            chars,
            length,
        }
    }

    /// Get an array of [jchar]s.  Generally UTF16, but not guaranteed to be valid UTF16.
    pub fn chars(&self) -> &[jchar] {
        unsafe { slice::from_raw_parts(self.chars, self.length as usize) }
    }

    /// [std::char::decode_utf16]\(...\)s these string characters.
    pub fn decode(&self) -> char::DecodeUtf16<iter::Cloned<slice::Iter<'_, u16>>> {
        char::decode_utf16(self.chars().iter().cloned())
    }

    /// Returns a new [Ok]\([String]\), or an [Err]\([DecodeUtf16Error](char::DecodeUtf16Error)\) if if it contained any invalid UTF16.
    pub fn to_string(&self) -> Result<String, char::DecodeUtf16Error> {
        self.decode().collect()
    }

    /// Returns a new [String] with any invalid UTF16 characters replaced with [REPLACEMENT_CHARACTER](char::REPLACEMENT_CHARACTER)s (`'\u{FFFD}'`.)
    pub fn to_string_lossy(&self) -> String {
        self.decode()
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }
}

impl<'env> Drop for StringChars<'env> {
    fn drop(&mut self) {
        unsafe { self.env.release_string_chars(self.string, self.chars) };
    }
}
