use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::ptr::null_mut;

use jni_sys::*;

use crate::{AsJValue, AsValidJObjectAndEnv, Env, JniType, Local, ObjectAndEnv, ThrowableType};

/// A Java Array of some POD-like type such as bool, jbyte, jchar, jshort, jint, jlong, jfloat, or jdouble.
///
/// See also [ObjectArray] for arrays of reference types.
///
/// | JNI Type      | PrimitiveArray Implementation |
/// | ------------- | ----------------- |
/// | [bool]\[\]    | [BooleanArray]    |
/// | [jbyte]\[\]   | [ByteArray]       |
/// | [jchar]\[\]   | [CharArray]       |
/// | [jint]\[\]    | [IntArray]        |
/// | [jlong]\[\]   | [LongArray]       |
/// | [jfloat]\[\]  | [FloatArray]      |
/// | [jdouble]\[\] | [DoubleArray]     |
///
/// [bool]:         https://doc.rust-lang.org/std/primitive.bool.html
/// [jbyte]:        https://docs.rs/jni-sys/0.3.0/jni_sys/type.jbyte.html
/// [jchar]:        struct.jchar.html
/// [jint]:         https://docs.rs/jni-sys/0.3.0/jni_sys/type.jint.html
/// [jlong]:        https://docs.rs/jni-sys/0.3.0/jni_sys/type.jlong.html
/// [jfloat]:       https://docs.rs/jni-sys/0.3.0/jni_sys/type.jfloat.html
/// [jdouble]:      https://docs.rs/jni-sys/0.3.0/jni_sys/type.jdouble.html
///
/// [BooleanArray]: struct.BooleanArray.html
/// [ByteArray]:    struct.ByteArray.html
/// [CharArray]:    struct.CharArray.html
/// [IntArray]:     struct.IntArray.html
/// [LongArray]:    struct.LongArray.html
/// [FloatArray]:   struct.FloatArray.html
/// [DoubleArray]:  struct.DoubleArray.html
/// [ObjectArray]:  struct.ObjectArray.html
///
pub trait PrimitiveArray<T>
where
    Self: Sized + AsValidJObjectAndEnv,
    T: Clone + Default,
{
    /// Uses env.New{Type}Array to create a new java array containing "size" elements.
    fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self>;

    /// Uses env.GetArrayLength to get the length of the java array.
    fn len(&self) -> usize;

    /// Uses env.Get{Type}ArrayRegion to read the contents of the java array from \[start .. start + elements.len())
    fn get_region(&self, start: usize, elements: &mut [T]);

    /// Uses env.Set{Type}ArrayRegion to set the contents of the java array from \[start .. start + elements.len())
    fn set_region(&self, start: usize, elements: &[T]);

    /// Uses env.New{Type}Array + Set{Type}ArrayRegion to create a new java array containing a copy of "elements".
    fn from<'env>(env: Env<'env>, elements: &[T]) -> Local<'env, Self> {
        let array = Self::new(env, elements.len());
        array.set_region(0, elements);
        array
    }

    /// Uses env.GetArrayLength + env.Get{Type}ArrayRegion to read the contents of the java array from range into a new Vec.
    fn get_region_as_vec(&self, range: impl RangeBounds<usize>) -> Vec<T> {
        let len = self.len();

        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
        };

        let end = match range.end_bound() {
            Bound::Unbounded => len,
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
        };

        assert!(start <= end);
        assert!(end <= len);
        let vec_len = end - start;

        let mut vec = Vec::new();
        vec.resize(vec_len, Default::default());
        self.get_region(start, &mut vec[..]);
        vec
    }

    /// Uses env.GetArrayLength + env.Get{Type}ArrayRegion to read the contents of the entire java array into a new Vec.
    fn as_vec(&self) -> Vec<T> {
        self.get_region_as_vec(0..self.len())
    }
}

macro_rules! primitive_array {
    (#[repr(transparent)] pub struct $name:ident = $type_str:expr, $type:ident { $new_array:ident $set_region:ident $get_region:ident } ) => {
        /// A [PrimitiveArray](trait.PrimitiveArray.html) implementation.
        #[repr(transparent)]
        pub struct $name(ObjectAndEnv);

        unsafe impl AsValidJObjectAndEnv for $name {}
        unsafe impl AsJValue for $name {
            fn as_jvalue(&self) -> jni_sys::jvalue {
                jni_sys::jvalue { l: self.0.object }
            }
        }
        unsafe impl JniType for $name {
            fn static_with_jni_type<R>(callback: impl FnOnce(&str) -> R) -> R {
                callback($type_str)
            }
        }

        impl PrimitiveArray<$type> for $name {
            fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self> {
                assert!(size <= std::i32::MAX as usize); // jsize == jint == i32
                let size = size as jsize;
                let jnienv = env.as_raw();
                unsafe {
                    let object = ((**jnienv).v1_2.$new_array)(jnienv, size);
                    let exception = ((**jnienv).v1_2.ExceptionOccurred)(jnienv);
                    assert!(exception.is_null()); // Only sane exception here is an OOM exception
                    Local::from_raw(env, object)
                }
            }

            fn from<'env>(env: Env<'env>, elements: &[$type]) -> Local<'env, Self> {
                let array = Self::new(env, elements.len());
                let size = elements.len() as jsize;
                let env = array.0.env;
                let object = array.0.object;
                unsafe {
                    ((**env).v1_1.$set_region)(env, object, 0, size, elements.as_ptr() as *const _);
                }
                array
            }

            fn len(&self) -> usize {
                unsafe { ((**self.0.env).v1_2.GetArrayLength)(self.0.env as *mut _, self.0.object) as usize }
            }

            fn get_region(&self, start: usize, elements: &mut [$type]) {
                assert!(start <= std::i32::MAX as usize); // jsize == jint == i32
                assert!(elements.len() <= std::i32::MAX as usize); // jsize == jint == i32
                let self_len = self.len() as jsize;
                let elements_len = elements.len() as jsize;

                let start = start as jsize;
                let end = start + elements_len;
                assert!(start <= end);
                assert!(end <= self_len);

                unsafe {
                    ((**self.0.env).v1_1.$get_region)(
                        self.0.env as *mut _,
                        self.0.object,
                        start,
                        elements_len,
                        elements.as_mut_ptr() as *mut _,
                    )
                };
            }

            fn set_region(&self, start: usize, elements: &[$type]) {
                assert!(start <= std::i32::MAX as usize); // jsize == jint == i32
                assert!(elements.len() <= std::i32::MAX as usize); // jsize == jint == i32
                let self_len = self.len() as jsize;
                let elements_len = elements.len() as jsize;

                let start = start as jsize;
                let end = start + elements_len;
                assert!(start <= end);
                assert!(end <= self_len);

                unsafe {
                    ((**self.0.env).v1_1.$set_region)(
                        self.0.env as *mut _,
                        self.0.object,
                        start,
                        elements_len,
                        elements.as_ptr() as *const _,
                    )
                };
            }
        }
    };
}

primitive_array! { #[repr(transparent)] pub struct BooleanArray = "[Z\0", bool    { NewBooleanArray SetBooleanArrayRegion GetBooleanArrayRegion } }
primitive_array! { #[repr(transparent)] pub struct ByteArray    = "[B\0", jbyte   { NewByteArray    SetByteArrayRegion    GetByteArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct CharArray    = "[C\0", jchar   { NewCharArray    SetCharArrayRegion    GetCharArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct ShortArray   = "[S\0", jshort  { NewShortArray   SetShortArrayRegion   GetShortArrayRegion   } }
primitive_array! { #[repr(transparent)] pub struct IntArray     = "[I\0", jint    { NewIntArray     SetIntArrayRegion     GetIntArrayRegion     } }
primitive_array! { #[repr(transparent)] pub struct LongArray    = "[J\0", jlong   { NewLongArray    SetLongArrayRegion    GetLongArrayRegion    } }
primitive_array! { #[repr(transparent)] pub struct FloatArray   = "[F\0", jfloat  { NewFloatArray   SetFloatArrayRegion   GetFloatArrayRegion   } }
primitive_array! { #[repr(transparent)] pub struct DoubleArray  = "[D\0", jdouble { NewDoubleArray  SetDoubleArrayRegion  GetDoubleArrayRegion  } }

/// A Java Array of reference types (classes, interfaces, other arrays, etc.)
///
/// See also [PrimitiveArray] for arrays of reference types.
///
/// [PrimitiveArray]:   struct.PrimitiveArray.html
///
#[repr(transparent)]
pub struct ObjectArray<T: AsValidJObjectAndEnv, E: ThrowableType>(ObjectAndEnv, PhantomData<(T, E)>);

unsafe impl<T: AsValidJObjectAndEnv, E: ThrowableType> AsValidJObjectAndEnv for ObjectArray<T, E> {}

unsafe impl<T: AsValidJObjectAndEnv, E: ThrowableType> JniType for ObjectArray<T, E> {
    fn static_with_jni_type<R>(callback: impl FnOnce(&str) -> R) -> R {
        T::static_with_jni_type(|inner| callback(format!("[{}", inner).as_str()))
    }
}

unsafe impl<T: AsValidJObjectAndEnv, E: ThrowableType> AsJValue for ObjectArray<T, E> {
    fn as_jvalue(&self) -> jni_sys::jvalue {
        jni_sys::jvalue { l: self.0.object }
    }
}

impl<T: AsValidJObjectAndEnv, E: ThrowableType> ObjectArray<T, E> {
    pub fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self> {
        assert!(size <= std::i32::MAX as usize); // jsize == jint == i32
        let class = Self::static_with_jni_type(|t| unsafe { env.require_class(t) });
        let size = size as jsize;
        let jnienv = env.as_raw();
        unsafe {
            let fill = null_mut();
            let object = ((**jnienv).v1_2.NewObjectArray)(jnienv, size, class, fill);
            let exception = ((**jnienv).v1_2.ExceptionOccurred)(jnienv);
            assert!(exception.is_null()); // Only sane exception here is an OOM exception
            Local::from_raw(env, object)
        }
    }

    pub fn iter(&self) -> ObjectArrayIter<'_, T, E> {
        ObjectArrayIter {
            array: self,
            index: 0,
            length: self.len(),
        }
    }

    pub fn from<'env>(
        env: Env<'env>,
        elements: impl 'env + ExactSizeIterator + Iterator<Item = impl Into<Option<&'env T>>>,
    ) -> Local<'env, Self> {
        let size = elements.len();
        let array = Self::new(env, size);
        let env = array.0.env;
        let this = array.0.object;

        for (index, element) in elements.enumerate() {
            assert!(index < size); // Should only be violated by an invalid ExactSizeIterator implementation.
            let value = element
                .into()
                .map(|v| unsafe { AsJValue::as_jvalue(v).l })
                .unwrap_or(null_mut());
            unsafe { ((**env).v1_2.SetObjectArrayElement)(env, this, index as jsize, value) };
        }
        array
    }

    pub fn len(&self) -> usize {
        unsafe { ((**self.0.env).v1_2.GetArrayLength)(self.0.env as *mut _, self.0.object) as usize }
    }

    /// XXX: Expose this via std::ops::Index
    pub fn get(&self, index: usize) -> Result<Option<Local<'_, T>>, Local<'_, E>> {
        assert!(index <= std::i32::MAX as usize); // jsize == jint == i32 XXX: Should maybe be treated as an exception?
        let index = index as jsize;
        let env = self.0.env;
        let this = self.0.object;
        unsafe {
            let result = ((**env).v1_2.GetObjectArrayElement)(env, this, index);
            let exception = ((**env).v1_2.ExceptionOccurred)(env);
            if !exception.is_null() {
                ((**env).v1_2.ExceptionClear)(env);
                Err(Local::from_raw(Env::from_raw(env), exception))
            } else if result.is_null() {
                Ok(None)
            } else {
                Ok(Some(Local::from_raw(Env::from_raw(env), result)))
            }
        }
    }

    /// XXX: I don't think there's a way to expose this via std::ops::IndexMut sadly?
    pub fn set<'env>(&'env self, index: usize, value: impl Into<Option<&'env T>>) -> Result<(), Local<'env, E>> {
        assert!(index <= std::i32::MAX as usize); // jsize == jint == i32 XXX: Should maybe be treated as an exception?
        let value = value
            .into()
            .map(|v| unsafe { AsJValue::as_jvalue(v).l })
            .unwrap_or(null_mut());
        let index = index as jsize;
        let env = self.0.env;
        let this = self.0.object;
        unsafe {
            ((**env).v1_2.SetObjectArrayElement)(env, this, index, value);
            let exception = ((**env).v1_2.ExceptionOccurred)(env);
            if !exception.is_null() {
                ((**env).v1_2.ExceptionClear)(env);
                Err(Local::from_raw(Env::from_raw(env), exception))
            } else {
                Ok(())
            }
        }
    }
}

pub struct ObjectArrayIter<'env, T: AsValidJObjectAndEnv, E: ThrowableType> {
    array: &'env ObjectArray<T, E>,
    index: usize,
    length: usize,
}

impl<'env, T: AsValidJObjectAndEnv, E: ThrowableType> Iterator for ObjectArrayIter<'env, T, E> {
    type Item = Option<Local<'env, T>>;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        if index < self.length {
            self.index = index + 1;
            Some(self.array.get(index).unwrap_or(None))
        } else {
            None
        }
    }
}
