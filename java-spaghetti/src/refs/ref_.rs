use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem::transmute;

use jni_sys::jobject;

use crate::{AssignableTo, Env, Global, JavaDebug, JavaDisplay, Local, ReferenceType};

/// A non-null, [reference](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=collector-overview-jni-object-references)
/// to a Java object (+ [Env]).  This may refer to a [Local](crate::Local), [Global](crate::Global), local [Arg](crate::Arg), etc.
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
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
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            object,
            env,
            _class: PhantomData,
        }
    }

    pub fn env(&self) -> Env<'env> {
        self.env
    }

    pub fn as_raw(&self) -> jobject {
        self.object
    }

    pub fn as_global(&self) -> Global<T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Global::from_raw(env.vm(), object) }
    }

    pub fn as_local(&self) -> Local<'env, T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Local::from_raw(self.env(), object) }
    }

    fn check_assignable<U: ReferenceType>(&self) -> Result<(), crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.as_raw()) };
        let class2 = U::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        Ok(())
    }

    pub unsafe fn cast_unchecked<U: ReferenceType>(self) -> Ref<'env, U> {
        transmute(self)
    }

    pub fn cast<U: ReferenceType>(self) -> Result<Ref<'env, U>, crate::CastError> {
        self.check_assignable::<U>()?;
        Ok(unsafe { self.cast_unchecked() })
    }

    pub fn upcast<U: ReferenceType>(self) -> Ref<'env, U>
    where
        Self: AssignableTo<U>,
    {
        unsafe { self.cast_unchecked() }
    }

    pub unsafe fn cast_ref_unchecked<U: ReferenceType>(&self) -> &Ref<'env, U> {
        transmute(self)
    }

    pub fn cast_ref<U: ReferenceType>(&self) -> Result<&Ref<'env, U>, crate::CastError> {
        self.check_assignable::<U>()?;
        Ok(unsafe { self.cast_ref_unchecked() })
    }

    pub fn upcast_ref<U: ReferenceType>(&self) -> &Ref<'env, U>
    where
        Self: AssignableTo<U>,
    {
        unsafe { self.cast_ref_unchecked() }
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
