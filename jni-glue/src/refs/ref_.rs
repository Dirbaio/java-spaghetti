use std::marker::PhantomData;
use std::ops::Deref;

use jni_sys::jobject;

use crate::{AsValidJObjectAndEnv, Env, ObjectAndEnv};

/// A non-null, [reference](https://www.ibm.com/support/knowledgecenter/en/SSYKE2_8.0.0/com.ibm.java.vm.80.doc/docs/jni_refs.html)
/// to a Java object (+ &[Env]).  This may refer to a [Local], [Global], local [Argument], etc.
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
///
/// [Env]:      struct.Env.html
/// [Local]:    struct.Local.html
/// [Global]:   struct.Global.html
/// [Argument]: struct.Argument.html
#[derive(Copy, Clone)]
pub struct Ref<'env, Class: AsValidJObjectAndEnv> {
    pub(crate) oae: ObjectAndEnv,
    pub(crate) _env: PhantomData<Env<'env>>,
    pub(crate) _class: PhantomData<&'env Class>,
}

impl<'env, Class: AsValidJObjectAndEnv> Ref<'env, Class> {
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

    pub fn cast<Class2: AsValidJObjectAndEnv>(&self) -> Result<Ref<'env, Class2>, crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.oae.object) };
        let class2 = Class2::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        Ok(unsafe { Ref::from_raw(env, self.oae.object) })
    }
}

impl<'env, Class: AsValidJObjectAndEnv> Deref for Ref<'env, Class> {
    type Target = Class;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(&self.oae as *const ObjectAndEnv as *const Self::Target) }
    }
}
