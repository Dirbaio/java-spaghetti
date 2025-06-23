use cafebabe::MethodAccessFlags;
use cafebabe::attributes::AttributeData;
use cafebabe::descriptors::MethodDescriptor;

pub struct JavaMethod<'a> {
    java: &'a cafebabe::MethodInfo<'a>,
}

impl<'a> From<&'a cafebabe::MethodInfo<'a>> for JavaMethod<'a> {
    fn from(value: &'a cafebabe::MethodInfo<'a>) -> Self {
        Self { java: value }
    }
}

impl<'a> std::ops::Deref for JavaMethod<'a> {
    type Target = cafebabe::MethodInfo<'a>;
    fn deref(&self) -> &Self::Target {
        self.java
    }
}

impl<'a> JavaMethod<'a> {
    pub fn name<'s>(&'s self) -> &'a str {
        self.java.name.as_ref()
    }

    pub fn is_public(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::PUBLIC)
    }
    #[allow(unused)]
    pub fn is_private(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::PRIVATE)
    }
    #[allow(unused)]
    pub fn is_protected(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::PROTECTED)
    }
    pub fn is_static(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STATIC)
    }
    #[allow(unused)]
    pub fn is_final(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::FINAL)
    }
    #[allow(unused)]
    pub fn is_synchronized(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::SYNCHRONIZED)
    }
    pub fn is_bridge(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::BRIDGE)
    }
    pub fn is_varargs(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::VARARGS)
    }
    #[allow(unused)]
    pub fn is_native(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::NATIVE)
    }
    #[allow(unused)]
    pub fn is_abstract(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::ABSTRACT)
    }
    #[allow(unused)]
    pub fn is_strict(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::STRICT)
    }
    #[allow(unused)]
    pub fn is_synthetic(&self) -> bool {
        self.access_flags.contains(MethodAccessFlags::SYNTHETIC)
    }

    pub fn is_constructor(&self) -> bool {
        self.name() == "<init>"
    }
    pub fn is_static_init(&self) -> bool {
        self.name() == "<clinit>"
    }

    #[allow(unused)]
    pub fn access(&self) -> Option<&'static str> {
        if self.is_private() {
            Some("private")
        } else if self.is_protected() {
            Some("protected")
        } else if self.is_public() {
            Some("public")
        } else {
            None
        }
    }

    pub fn deprecated(&self) -> bool {
        self.attributes
            .iter()
            .any(|attr| matches!(attr.data, AttributeData::Deprecated))
    }

    pub fn descriptor<'s>(&'s self) -> &'a MethodDescriptor<'a> {
        &self.java.descriptor
    }
}
