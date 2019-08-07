use super::*;
use std::marker::*;
use std::ops::*;

pub trait PrimitiveArray<T> where Self : Sized {
    fn new(env: &Env, size: usize) -> Self;
    fn from(env: &Env, elements: &[T]) -> Self;
    fn len(&self) -> usize;
    fn get_region_as_vec(&self, range: Range<usize>) -> Vec<T>;

    fn as_vec(&self) -> Vec<T> { self.get_region_as_vec(0..self.len()) }
}

// I assume jboolean as used exclusively by JNI/JVM is compatible with bool.
// This is *not* a sound/safe assumption in the general case as jboolean can be any u8 bit pattern.
// However, I believe this *is* a sound/safe assumption when exclusively dealing with JNI/JVM APIs which *should* be
// returning exclusively JNI_TRUE or JNI_FALSE, which are bitwise compatible with Rust's definitions of true / false.
#[test] fn bool_ffi_assumptions_test() {
    use std::mem::*;

    // Assert that the sizes are indeed the same.
    assert_eq!(size_of::<jboolean>(), 1); // Forever
    assert_eq!(size_of::<bool>(),     1); // As of https://github.com/rust-lang/rust/pull/46156/commits/219ba511c824bc44149d55c570f723dcd0f0217d

    // Assert that the underlying representations are indeed the same.
    assert_eq!(unsafe { std::mem::transmute::<bool, u8>(true ) }, JNI_TRUE );
    assert_eq!(unsafe { std::mem::transmute::<bool, u8>(false) }, JNI_FALSE);
}

macro_rules! primitive_array {
    (#[repr(transparent)] pub struct $name:ident = $type:ident { $new_array:ident $set_region:ident $get_region:ident } ) => {
        #[repr(transparent)] pub struct $name(ObjectAndEnv);

        unsafe impl AsValidJObjectAndEnv for $name {}
        unsafe impl AsJValue for $name { fn as_jvalue(&self) -> jni_sys::jvalue { jni_sys::jvalue { l: self.0.object } } }

        impl PrimitiveArray<$type> for $name {
            fn new(env: &Env, size: usize) -> Self {
                assert!(size <= std::i32::MAX as usize); // jsize == jint == i32
                let size = size as jsize;
                let env = env.as_jni_env();
                unsafe {
                    let object = (**env).$new_array.unwrap()(env, size);
                    let exception = (**env).ExceptionOccurred.unwrap()(env);
                    assert!(exception.is_null()); // Only sane exception here is an OOM exception
                    Self(ObjectAndEnv { object, env })
                }
            }

            fn from(env: &Env, elements: &[$type]) -> Self {
                let array  = Self::new(env, elements.len());
                let size   = elements.len() as jsize;
                let env    = array.0.env as *mut JNIEnv;
                let object = array.0.object;
                unsafe {
                    (**env).$set_region.unwrap()(env, object, 0, size, elements.as_ptr() as *const _);
                }
                array
            }

            fn len(&self) -> usize {
                unsafe { (**self.0.env).GetArrayLength.unwrap()(self.0.env as *mut _, self.0.object) as usize }
            }

            fn get_region_as_vec(&self, range: Range<usize>) -> Vec<$type> {
                let len = self.len();
                assert!(range.start <= range.end);
                assert!(range.start <= len);
                assert!(range.end   <= len);

                let mut vec = Vec::new();
                vec.resize(len, Default::default());
                unsafe { (**self.0.env).$get_region.unwrap()(self.0.env as *mut _, self.0.object, range.start as jsize, (range.end - range.start) as jsize, vec.as_mut_ptr() as *mut _) };
                vec
            }
        }
    };
}

primitive_array! { #[repr(transparent)] pub struct BooleanArray = bool    { NewBooleanArray SetBooleanArrayRegion GetBooleanArrayRegion } }
primitive_array! { #[repr(transparent)] pub struct ByteArray    = jbyte   { NewByteArray    SetByteArrayRegion    GetByteArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct CharArray    = jchar   { NewCharArray    SetCharArrayRegion    GetCharArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct ShortArray   = jshort  { NewShortArray   SetShortArrayRegion   GetShortArrayRegion   } }
primitive_array! { #[repr(transparent)] pub struct IntArray     = jint    { NewIntArray     SetIntArrayRegion     GetIntArrayRegion     } }
primitive_array! { #[repr(transparent)] pub struct LongArray    = jlong   { NewLongArray    SetLongArrayRegion    GetLongArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct FloatArray   = jfloat  { NewFloatArray   SetFloatArrayRegion   GetFloatArrayRegion   } }
primitive_array! { #[repr(transparent)] pub struct DoubleArray  = jdouble { NewDoubleArray  SetDoubleArrayRegion  GetDoubleArrayRegion  } }

// TODO: ObjectArray - this is *not* a primitive array.
