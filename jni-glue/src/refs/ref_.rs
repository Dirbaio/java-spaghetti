use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;

use jni_sys::jobject;

use crate::{Env, ObjectAndEnv, ReferenceType};

/// A non-null, [reference](https://www.ibm.com/support/knowledgecenter/en/SSYKE2_8.0.0/com.ibm.java.vm.80.doc/docs/jni_refs.html)
/// to a Java object (+ [Env]).  This may refer to a [Local](crate::Local), [Global](crate::Global), local [Argument](crate::Argument), etc.
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
#[repr(transparent)]
pub struct Ref<'env, T: ReferenceType> {
    oae: ObjectAndEnv,
    _env: PhantomData<Env<'env>>,
    _class: PhantomData<&'env T>,
}

impl<'env, T: ReferenceType> Copy for Ref<'env, T> {}
impl<'env, T: ReferenceType> Clone for Ref<'env, T> {
    fn clone(&self) -> Self {
        Self {
            oae: self.oae,
            _env: PhantomData,
            _class: PhantomData,
        }
    }
}

impl<'env, T: ReferenceType> Ref<'env, T> {
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            oae: ObjectAndEnv {
                object,
                env: env.as_raw(),
            },
            _env: PhantomData,
            _class: PhantomData,
        }
    }

    pub fn env(&self) -> Env<'env> {
        unsafe { Env::from_raw(self.oae.env) }
    }

    pub fn as_raw(&self) -> jobject {
        self.oae.object
    }

    pub fn cast<U: ReferenceType>(&self) -> Result<Ref<'env, U>, crate::CastError> {
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

impl<'env, T: ReferenceType> Deref for Ref<'env, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(&self.oae as *const ObjectAndEnv as *const Self::Target) }
    }
}

impl<'env, T: ReferenceType + Debug> Debug for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'env, T: ReferenceType + Display> Display for Ref<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
