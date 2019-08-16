# jni-bindgen

[![GitHub](https://img.shields.io/github/stars/MaulingMonkey/jni-bindgen.svg?label=GitHub&style=social)](https://github.com/MaulingMonkey/jni-bindgen)
[![Build Status](https://travis-ci.org/MaulingMonkey/jni-bindgen.svg)](https://travis-ci.org/MaulingMonkey/jni-bindgen)
![unsafe: yes](https://img.shields.io/badge/unsafe-yes-yellow.svg)
![rust: 1.36.0+](https://img.shields.io/badge/rust-1.36.0%2B-green.svg)
[![Open issues](https://img.shields.io/github/issues-raw/MaulingMonkey/jni-bindgen.svg)](https://github.com/MaulingMonkey/jni-bindgen/issues)
[![License](https://img.shields.io/crates/l/jni-bindgen.svg)](https://github.com/MaulingMonkey/jni-bindgen)
[![dependency status](https://deps.rs/repo/github/MaulingMonkey/jni-bindgen/status.svg)](https://deps.rs/repo/github/MaulingMonkey/jni-bindgen)

**Work in progress, only barely kinda partially usable**

Vaguely inspired by, but otherwise unrelated to, [bindgen](https://github.com/rust-lang/rust-bindgen) and
[wasm-bindgen](https://github.com/rustwasm/wasm-bindgen)'s WebIDL stuff.

Generate Rust JVM FFI wrappers around APIs defined by `.jar` or `.class` files, because maintaining your own
hand-written bindings is an exercise in boredom, soundness bugs, and pain.

## Goals

* Provide a means of using Android system APIs specifically.
* Provide a means of using Java, Kotlin, Scala, or other JVM based APIs.
* Automatically link API documentation, so people might actually read it.
* Eliminate the need to manually write unsound, unreviewed, and [unaudited](https://github.com/dpc/crev) `unsafe { ... }` APIs

## Local Crates

| [github.com](https://github.com)                                                                      | [crates.io](https://crates.io)                                                                                | [docs.rs](https://docs.rs)                                                                | Description |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ----------- |
| [jni-android-sys](https://github.com/MaulingMonkey/jni-bindgen/tree/master/jni-android-sys)           | [![Crates.io](https://img.shields.io/crates/v/jni-android-sys.svg)](https://crates.io/crates/jni-android-sys) | [![Docs](https://docs.rs/jni-android-sys/badge.svg)](https://docs.rs/jni-android-sys/)    | Bindings to Android Java APIs
| [jni-android-sys-gen](https://github.com/MaulingMonkey/jni-bindgen/tree/master/jni-android-sys-gen)   | N/A                                                                                                           | N/A                                                                                       | Generates jni-android-sys
| [jni-bindgen](https://github.com/MaulingMonkey/jni-bindgen/tree/master/jni-bindgen)                   | [![Crates.io](https://img.shields.io/crates/v/jni-bindgen.svg)](https://crates.io/crates/jni-bindgen)         | [![Docs](https://docs.rs/jni-bindgen/badge.svg)](https://docs.rs/jni-bindgen/)            | Generator of Java API bindings
| [jni-glue](https://github.com/MaulingMonkey/jni-bindgen/tree/master/jni-glue)                         | [![Crates.io](https://img.shields.io/crates/v/jni-glue.svg)](https://crates.io/crates/jni-glue)               | [![Docs](https://docs.rs/jni-glue/badge.svg)](https://docs.rs/jni-glue/)                  | Utility functions for Java API bindings

## External Crates

| [github.com](https://github.com)                                                              | [crates.io](https://crates.io)                                                                                | [docs.rs](https://docs.rs)                                                                | License |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ------- |
| [jni-sys](https://github.com/sfackler/rust-jni-sys)                                           | [![Crates.io](https://img.shields.io/crates/v/jni-sys.svg)](https://crates.io/crates/jni-sys)                 | [![Docs](https://docs.rs/jni-sys/badge.svg)](https://docs.rs/jni-sys/)                    | [![License](https://img.shields.io/crates/l/jni-sys.svg)](https://github.com/MaulingMonkey/jni-sys)

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
