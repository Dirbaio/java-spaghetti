use std::fmt::{self, Debug, Display, Formatter};
use std::mem::transmute;
use std::ops::Deref;

use jni_sys::*;

use crate::{AssignableTo, Env, Global, JavaDebug, JavaDisplay, Ref, ReferenceType, Return};

/// A [Local](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=collector-overview-jni-object-references),
/// non-null, reference to a Java object (+ [Env]) limited to the current thread/stack.
///
/// Including the `env` allows for the convenient execution of methods without having to individually pass the `env` as
/// an argument to each and every one.  Since this is limited to the current thread/stack, these cannot be sanely stored
/// in any kind of static storage, nor shared between threads - instead use a [Global] if you need to do either.
///
/// Will `DeleteLocalRef` when dropped, invalidating the `jobject` but ensuring threads that rarely or never return to
/// Java may run without being guaranteed to eventually exhaust their local reference limit.  If this is not desired,
/// convert to a plain [Ref] with:
///
/// ```rust,no_run
/// # use java_spaghetti::*;
/// # fn example<T: ReferenceType>(local: Local<T>) {
/// let local = Local::leak(local);
/// # }
/// ```
///
/// **Not FFI Safe:**  `#[repr(rust)]`, and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// `*const JNIEnv` in thread local storage instead of lugging it around in every [Local].  Of course, there's no
/// guarantee that's actually an *optimization*...
#[repr(C)] // this is NOT for FFI-safety, this is to ensure `cast` methods are sound.
pub struct Local<'env, T: ReferenceType> {
    ref_: Ref<'env, T>,
}

impl<'env, T: ReferenceType> Local<'env, T> {
    /// Wraps an owned raw JNI local reference, taking the ownership.
    ///
    /// # Safety
    ///
    /// - `object` must be an owned non-null JNI local reference that belongs to `env`,
    ///   not to be deleted by another wrapper.
    /// - `object` references an instance of type `T`.
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            ref_: Ref::from_raw(env, object),
        }
    }

    /// Gets the [Env] under which the JNI reference is valid.
    pub fn env(&self) -> Env<'env> {
        self.ref_.env()
    }

    /// Returns the raw JNI reference pointer.
    pub fn as_raw(&self) -> jobject {
        self.ref_.as_raw()
    }

    /// Leaks the `Local` and turns it into a raw pointer, preserving the ownership of one JNI
    /// local reference; prevents `DeleteLocalRef` from being called on dropping. See [Local::leak].
    pub fn into_raw(self) -> jobject {
        let object = self.ref_.as_raw();
        std::mem::forget(self); // Don't allow `Local` to delete it
        object
    }

    /// Leaks the `Local`, prevents `DeleteLocalRef` from being called on dropping.
    ///
    /// If the current thread is a Java thread, it will be freed when the control flow returns
    /// to Java; otherwise it will be freed when the native thread exits.
    ///
    /// NOTE: some JVM implementations have a strict limitation of local reference capacity,
    /// an uncatchable error will be thrown if the capacity is full.
    pub fn leak(self) -> Ref<'env, T> {
        unsafe { Ref::from_raw(self.env(), self.into_raw()) }
    }

    /// Returns a new JNI global reference of the same Java object.
    pub fn as_global(&self) -> Global<T> {
        self.as_ref().as_global()
    }

    /// Creates and leaks a new local reference to be returned from the JNI `extern` callback function.
    /// It will be freed as soon as the control flow returns to Java.
    pub fn as_return(&self) -> Return<'env, T> {
        self.clone().into_return()
    }

    /// Leaks the local reference to be returned from the JNI `extern` callback function.
    /// It will be freed as soon as the control flow returns to Java.
    pub fn into_return(self) -> Return<'env, T> {
        unsafe { Return::from_raw(self.into_raw()) }
    }

    /// Tries to cast itself to a JNI reference of type `U`.
    pub fn cast<U: ReferenceType>(self) -> Result<Local<'env, U>, crate::CastError> {
        self.as_ref().check_assignable::<U>()?;
        // Memory layout of the inner `Ref<'env, U>` is the same as `Ref<'env, T>`.
        Ok(unsafe { transmute::<Local<'_, T>, Local<'_, U>>(self) })
    }

    /// Casts itself towards a super class type, without the cost of runtime checking.
    pub fn upcast<U: ReferenceType>(self) -> Local<'env, U>
    where
        Self: AssignableTo<U>,
    {
        // Memory layout of the inner `Ref<'env, U>` is the same as `Ref<'env, T>`.
        unsafe { transmute(self) }
    }
}

impl<'env, T: ReferenceType> From<Ref<'env, T>> for Local<'env, T> {
    fn from(x: Ref<'env, T>) -> Self {
        x.as_local()
    }
}

impl<'env, T: ReferenceType> From<&Local<'env, T>> for Local<'env, T> {
    fn from(x: &Local<'env, T>) -> Self {
        x.clone()
    }
}

impl<'env, T: ReferenceType> From<&Ref<'env, T>> for Local<'env, T> {
    fn from(x: &Ref<'env, T>) -> Self {
        x.as_local()
    }
}

// NOTE: `AsRef` would become **unsound** if `Ref` should implement `Copy` or `Clone`.
//
// It is possible to have a safe `pub fn as_ref(&'env self) -> Ref<'env, T>` outside of
// `AsRef` trait, however a borrowed `Ref` is returned for the convenience of use.
impl<'env, T: ReferenceType> AsRef<Ref<'env, T>> for Local<'env, T> {
    fn as_ref(&self) -> &Ref<'env, T> {
        &self.ref_
    }
}

// NOTE: `Deref` would become **unsound** if `Ref` should implement `Copy` or `Clone`.
impl<'env, T: ReferenceType> Deref for Local<'env, T> {
    type Target = Ref<'env, T>;
    fn deref(&self) -> &Self::Target {
        &self.ref_
    }
}

impl<'env, T: ReferenceType> Clone for Local<'env, T> {
    fn clone(&self) -> Self {
        let env = self.env().as_raw();
        let object = unsafe { ((**env).v1_2.NewLocalRef)(env, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Self::from_raw(self.env(), object) }
    }
}

impl<'env, T: ReferenceType> Drop for Local<'env, T> {
    fn drop(&mut self) {
        let env = self.env().as_raw();
        unsafe { ((**env).v1_2.DeleteLocalRef)(env, self.as_raw()) }
    }
}

impl<'env, T: JavaDebug> Debug for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<'env, T: JavaDisplay> Display for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}
