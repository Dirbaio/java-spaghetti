// For easier review, codegen uses this macro, to ensure all output is consistent.

#[doc(hidden)] // For codegen use only, not (yet?) an otherwise stable part of the glue interface.
#[macro_export]
macro_rules! __jni_bindgen {
    () => {};



    (@deref $from:ty => (); $($rest:tt)*) => {
        __jni_bindgen! { $($rest)* }
    };

    (@deref $from:ty => $target:ty; $($rest:tt)*) => {
        impl $crate::std::ops::Deref for $from {
            type Target = $target;
            fn deref(&self) -> &Self::Target {
                unsafe { &*(self as *const Self as *const Self::Target) }
            }
        }
        __jni_bindgen! { $($rest)* }
    };

    (@implements $from:ty => $target:ty; $($rest:tt)*) => {
        impl $crate::std::convert::AsRef<$target> for $from {
            fn as_ref(&self) -> &$target {
                unsafe { &*(self as *const Self as *const $target) }
            }
        }
        __jni_bindgen! { $($rest)* }
    };



    ($(#[$attr:meta])* private static class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] struct $name;
        impl $name { $($body)* }
        __jni_bindgen! {
            // static
            $($rest)*
        }
    };

    ($(#[$attr:meta])* private final class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* private class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* private enum $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* private interface $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };



    ($(#[$attr:meta])* public static class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] pub struct $name;
        impl $name { $($body)* }
        __jni_bindgen! {
            // static
            $($rest)*
        }
    };

    ($(#[$attr:meta])* public final class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] pub struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* public class $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] pub struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* public enum $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] pub struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };

    ($(#[$attr:meta])* public interface $name:ident extends $parent:ty $(, implements $($interface:ty),+)* { $($body:tt)* } $($rest:tt)*) => {
        $(#[$attr])* #[repr(transparent)] pub struct $name($crate::ObjectAndEnv);
        impl $name { $($body)* }
        unsafe impl $crate::AsValidJObjectAndEnv for $name {}
        unsafe impl $crate::AsJValue for $name { fn as_jvalue(&self) -> $crate::jni_sys::jvalue { $crate::jni_sys::jvalue { l: self.0.object } } }
        __jni_bindgen! {
            $($(@implements $name => $interface;)*)*
            @deref $name => $parent;
            $($rest)*
        }
    };
}
