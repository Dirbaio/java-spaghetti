// Migrated from <https://docs.rs/jreflection/0.0.11/src/jreflection/class.rs.html>.

use std::fmt::Write;

/// Owned Java class binary name (internal form).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdBuf(String);

impl IdBuf {
    pub fn new(s: String) -> Self {
        Self(s)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub fn as_id(&self) -> Id {
        Id(self.0.as_str())
    }
    #[allow(dead_code)]
    pub fn iter(&self) -> IdIter {
        IdIter::new(self.0.as_str())
    }
}

// XXX: This should really be `#[repr(transparent)] pub struct Id(str);`...
// Also, patterns apparently can't handle Id::new(...) even when it's a const fn.

/// Borrowed Java class binary name (internal form).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<'a>(pub &'a str);

impl<'a> Id<'a> {
    pub fn as_str(&self) -> &'a str {
        self.0
    }
    pub fn iter(&self) -> IdIter<'a> {
        IdIter::new(self.0)
    }
}

impl<'a> IntoIterator for Id<'a> {
    type Item = IdPart<'a>;
    type IntoIter = IdIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdPart<'a> {
    Namespace(&'a str),
    ContainingClass(&'a str),
    LeafClass(&'a str),
}

/// Iterates through names of namespaces, superclasses and the "leaf" class
/// in the Java class binary name.
pub struct IdIter<'a> {
    rest: &'a str,
}

impl<'a> IdIter<'a> {
    pub fn new(path: &'a str) -> Self {
        IdIter { rest: path }
    }
}

impl<'a> Iterator for IdIter<'a> {
    type Item = IdPart<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(slash) = self.rest.find('/') {
            let (namespace, rest) = self.rest.split_at(slash);
            self.rest = &rest[1..];
            return Some(IdPart::Namespace(namespace));
        }

        if let Some(dollar) = self.rest.find('$') {
            let (class, rest) = self.rest.split_at(dollar);
            self.rest = &rest[1..];
            return Some(IdPart::ContainingClass(class));
        }

        if !self.rest.is_empty() {
            let class = self.rest;
            self.rest = "";
            return Some(IdPart::LeafClass(class));
        }

        None
    }
}

#[test]
fn id_iter_test() {
    assert_eq!(Id("").iter().collect::<Vec<_>>(), &[]);

    assert_eq!(Id("Bar").iter().collect::<Vec<_>>(), &[IdPart::LeafClass("Bar"),]);

    assert_eq!(
        Id("java/foo/Bar").iter().collect::<Vec<_>>(),
        &[
            IdPart::Namespace("java"),
            IdPart::Namespace("foo"),
            IdPart::LeafClass("Bar"),
        ]
    );

    assert_eq!(
        Id("java/foo/Bar$Inner").iter().collect::<Vec<_>>(),
        &[
            IdPart::Namespace("java"),
            IdPart::Namespace("foo"),
            IdPart::ContainingClass("Bar"),
            IdPart::LeafClass("Inner"),
        ]
    );

    assert_eq!(
        Id("java/foo/Bar$Inner$MoreInner").iter().collect::<Vec<_>>(),
        &[
            IdPart::Namespace("java"),
            IdPart::Namespace("foo"),
            IdPart::ContainingClass("Bar"),
            IdPart::ContainingClass("Inner"),
            IdPart::LeafClass("MoreInner"),
        ]
    );
}

/// Newtype for `cafebabe::descriptors::ClassName`.
///
/// XXX: cannot get the original string from `cafebabe::descriptors::ClassName`; the binary
/// name is split into `UnqualifiedSegment`s, not caring about Java-specific nested classes.
/// See <https://github.com/staktrace/cafebabe/issues/52>.
#[derive(Clone, Copy, Debug)]
pub struct ClassName<'a> {
    inner: &'a cafebabe::descriptors::ClassName<'a>,
}

impl<'a> From<&'a cafebabe::descriptors::ClassName<'a>> for ClassName<'a> {
    fn from(value: &'a cafebabe::descriptors::ClassName<'a>) -> Self {
        Self { inner: value }
    }
}

impl<'a> std::ops::Deref for ClassName<'a> {
    type Target = cafebabe::descriptors::ClassName<'a>;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl std::fmt::Display for ClassName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut segs = self.segments.iter();
        f.write_str(segs.next().unwrap().name.as_ref())?;
        for seg in segs {
            f.write_char('/')?;
            f.write_str(seg.name.as_ref())?;
        }
        Ok(())
    }
}

impl<'a> From<&ClassName<'a>> for IdBuf {
    fn from(value: &ClassName<'a>) -> Self {
        Self::new(value.to_string())
    }
}

impl<'a> From<&cafebabe::descriptors::ClassName<'a>> for IdBuf {
    fn from(value: &cafebabe::descriptors::ClassName<'a>) -> Self {
        Self::new(ClassName::from(value).to_string())
    }
}

impl<'a> ClassName<'a> {
    pub fn iter<'s>(&'s self) -> ClassNameIter<'a> {
        let segments = &self.inner.segments;
        if segments.len() > 1 {
            ClassNameIter::RestPath(segments)
        } else {
            let classes = segments.last().map(|s| s.name.as_ref()).unwrap_or("");
            ClassNameIter::RestClasses(IdIter::new(classes))
        }
    }
}

impl<'a> IntoIterator for ClassName<'a> {
    type Item = IdPart<'a>;
    type IntoIter = ClassNameIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub enum ClassNameIter<'a> {
    RestPath(&'a [cafebabe::descriptors::UnqualifiedSegment<'a>]),
    RestClasses(IdIter<'a>),
}

impl<'a> Iterator for ClassNameIter<'a> {
    type Item = IdPart<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::RestPath(segments) => {
                // `segments.len() > 1` must be true at here
                let namespace = IdPart::Namespace(&segments[0].name);
                *self = if segments.len() - 1 > 1 {
                    Self::RestPath(&segments[1..])
                } else {
                    Self::RestClasses(IdIter::new(&segments.last().unwrap().name))
                };
                Some(namespace)
            }
            Self::RestClasses(id_iter) => id_iter.next(),
        }
    }
}

#[allow(unused)]
pub trait IterableId<'a>: IntoIterator<Item = IdPart<'a>> + Copy {
    fn is_string_class(self) -> bool {
        let mut iter = self.into_iter();
        iter.next() == Some(IdPart::Namespace("java"))
            && iter.next() == Some(IdPart::Namespace("lang"))
            && iter.next() == Some(IdPart::LeafClass("String"))
            && iter.next().is_none()
    }
}

impl<'a> IterableId<'a> for Id<'a> {}
impl<'a> IterableId<'a> for ClassName<'a> {}
