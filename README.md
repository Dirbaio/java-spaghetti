# ☕️🍝 `java-spaghetti`

Generate type-safe bindings for calling Java APIs from Rust, so you can fearlessly make your humongous Java spaghetti code aglomeration trascend language barriers and occupy an even larger fraction of the universe's space-time.

## Differences vs `jni-bindgen`

This project originally started out as a fork of [`jni-bindgen`](https://github.com/MaulingMonkey/jni-bindgen).

The main difference is the intended usage: `jni-bindgen` aims to generate crates with bindings for a whole Java API (such as `jni-android-sys`) which
you then use from your crate. `java-spaghetti` is instead designed to generate "mini-bindings" tailored to your project, that you can embed within your crate. This has a few advantages:

- You can generate a single bindings file for e.g. part of the Android API and your project's classes at the same time, which is better because you end up with only one copy of shared classes like `java.lang.String`.
- Java APIs can get big. `jni-android-sys` uses one Cargo feature per class to avoid compile time bloat, which is [no longer allowed on crates.io](https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html).

The full list of differences are:

- Simplified and more consistent API.
- Support for casting and upcasting.
- Added FFI-safe `Return<T>` for returning Java objects from JNI calls.
- Arguments to method calls use a custom `AsArg` trait to make them more ergonomic (doesn't need stuff like `&**foo` or `Some(foo)`).
- You can filter which classes are generated in the TOML config.
- Generated code uses relative paths (`super::...`) instead of absolute paths (`crate::...`), so it works if you place it in a submodule not at the crate root.
- Generated code is a single `.rs` file, there's no support for spltting it in one file per class. You can still run the output through [form](https://github.com/djmcgill/form), if you want.
- Generated code uses cached method IDs and field IDs stored in `OnceLock` to speed up invocations by several times. Used classes are also stored as JNI global references in order to keep the validity of cached IDs. This may not ensure memory safety when class redefinition features (e.g. `java.lang.instrument` which is unavailable on Android) of the JVM are being used.
- Generated code doesn't use macros.
- No support for generating Cargo features per class.
- Modernized rust, updated dependencies.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

<!-- https://doc.rust-lang.org/1.4.0/complement-project-faq.html#why-dual-mit/asl2-license? -->
<!-- https://rust-lang-nursery.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive -->
<!-- https://choosealicense.com/licenses/apache-2.0/ -->
<!-- https://choosealicense.com/licenses/mit/ -->
