//! Rust generation logic

mod classes;
mod context;
mod fields;
mod known_docs_url;
mod methods;
mod modules;
mod preamble;

pub use context::Context;

/// Writes the string (with "\0" added at the right side) surrounded by double quotes.
///
/// XXX: This implementation (as well as `Env` methods in `java-spaghetti` crate)
/// should probably use byte slices so that full Unicode support can be made possible:
/// JNI `GetFieldID` and `GetMethodID` expects *modified* UTF-8 string name and signature.
/// Note: `cafebabe` converts modified UTF-8 string to real UTF-8 string at first hand.
struct StrEmitter<T: std::fmt::Display>(pub T);

impl<T: std::fmt::Display> std::fmt::Display for StrEmitter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        f.write_char('\"')?;
        self.0.fmt(f)?;
        f.write_str("\\0\"")
    }
}
