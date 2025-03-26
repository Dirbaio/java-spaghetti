use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem::transmute;
use std::ops::Deref;

use jni_sys::jobject;

use crate::{AssignableTo, Env, Global, JavaDebug, JavaDisplay, Local, ReferenceType};

/// A non-null, [reference](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=collector-overview-jni-object-references)
/// to a Java object (+ [Env]).  This may refer to a [Local](crate::Local), [Global](crate::Global), local [Arg](crate::Arg), etc.
///
/// **Not FFI Safe:**  `#[repr(rust)]`, and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// `*const JNIEnv` in thread local storage instead of lugging it around in every [Ref].  Of course, there's no
/// guarantee that's actually an *optimization*...
#[repr(C)] // this is NOT for FFI-safety, this is to ensure `cast` methods are sound.
pub struct Ref<'env, T: ReferenceType> {
    object: jobject,
    env: Env<'env>,
    _class: PhantomData<T>,
}

impl<'env, T: ReferenceType> std::ops::Receiver for Ref<'env, T> {
    type Target = T;
}

impl<'env, T: ReferenceType> Ref<'env, T> {
    /// Wraps an raw JNI reference.
    ///
    /// # Safety
    ///
    /// - `object` is a non-null JNI reference, and it must keep valid for `'env` lifetime;
    /// - `object` references an instance of type `T`.
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            object,
            env,
            _class: PhantomData,
        }
    }

    /// Gets the [Env] under which the JNI reference is valid.
    pub fn env(&self) -> Env<'env> {
        self.env
    }

    /// Returns the raw JNI reference pointer.
    pub fn as_raw(&self) -> jobject {
        self.object
    }

    /// Returns a new JNI global reference of the same Java object.
    pub fn as_global(&self) -> Global<T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Global::from_raw(env.vm(), object) }
    }

    /// Returns a new JNI local reference of the same Java object.
    pub fn as_local(&self) -> Local<'env, T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Local::from_raw(env, object) }
    }

    /// Tests whether two JNI references refer to the same Java object.
    pub fn is_same_object<O: ReferenceType>(&self, other: &Ref<'_, O>) -> bool {
        let jnienv = self.env.as_raw();
        unsafe { ((**jnienv).v1_2.IsSameObject)(jnienv, self.as_raw(), other.as_raw()) }
    }

    /// Checks if the Java object can be safely casted to type `U`.
    pub(crate) fn check_assignable<U: ReferenceType>(&self) -> Result<(), crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class = U::jni_get_class(env).as_raw();
        if !unsafe { ((**jnienv).v1_2.IsInstanceOf)(jnienv, self.as_raw(), class) } {
            return Err(crate::CastError);
        }
        Ok(())
    }

    /// Casts itself to a JNI reference of type `U` forcefully, without the cost of runtime checking.
    ///
    /// # Safety
    ///
    /// - `self` references an instance of type `U`.
    pub unsafe fn cast_unchecked<U: ReferenceType>(self) -> Ref<'env, U> {
        transmute(self)
    }

    /// Tries to cast itself to a JNI reference of type `U`.
    pub fn cast<U: ReferenceType>(self) -> Result<Ref<'env, U>, crate::CastError> {
        self.check_assignable::<U>()?;
        Ok(unsafe { self.cast_unchecked() })
    }

    /// Casts itself towards a super class type, without the cost of runtime checking.
    pub fn upcast<U: ReferenceType>(self) -> Ref<'env, U>
    where
        Self: AssignableTo<U>,
    {
        unsafe { self.cast_unchecked() }
    }

    /// Casts the borrowed `Ref` to a JNI reference of type `U` forcefully, without the cost of runtime checking.
    ///
    /// # Safety
    ///
    /// - `self` references an instance of type `U`.
    pub unsafe fn cast_ref_unchecked<U: ReferenceType>(&self) -> &Ref<'env, U> {
        transmute(self)
    }

    /// Tries to cast the borrowed `Ref` to a JNI reference of type `U`.
    pub fn cast_ref<U: ReferenceType>(&self) -> Result<&Ref<'env, U>, crate::CastError> {
        self.check_assignable::<U>()?;
        Ok(unsafe { self.cast_ref_unchecked() })
    }

    /// Casts the borrowed `Ref` towards a super class type, without the cost of runtime checking.
    pub fn upcast_ref<U: ReferenceType>(&self) -> &Ref<'env, U>
    where
        Self: AssignableTo<U>,
    {
        unsafe { self.cast_ref_unchecked() }
    }

    /// Enters monitored mode for the corresponding object in this thread. See [Monitor].
    pub fn monitor<'r>(&'r self) -> Monitor<'env, 'r, T> {
        Monitor::new(self)
    }
}

impl<'env, T: JavaDebug> Debug for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<'env, T: JavaDisplay> Display for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

/// A non-null reference of a Java object locked with the JNI monitor mechanism, providing *limited* thread safety.
///
/// It is important to drop the monitor or call [Monitor::unlock()] when appropriate.
///
/// Limitations:
///
/// - It merely blocks other native functions from owning the JNI monitor of the same object before it drops.
/// - It may not block other native functions from using the corresponding object without entering monitored mode.
/// - It may not prevent any Java method or block from using this object, even if it is marked as `synchronized`.
/// - While it is a reentrant lock for the current thread, dead lock is still possible under multi-thread conditions.
pub struct Monitor<'env, 'r, T: ReferenceType> {
    inner: Option<&'r Ref<'env, T>>,
}

impl<'env, 'r, T: ReferenceType> Monitor<'env, 'r, T> {
    fn new(reference: &'r Ref<'env, T>) -> Self {
        let jnienv = reference.env.as_raw();
        let result = unsafe { ((**jnienv).v1_2.MonitorEnter)(jnienv, reference.as_raw()) };
        assert!(result == jni_sys::JNI_OK);
        Self { inner: Some(reference) }
    }

    /// Decrements the JNI monitor counter indicating the number of times it has entered this monitor. If the value of
    /// the counter becomes zero, the current thread releases the monitor.
    pub fn unlock(mut self) -> &'r Ref<'env, T> {
        self.unlock_inner();
        self.inner.take().unwrap()
    }

    fn unlock_inner(&mut self) {
        if let Some(inner) = self.inner.as_ref() {
            let env = inner.env;
            let jnienv = env.as_raw();
            let result = unsafe { ((**jnienv).v1_2.MonitorExit)(jnienv, inner.as_raw()) };
            assert!(result == jni_sys::JNI_OK);
            if let Err(exception) = env.exception_check_raw() {
                panic!(
                    "exception happened calling JNI MonitorExit, the monitor is probably broken previously: {}",
                    unsafe { env.raw_exception_to_string(exception) }
                );
            }
        }
    }
}

impl<'env, 'r, T: ReferenceType> Deref for Monitor<'env, 'r, T> {
    type Target = Ref<'env, T>;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<'env, 'r, T: ReferenceType> Drop for Monitor<'env, 'r, T> {
    fn drop(&mut self) {
        self.unlock_inner();
    }
}
