use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::ptr::null_mut;
use std::sync::{LazyLock, OnceLock, RwLock};

use jni_sys::*;

use crate::{AsArg, Env, JClass, Local, Ref, ReferenceType, ThrowableType};

/// A Java Array of some POD-like type such as `bool`, `jbyte`, `jchar`, `jshort`, `jint`, `jlong`, `jfloat`, or `jdouble`.
///
/// Thread safety of avoiding [race conditions](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=jni-synchronization)
/// is not guaranteed. JNI `GetPrimitiveArrayCritical` cannot ensure exclusive access to the array, so it is not used here.
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
pub trait PrimitiveArray<T>: Sized + ReferenceType
where
    T: Clone + Default,
{
    /// Uses JNI `New{Type}Array` to create a new Java array containing "size" elements.
    fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self>;

    /// Uses JNI `GetArrayLength` to get the length of the Java array.
    fn len(self: &Ref<'_, Self>) -> usize;

    /// Uses JNI `GetArrayLength` to get the length of the Java array, returns `true` if it is 0.
    fn is_empty(self: &Ref<'_, Self>) -> bool {
        self.len() == 0
    }

    /// Uses JNI `Get{Type}ArrayRegion` to read the contents of the Java array within `[start .. start + elements.len()]`.
    ///
    /// Panics if the index is out of bound.
    fn get_region(self: &Ref<'_, Self>, start: usize, elements: &mut [T]);

    /// Uses JNI `Set{Type}ArrayRegion` to set the contents of the Java array within `[start .. start + elements.len()]`.
    ///
    /// Panics if the index is out of bound.
    fn set_region(self: &Ref<'_, Self>, start: usize, elements: &[T]);

    /// Uses JNI `New{Type}Array` + `Set{Type}ArrayRegion` to create a new Java array containing a copy of "elements".
    fn new_from<'env>(env: Env<'env>, elements: &[T]) -> Local<'env, Self> {
        let array = Self::new(env, elements.len());
        array.set_region(0, elements);
        array
    }

    /// Uses JNI `GetArrayLength` + `Get{Type}ArrayRegion` to read the contents of the Java array within given range
    /// into a new `Vec`.
    ///
    /// Panics if the index is out of bound.
    fn get_region_as_vec(self: &Ref<'_, Self>, range: impl RangeBounds<usize>) -> Vec<T> {
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

    /// Uses JNI `GetArrayLength` + `Get{Type}ArrayRegion` to read the contents of the entire Java array into a new `Vec`.
    fn as_vec(self: &Ref<'_, Self>) -> Vec<T> {
        self.get_region_as_vec(0..self.len())
    }
}

macro_rules! primitive_array {
    ($name:ident, $type_str:expr, $type:ident { $new_array:ident $set_region:ident $get_region:ident } ) => {
        /// A [PrimitiveArray] implementation.
        pub enum $name {}

        unsafe impl ReferenceType for $name {
            fn jni_reference_type_name() -> Cow<'static, CStr> {
                Cow::Borrowed($type_str)
            }
            unsafe fn jni_class_cache_once_lock() -> &'static OnceLock<JClass> {
                static CLASS_CACHE: OnceLock<JClass> = OnceLock::new();
                &CLASS_CACHE
            }
        }

        impl PrimitiveArray<$type> for $name {
            fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self> {
                assert!(size <= i32::MAX as usize); // jsize == jint == i32
                let size = size as jsize;
                let jnienv = env.as_raw();
                unsafe {
                    let object = ((**jnienv).v1_2.$new_array)(jnienv, size);
                    env.exception_check_raw().expect("OOM");
                    Local::from_raw(env, object)
                }
            }

            fn new_from<'env>(env: Env<'env>, elements: &[$type]) -> Local<'env, Self> {
                let array = Self::new(env, elements.len());
                let size = elements.len() as jsize;
                let env = array.env().as_raw();
                unsafe {
                    ((**env).v1_1.$set_region)(env, array.as_raw(), 0, size, elements.as_ptr() as *const _);
                }
                array
            }

            fn len(self: &Ref<'_, Self>) -> usize {
                let env = self.env().as_raw();
                unsafe { ((**env).v1_2.GetArrayLength)(env, self.as_raw()) as usize }
            }

            fn get_region(self: &Ref<'_, Self>, start: usize, elements: &mut [$type]) {
                assert!(start <= i32::MAX as usize); // jsize == jint == i32
                assert!(elements.len() <= i32::MAX as usize); // jsize == jint == i32
                let self_len = self.len() as jsize;
                let elements_len = elements.len() as jsize;

                let start = start as jsize;
                let end = start + elements_len;
                assert!(start <= end);
                assert!(end <= self_len);

                let env = self.env().as_raw();
                unsafe {
                    ((**env).v1_1.$get_region)(
                        env,
                        self.as_raw(),
                        start,
                        elements_len,
                        elements.as_mut_ptr() as *mut _,
                    )
                };
            }

            fn set_region(self: &Ref<'_, Self>, start: usize, elements: &[$type]) {
                assert!(start <= i32::MAX as usize); // jsize == jint == i32
                assert!(elements.len() <= i32::MAX as usize); // jsize == jint == i32
                let self_len = self.len() as jsize;
                let elements_len = elements.len() as jsize;

                let start = start as jsize;
                let end = start + elements_len;
                assert!(start <= end);
                assert!(end <= self_len);

                let env = self.env().as_raw();
                unsafe {
                    ((**env).v1_1.$set_region)(
                        env,
                        self.as_raw(),
                        start,
                        elements_len,
                        elements.as_ptr() as *const _,
                    )
                };
            }
        }
    };
}

primitive_array! { BooleanArray, c"[Z", bool    { NewBooleanArray SetBooleanArrayRegion GetBooleanArrayRegion } }
primitive_array! { ByteArray,    c"[B", jbyte   { NewByteArray    SetByteArrayRegion    GetByteArrayRegion    } }
primitive_array! { CharArray,    c"[C", jchar   { NewCharArray    SetCharArrayRegion    GetCharArrayRegion    } }
primitive_array! { ShortArray,   c"[S", jshort  { NewShortArray   SetShortArrayRegion   GetShortArrayRegion   } }
primitive_array! { IntArray,     c"[I", jint    { NewIntArray     SetIntArrayRegion     GetIntArrayRegion     } }
primitive_array! { LongArray,    c"[J", jlong   { NewLongArray    SetLongArrayRegion    GetLongArrayRegion    } }
primitive_array! { FloatArray,   c"[F", jfloat  { NewFloatArray   SetFloatArrayRegion   GetFloatArrayRegion   } }
primitive_array! { DoubleArray,  c"[D", jdouble { NewDoubleArray  SetDoubleArrayRegion  GetDoubleArrayRegion  } }

/// A Java Array of reference types (classes, interfaces, other arrays, etc.)
///
/// Thread safety of avoiding [race conditions](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=jni-synchronization)
/// is not guaranteed.
///
/// See also [PrimitiveArray] for arrays of reference types.
pub struct ObjectArray<T: ReferenceType, E: ThrowableType>(core::convert::Infallible, PhantomData<(T, E)>);

// NOTE: This is a performance compromise for returning `&'static JClass`, still faster than non-cached `FindClass`.
static OBJ_ARR_CLASSES: LazyLock<RwLock<HashMap<CString, &'static OnceLock<JClass>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

unsafe impl<T: ReferenceType, E: ThrowableType> ReferenceType for ObjectArray<T, E> {
    fn jni_reference_type_name() -> Cow<'static, CStr> {
        let item_type = T::jni_reference_type_name();
        let item_type = item_type.to_string_lossy();
        let array_type = if !item_type.starts_with('[') {
            format!("[L{item_type};")
        } else {
            format!("[{item_type}")
        };
        Cow::Owned(CString::new(array_type).unwrap())
    }

    unsafe fn jni_class_cache_once_lock() -> &'static OnceLock<JClass> {
        let t = Self::jni_reference_type_name();
        let class_map_reader = OBJ_ARR_CLASSES.read().unwrap();
        if let Some(&once_lock) = class_map_reader.get(t.as_ref()) {
            once_lock
        } else {
            drop(class_map_reader);
            let once_lock: &'static OnceLock<_> = Box::leak(Box::new(OnceLock::new()));
            let _ = OBJ_ARR_CLASSES.write().unwrap().insert(t.into_owned(), once_lock);
            once_lock
        }
    }
}

impl<T: ReferenceType, E: ThrowableType> ObjectArray<T, E> {
    /// Uses JNI `NewObjectArray` to create a new Java object array.
    pub fn new<'env>(env: Env<'env>, size: usize) -> Local<'env, Self> {
        assert!(size <= i32::MAX as usize); // jsize == jint == i32
        let class = T::jni_get_class(env).unwrap().as_raw();
        let size = size as jsize;

        let object = unsafe {
            let env = env.as_raw();
            let fill = null_mut();
            ((**env).v1_2.NewObjectArray)(env, size, class, fill)
        };
        // Only sane exception here is an OOM exception
        env.exception_check::<E>().map_err(|_| "OOM").unwrap();
        unsafe { Local::from_raw(env, object) }
    }

    /// Iterates through object items of the array. See [ObjectArrayIter].
    pub fn iter<'a, 'env>(self: &'a Ref<'env, Self>) -> ObjectArrayIter<'a, 'env, T, E> {
        ObjectArrayIter {
            array: self,
            index: 0,
            length: self.len(),
        }
    }

    /// Uses JNI `NewObjectArray` to create a new Java object array of the exact size, then sets its items
    /// with the iterator of JNI (null?) references.
    pub fn new_from<'env>(env: Env<'env>, elements: impl ExactSizeIterator<Item = impl AsArg<T>>) -> Local<'env, Self> {
        let size = elements.len();
        let array = Self::new(env, size);
        let env = array.env().as_raw();
        for (index, element) in elements.enumerate() {
            assert!(index < size); // Should only be violated by an invalid ExactSizeIterator implementation.
            unsafe { ((**env).v1_2.SetObjectArrayElement)(env, array.as_raw(), index as jsize, element.as_arg()) };
        }
        array
    }

    /// Uses JNI `GetArrayLength` to get the length of the Java array.
    pub fn len(self: &Ref<'_, Self>) -> usize {
        let env = self.env().as_raw();
        unsafe { ((**env).v1_2.GetArrayLength)(env, self.as_raw()) as usize }
    }

    /// Gets a local reference of the object item at given `index` in the array.
    /// Returns `None` if it is null; returns an exception if the index is invalid.
    ///
    /// XXX: Expose this via `std::ops::Index`.
    pub fn get<'env>(self: &Ref<'env, Self>, index: usize) -> Result<Option<Local<'env, T>>, Local<'env, E>> {
        assert!(index <= i32::MAX as usize); // jsize == jint == i32 XXX: Should maybe be treated as an exception?
        let index = index as jsize;
        let env = self.env();
        let result = unsafe {
            let env = env.as_raw();
            ((**env).v1_2.GetObjectArrayElement)(env, self.as_raw(), index)
        };
        env.exception_check()?;
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(unsafe { Local::from_raw(env, result) }))
        }
    }

    /// Sets an element at the given `index` in the array. Returns an exception if the index is invalid.
    ///
    /// XXX: I don't think there's a way to expose this via `std::ops::IndexMut` sadly?
    pub fn set<'env>(self: &Ref<'env, Self>, index: usize, value: impl AsArg<T>) -> Result<(), Local<'env, E>> {
        assert!(index <= i32::MAX as usize); // jsize == jint == i32 XXX: Should maybe be treated as an exception?
        let index = index as jsize;
        let env = self.env();
        unsafe {
            let env = env.as_raw();
            ((**env).v1_2.SetObjectArrayElement)(env, self.as_raw(), index, value.as_arg());
        }
        env.exception_check()
    }
}

/// An iterator over object items of an [ObjectArray]. Local references of object items
/// will be created automatically.
pub struct ObjectArrayIter<'a, 'env, T: ReferenceType, E: ThrowableType> {
    array: &'a Ref<'env, ObjectArray<T, E>>,
    index: usize,
    length: usize,
}

impl<'a, 'env, T: ReferenceType, E: ThrowableType> Iterator for ObjectArrayIter<'a, 'env, T, E> {
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
