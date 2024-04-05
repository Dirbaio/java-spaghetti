use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write;
use std::io;

use jreflection::class;

use super::fields::Field;
use super::known_docs_url::KnownDocsUrl;
use super::methods::Method;
use crate::emit_rust::Context;
use crate::identifiers::{FieldMangling, RustIdentifier};

#[derive(Debug, Default)]
pub(crate) struct StructPaths {
    pub mod_: String,
    pub struct_name: String,
}

impl StructPaths {
    pub(crate) fn new(context: &Context, class: class::Id) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mod_: Struct::mod_for(context, class)?,
            struct_name: Struct::name_for(context, class)?,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct Struct {
    pub rust: StructPaths,
    pub java: jreflection::Class,
}

fn rust_id(id: &str) -> Result<&str, Box<dyn Error>> {
    Ok(match RustIdentifier::from_str(id) {
        RustIdentifier::Identifier(id) => id,
        RustIdentifier::KeywordRawSafe(id) => id,
        RustIdentifier::KeywordUnderscorePostfix(id) => id,
        RustIdentifier::NonIdentifier(id) => io_data_err!(
            "Unable to add_struct(): java identifier {:?} has no rust equivalent (yet?)",
            id
        )?,
    })
}

impl Struct {
    pub(crate) fn mod_for(_context: &Context, class: class::Id) -> Result<String, Box<dyn Error>> {
        let mut buf = String::new();
        for component in class.iter() {
            match component {
                class::IdPart::Namespace(id) => {
                    if !buf.is_empty() {
                        buf.push_str("::");
                    }
                    buf.push_str(rust_id(id)?);
                }
                class::IdPart::ContainingClass(_) => {}
                class::IdPart::LeafClass(_) => {}
            }
        }
        Ok(buf)
    }

    pub(crate) fn name_for(context: &Context, class: class::Id) -> Result<String, Box<dyn Error>> {
        let rename_to = context
            .config
            .rename_classes
            .get(class.as_str())
            .map(|name| name.as_str())
            .ok_or(());
        let mut buf = String::new();
        for component in class.iter() {
            match component {
                class::IdPart::Namespace(_) => {}
                class::IdPart::ContainingClass(id) => write!(&mut buf, "{}_", rust_id(id)?)?,
                class::IdPart::LeafClass(id) => write!(&mut buf, "{}", rename_to.or_else(|_| rust_id(id))?)?,
            }
        }
        Ok(buf)
    }

    pub(crate) fn new(context: &mut Context, java: jreflection::Class) -> Result<Self, Box<dyn Error>> {
        let rust = StructPaths::new(context, java.path.as_id())?;

        Ok(Self { rust, java })
    }

    pub(crate) fn write(&self, context: &Context, out: &mut impl io::Write) -> io::Result<()> {
        writeln!(out)?;

        // Ignored access_flags: SUPER, SYNTHETIC, ANNOTATION, ABSTRACT

        let keyword = if self.java.is_interface() {
            "interface"
        } else if self.java.is_enum() {
            "enum"
        } else if self.java.is_static() {
            "static class"
        } else if self.java.is_final() {
            "final class"
        } else {
            "class"
        };

        let visibility = if self.java.is_public() { "pub" } else { "" };
        let attributes = (if self.java.deprecated { "#[deprecated] " } else { "" }).to_string();

        if let Some(url) = KnownDocsUrl::from_class(context, self.java.path.as_id()) {
            writeln!(out, "/// {} {} {}", visibility, keyword, url)?;
        } else {
            writeln!(out, "/// {} {} {}", visibility, keyword, self.java.path.as_str())?;
        }

        let rust_name = &self.rust.struct_name;
        if self.java.is_static() {
            writeln!(out, "{attributes}{visibility} enum {rust_name}{{}}")?;
        } else {
            writeln!(
                out,
                "#[repr(transparent)] {attributes}{visibility} struct {rust_name}(pub(crate) ::java_spaghetti::ObjectAndEnv);
                unsafe impl ::java_spaghetti::ReferenceType for {rust_name} {{}}
                unsafe impl ::java_spaghetti::AsJValue for {rust_name} {{ fn as_jvalue(&self) -> ::java_spaghetti::sys::jvalue {{ ::java_spaghetti::sys::jvalue {{ l: self.0.object }} }} }}
                ",
            )?;
        }
        writeln!(
            out,
            "unsafe impl ::java_spaghetti::JniType for {rust_name} {{
                fn static_with_jni_type<R>(callback: impl FnOnce(&str) -> R) -> R {{
                    callback({:?})
                }}
            }}",
            self.java.path.as_str().to_string() + "\0",
        )?;

        // recursively visit all superclasses and superinterfaces.
        let mut queue = Vec::new();
        let mut visited = HashSet::new();
        queue.push(self.java.path.clone());
        visited.insert(self.java.path.clone());
        while let Some(path) = queue.pop() {
            let class = context.all_classes.get(path.as_str()).unwrap();
            for path2 in self.java.interfaces.iter().chain(class.java.super_path.as_ref()) {
                if context.all_classes.contains_key(path2.as_str()) && !visited.contains(path2) {
                    let rust_path = context.java_to_rust_path(path2.as_id(), &self.rust.mod_).unwrap();
                    writeln!(
                        out,
                        "unsafe impl ::java_spaghetti::AssignableTo<{rust_path}> for {rust_name} {{}}"
                    )?;
                    queue.push(path2.clone());
                    visited.insert(path2.clone());
                }
            }
        }

        if let Some(super_path) = self.java.super_path.as_ref() {
            let super_path = context.java_to_rust_path(super_path.as_id(), &self.rust.mod_).unwrap();
            writeln!(
                out,
                "impl ::std::ops::Deref for {rust_name} {{
                    type Target = {super_path};
                    fn deref(&self) -> &Self::Target {{
                        unsafe {{ &*(self as *const Self as *const Self::Target) }}
                    }}
                }}"
            )?;
        };

        for interface in &self.java.interfaces {
            if !context.all_classes.contains_key(interface.as_str()) {
                continue;
            }
            let implements_path = context.java_to_rust_path(interface.as_id(), &self.rust.mod_).unwrap();
            writeln!(
                out,
                "impl ::std::convert::AsRef<{implements_path}> for {rust_name} {{
                    fn as_ref(&self) -> &{implements_path} {{
                        unsafe {{ &*(self as *const Self as *const {implements_path}) }}
                    }}
                }}"
            )?;
        }
        writeln!(out, "impl {rust_name} {{")?;

        let mut id_repeats = HashMap::new();

        let mut methods: Vec<Method> = self
            .java
            .methods
            .iter()
            .map(|m| Method::new(context, &self.java, m))
            .collect();
        let mut fields: Vec<Field> = self
            .java
            .fields
            .iter()
            .map(|f| Field::new(context, &self.java, f))
            .collect();

        for method in &methods {
            if !method.java.is_public() {
                continue;
            } // Skip private/protected methods
            if let Some(name) = method.rust_name() {
                *id_repeats.entry(name.to_owned()).or_insert(0) += 1;
            }
        }

        for field in &fields {
            if !field.java.is_public() {
                continue;
            } // Skip private/protected fields
            match field.rust_names.as_ref() {
                Ok(FieldMangling::ConstValue(name, _)) => {
                    *id_repeats.entry(name.to_owned()).or_insert(0) += 1;
                }
                Ok(FieldMangling::GetSet(get, set)) => {
                    *id_repeats.entry(get.to_owned()).or_insert(0) += 1;
                    *id_repeats.entry(set.to_owned()).or_insert(0) += 1;
                }
                Err(_) => {}
            }
        }

        for method in &mut methods {
            if let Some(name) = method.rust_name() {
                let repeats = *id_repeats.get(name).unwrap_or(&0);
                let overloaded = repeats > 1;
                if overloaded {
                    method.set_mangling_style(context.config.codegen.method_naming_style_collision);
                }
            }

            method.emit(context, &self.rust.mod_, out)?;
        }

        for field in &mut fields {
            field.emit(context, &self.rust.mod_, out)?;
        }

        writeln!(out, "}}")?;
        Ok(())
    }
}
