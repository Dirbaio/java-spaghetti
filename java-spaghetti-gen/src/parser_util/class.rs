use std::marker::PhantomPinned;
use std::pin::Pin;

pub use cafebabe::ClassAccessFlags;
use cafebabe::attributes::AttributeData;
use cafebabe::descriptors::ClassName;

use super::Id;

#[derive(Debug)]
pub struct JavaClass {
    #[allow(unused)]
    raw_bytes: Pin<Box<(Vec<u8>, PhantomPinned)>>,
    inner: cafebabe::ClassFile<'static>,
}

impl JavaClass {
    pub fn read(raw_bytes: Vec<u8>) -> Result<Self, cafebabe::ParseError> {
        let pinned = Box::pin((raw_bytes, PhantomPinned));
        // SAFETY: `get<'a>(&'a self)` restricts the lifetime parameter of
        // the returned referenced `ClassFile`.
        let fake_static = unsafe { std::slice::from_raw_parts(pinned.0.as_ptr(), pinned.0.len()) };
        let inner = cafebabe::parse_class(fake_static)?;
        Ok(Self {
            raw_bytes: pinned,
            inner,
        })
    }

    // It is probably not possible to implement `Deref` safely.
    pub fn get<'a>(&'a self) -> &'a cafebabe::ClassFile<'a> {
        // SAFETY: casts `self.inner` into `cafebabe::ClassFile<'a>` forcefully.
        // `cafebabe::parse_class` takes immutable &'a [u8], why is the returned
        // `ClassFile<'a>` invariant over `'a`?
        unsafe { &*(&raw const (self.inner)).cast() }
    }

    fn flags(&self) -> ClassAccessFlags {
        self.get().access_flags
    }

    pub fn is_public(&self) -> bool {
        self.flags().contains(ClassAccessFlags::PUBLIC)
    }
    pub fn is_final(&self) -> bool {
        self.flags().contains(ClassAccessFlags::FINAL)
    }
    pub fn is_static(&self) -> bool {
        (self.flags().bits() & 0x0008) != 0
    }
    #[allow(unused)]
    pub fn is_super(&self) -> bool {
        self.flags().contains(ClassAccessFlags::SUPER)
    }
    pub fn is_interface(&self) -> bool {
        self.flags().contains(ClassAccessFlags::INTERFACE)
    }
    #[allow(unused)]
    pub fn is_abstract(&self) -> bool {
        self.flags().contains(ClassAccessFlags::ABSTRACT)
    }
    #[allow(unused)]
    pub fn is_synthetic(&self) -> bool {
        self.flags().contains(ClassAccessFlags::SYNTHETIC)
    }
    #[allow(unused)]
    pub fn is_annotation(&self) -> bool {
        self.flags().contains(ClassAccessFlags::ANNOTATION)
    }
    pub fn is_enum(&self) -> bool {
        self.flags().contains(ClassAccessFlags::ENUM)
    }

    pub fn path(&self) -> Id<'_> {
        Id(self.get().this_class.as_ref())
    }

    pub fn super_path(&self) -> Option<Id<'_>> {
        self.get().super_class.as_ref().map(|class| Id(class))
    }

    pub fn interfaces(&self) -> std::slice::Iter<'_, ClassName<'_>> {
        self.get().interfaces.iter()
    }

    pub fn fields(&self) -> std::slice::Iter<'_, cafebabe::FieldInfo<'_>> {
        self.get().fields.iter()
    }

    pub fn methods(&self) -> std::slice::Iter<'_, cafebabe::MethodInfo<'_>> {
        self.get().methods.iter()
    }

    pub fn deprecated(&self) -> bool {
        self.get()
            .attributes
            .iter()
            .any(|attr| matches!(attr.data, AttributeData::Deprecated))
    }
}
