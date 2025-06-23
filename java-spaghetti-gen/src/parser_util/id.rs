// Migrated from <https://docs.rs/jreflection/0.0.11/src/jreflection/class.rs.html>.

// XXX: This may really be `#[repr(transparent)] pub struct Id(str);`...
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

    pub fn is_string_class(&self) -> bool {
        let mut iter = self.into_iter();
        iter.next() == Some(IdPart::Namespace("java"))
            && iter.next() == Some(IdPart::Namespace("lang"))
            && iter.next() == Some(IdPart::LeafClass("String"))
            && iter.next().is_none()
    }
}

impl<'r, 'a: 'r> From<&'r cafebabe::descriptors::ClassName<'a>> for Id<'r> {
    fn from(value: &'r cafebabe::descriptors::ClassName<'a>) -> Self {
        Self(value.as_ref())
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
