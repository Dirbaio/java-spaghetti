use std::fmt::Write;

use cafebabe::attributes::AttributeData;
use cafebabe::constant_pool::LiteralConstant;
use cafebabe::descriptors::{FieldDescriptor, FieldType};
use cafebabe::FieldAccessFlags;

use super::ClassName;

#[derive(Clone, Copy, Debug)]
pub struct JavaField<'a> {
    java: &'a cafebabe::FieldInfo<'a>,
}

impl<'a> From<&'a cafebabe::FieldInfo<'a>> for JavaField<'a> {
    fn from(value: &'a cafebabe::FieldInfo<'a>) -> Self {
        Self { java: value }
    }
}

impl<'a> std::ops::Deref for JavaField<'a> {
    type Target = cafebabe::FieldInfo<'a>;
    fn deref(&self) -> &Self::Target {
        self.java
    }
}

impl<'a> JavaField<'a> {
    pub fn name<'s>(&'s self) -> &'a str {
        self.java.name.as_ref()
    }

    pub fn is_public(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::PUBLIC)
    }
    pub fn is_private(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::PRIVATE)
    }
    pub fn is_protected(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::PROTECTED)
    }
    pub fn is_static(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::STATIC)
    }
    pub fn is_final(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::FINAL)
    }
    pub fn is_volatile(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::VOLATILE)
    }
    #[allow(unused)]
    pub fn is_transient(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::TRANSIENT)
    }
    #[allow(unused)]
    pub fn is_synthetic(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::SYNTHETIC)
    }
    #[allow(unused)]
    pub fn is_enum(&self) -> bool {
        self.access_flags.contains(FieldAccessFlags::ENUM)
    }

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

    pub fn is_constant(&self) -> bool {
        self.is_static()
            && self.is_final()
            && self
                .attributes
                .iter()
                .any(|attr| matches!(&attr.data, AttributeData::ConstantValue(_)))
    }

    pub fn constant<'s>(&'s self) -> Option<LiteralConstant<'a>> {
        if !self.is_static() || !self.is_final() {
            return None;
        }
        self.attributes.iter().find_map(|attr| {
            if let AttributeData::ConstantValue(c) = &attr.data {
                Some(c.clone())
            } else {
                None
            }
        })
    }

    pub fn deprecated(&self) -> bool {
        self.attributes
            .iter()
            .any(|attr| matches!(attr.data, AttributeData::Deprecated))
    }

    pub fn descriptor<'s>(&'s self) -> &'a FieldDescriptor<'a> {
        &self.java.descriptor
    }
}

// XXX: cannot get the original string from `cafebabe::descriptors::FieldDescriptor`.
// <https://github.com/staktrace/cafebabe/issues/52>
pub struct FieldSigWriter<'a>(pub &'a FieldDescriptor<'a>);

impl std::fmt::Display for FieldSigWriter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let descriptor = self.0;
        for _ in 0..descriptor.dimensions {
            f.write_char('[')?;
        }
        if let FieldType::Object(class_name) = &descriptor.field_type {
            f.write_char('L')?;
            ClassName::from(class_name).fmt(f)?;
            f.write_char(';')
        } else {
            let ch = match descriptor.field_type {
                FieldType::Boolean => 'Z',
                FieldType::Byte => 'B',
                FieldType::Char => 'C',
                FieldType::Short => 'S',
                FieldType::Integer => 'I',
                FieldType::Long => 'J',
                FieldType::Float => 'F',
                FieldType::Double => 'D',
                _ => unreachable!(),
            };
            f.write_char(ch)
        }
    }
}
