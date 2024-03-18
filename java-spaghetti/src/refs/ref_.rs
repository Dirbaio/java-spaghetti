use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;

use jni_sys::jobject;

use crate::{Env, JavaDebug, JavaDisplay, ReferenceType};

/// A non-null, [reference](https://www.ibm.com/support/knowledgecenter/en/SSYKE2_8.0.0/com.ibm.java.vm.80.doc/docs/jni_refs.html)
/// to a Java object (+ [Env]).  This may refer to a [Local](crate::Local), [Global](crate::Global), local [Argument](crate::Argument), etc.
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
pub struct Ref<'env, T: ReferenceType> {
    object: jobject,
    env: Env<'env>,
    _class: PhantomData<&'env T>,
}

impl<'env, T: ReferenceType> Copy for Ref<'env, T> {}
impl<'env, T: ReferenceType> Clone for Ref<'env, T> {
    fn clone(&self) -> Self {
        Self {
            object: self.object,
            env: self.env,
            _class: PhantomData,
        }
    }
}

impl<'env, T: ReferenceType> Ref<'env, T> {
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            object,
            env,
            _class: PhantomData,
        }
    }

    pub fn env(self) -> Env<'env> {
        self.env
    }

    pub fn as_raw(self) -> jobject {
        self.object
    }

    pub fn cast<U: ReferenceType>(self) -> Result<Ref<'env, U>, crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.as_raw()) };
        let class2 = U::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        Ok(unsafe { Ref::from_raw(env, self.as_raw()) })
    }
}

impl<'env, T: ReferenceType> std::ops::Receiver for Ref<'env, T> {}

impl<'env, T: ReferenceType> Deref for Ref<'env, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        panic!("pls no deref")
    }
}

impl<'env, T: JavaDebug> Debug for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(*self, f)
    }
}

impl<'env, T: JavaDisplay> Display for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(*self, f)
    }
}
