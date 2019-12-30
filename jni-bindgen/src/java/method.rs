//! [Java SE 7 &sect; 4.6](https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.6):  Parsing APIs and structures for class methods.

use crate::java::*;
use crate::java::io::*;
pub use field::BasicType;
pub use field::Descriptor as Type;

use bitflags::bitflags;

use std::io::{self, Read};



bitflags! {
    #[derive(Default)]
    /// [Java SE 7 &sect; 4.6](https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.6):  method_info::access_flags values.
    pub struct Flags : u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC        = 0x0001;
        /// Declared `private`; usable only with the defining class.
        const PRIVATE       = 0x0002;
        /// Declared `protectdd`; may be accessed within subclasses.
        const PROTECTED     = 0x0004;
        /// Declared `static`.
        const STATIC        = 0x0008;
        /// Declared `final`; no subclasses allowed.
        const FINAL         = 0x0010;
        /// Declared `syncronized`; invocation is wrapped by a monitor use.
        const SYNCRONIZED   = 0x0020;
        /// A bridge method, generated by the compiler.
        const BRIDGE        = 0x0040;
        /// Declared with variable number of arguments.
        const VARARGS       = 0x0080;
        /// Declared `native`; implemented in a langauge other than Java.
        const NATIVE        = 0x0100;
        /// Declared `abstract`; must not be instantiated.
        const ABSTRACT      = 0x0400;
        /// Declared `strictfp`; floating-point mode is FP-strict.
        const STRICT        = 0x0800;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC     = 0x1000;
    }
}

impl Flags {
    pub fn read(r: &mut impl Read) -> io::Result<Self> {
        Ok(Self::from_bits_truncate(read_u2(r)?))
    }
}



/// [Java SE 7 &sect; 4.6](https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.6):  method_info, minus the trailing attributes
#[derive(Clone, Debug)]
pub struct Method {
    pub flags:      Flags,
    pub name:       String,
    descriptor:     String,
    pub deprecated: bool,
    _incomplete:    (),
}

#[allow(dead_code)]
impl Method {
    pub fn new(flags: Flags, name: String, descriptor: String) -> io::Result<Self> {
        method::Descriptor::new(descriptor.as_str())?;
        Ok(Self {
            flags,
            name,
            descriptor,
            deprecated: false,
            _incomplete: (),
        })
    }

    pub fn descriptor_str(&self) -> &str { self.descriptor.as_str() }
    pub fn descriptor(&self) -> Descriptor { Descriptor::new(self.descriptor.as_str()).unwrap() } // Already validated in new/read_one

    pub fn is_public        (&self) -> bool { self.flags.contains(Flags::PUBLIC         ) }
    pub fn is_private       (&self) -> bool { self.flags.contains(Flags::PRIVATE        ) }
    pub fn is_protected     (&self) -> bool { self.flags.contains(Flags::PROTECTED      ) }
    pub fn is_static        (&self) -> bool { self.flags.contains(Flags::STATIC         ) }
    pub fn is_final         (&self) -> bool { self.flags.contains(Flags::FINAL          ) }
    pub fn is_syncronized   (&self) -> bool { self.flags.contains(Flags::SYNCRONIZED    ) }
    pub fn is_bridge        (&self) -> bool { self.flags.contains(Flags::BRIDGE         ) }
    pub fn is_varargs       (&self) -> bool { self.flags.contains(Flags::VARARGS        ) }
    pub fn is_native        (&self) -> bool { self.flags.contains(Flags::NATIVE         ) }
    pub fn is_abstract      (&self) -> bool { self.flags.contains(Flags::ABSTRACT       ) }
    pub fn is_strict        (&self) -> bool { self.flags.contains(Flags::STRICT         ) }
    pub fn is_synthetic     (&self) -> bool { self.flags.contains(Flags::SYNTHETIC      ) }

    pub fn is_constructor   (&self) -> bool { self.name == "<init>" }
    pub fn is_static_init   (&self) -> bool { self.name == "<clinit>" }

    pub fn access(&self) -> Option<&'static str> {
        if      self.is_private()   { Some("private") }
        else if self.is_protected() { Some("protected") }
        else if self.is_public()    { Some("public") }
        else                        { None }
    }

    pub(crate) fn read_one(read: &mut impl Read, constants: &Constants) -> io::Result<Self> {
        let flags               = Flags::read(read)?;
        let name                = constants.get_utf8(read_u2(read)?)?.to_owned();
        let descriptor          = constants.get_utf8(read_u2(read)?)?.to_owned();
        let attributes_count    = read_u2(read)? as usize;

        method::Descriptor::new(descriptor.as_str())?;

        let mut deprecated      = false;
        for _ in 0..attributes_count {
            match Attribute::read(read, constants)? {
                Attribute::Deprecated { .. } => { deprecated = true; },
                _ => {},
            }
        }

        Ok(Self{
            flags,
            name,
            descriptor,
            deprecated,
            _incomplete:    (),
        })
    }

    pub(crate) fn read_list(read: &mut impl Read, constants: &Constants) -> io::Result<Vec<Self>> {
        let n = read_u2(read)? as usize;
        let mut methods = Vec::with_capacity(n);
        for _ in 0..n {
            methods.push(Self::read_one(read, constants)?);
        }
        Ok(methods)
    }
}



#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Descriptor<'a> {
    string:     &'a str,
    end_paren:  usize,
}

impl<'a> Descriptor<'a> {
    pub fn new(desc: &'a str) -> io::Result<Descriptor<'a>> {
        if !desc.starts_with('(') { return io_data_err!("Method descriptor didn't start with '(': {:?}", desc); }
        let end_paren = if let Some(i) = desc.rfind(')') { i } else { return io_data_err!("Method descriptor doesn't contain a ')' terminating the arguments list: {:?}", desc); };
        let (arguments, return_) = desc.split_at(end_paren);

        Type::from_str(&return_[1..])?; // Skip )
        let mut args = &arguments[1..];  // Skip (
        while !args.is_empty() {
            Type::read_next(&mut args)?;
        }

        Ok(Descriptor { string: desc, end_paren })
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'a str { self.string }
    pub fn return_type(&self) -> Type<'a> { Type::from_str(&self.string[(1+self.end_paren)..]).unwrap() } // Already validated in Descriptor::new
    pub fn arguments(&self) -> ArgumentsIter { ArgumentsIter(&self.string[1..self.end_paren]) }
}

pub struct ArgumentsIter<'a>(&'a str);

impl<'a> Iterator for ArgumentsIter<'a> {
    type Item = Type<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            Some(Type::read_next(&mut self.0).unwrap()) // Already verified in Descriptor::next
        }
    }
}

#[test] fn jni_descriptor_from_str() {
    let d = Descriptor::new("(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;").unwrap();
    assert_eq!(d.return_type(), Type::Single(BasicType::Class(class::Id("android/database/Cursor"))));
    let mut d = d.arguments();
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/net/Uri")))));
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("java/lang/String")))));
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("java/lang/String")))));
    assert_eq!(d.next(), None);
    assert_eq!(d.next(), None);

    let d = Descriptor::new("(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;Landroid/os/CancellationSignal;)Landroid/database/Cursor;").unwrap();
    assert_eq!(d.return_type(), Type::Single(BasicType::Class(class::Id("android/database/Cursor"))));
    let mut d = d.arguments();
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/net/Uri")))));
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("java/lang/String")))));
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("java/lang/String")))));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/os/CancellationSignal")))));
    assert_eq!(d.next(), None);
    assert_eq!(d.next(), None);

    let d = Descriptor::new("(Landroid/net/Uri;[Ljava/lang/String;Landroid/os/Bundle;Landroid/os/CancellationSignal;)Landroid/database/Cursor;").unwrap();
    assert_eq!(d.return_type(), Type::Single(BasicType::Class(class::Id("android/database/Cursor"))));
    let mut d = d.arguments();
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/net/Uri")))));
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/os/Bundle")))));
    assert_eq!(d.next(), Some(Type::Single(BasicType::Class(class::Id("android/os/CancellationSignal")))));
    assert_eq!(d.next(), None);
    assert_eq!(d.next(), None);

    let d = Descriptor::new("([Ljava/lang/String;)V").unwrap();
    assert_eq!(d.return_type(), Type::Single(BasicType::Void));
    let mut d = d.arguments();
    assert_eq!(d.next(), Some(Type::Array { levels: 1, inner: BasicType::Class(class::Id("java/lang/String")) }));
    assert_eq!(d.next(), None);
    assert_eq!(d.next(), None);

    let d = Descriptor::new("()V").unwrap();
    assert_eq!(d.return_type(), Type::Single(BasicType::Void));
    let mut d = d.arguments();
    assert_eq!(d.next(), None);
    assert_eq!(d.next(), None);
}
