use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write;
use std::io;

use super::fields::Field;
use super::known_docs_url::KnownDocsUrl;
use super::methods::Method;
use super::StrEmitter;
use crate::emit_rust::Context;
use crate::identifiers::{FieldMangling, RustIdentifier};
use crate::parser_util::{Class, Id, IdPart};

#[derive(Debug, Default)]
pub(crate) struct StructPaths {
    pub mod_: String,
    pub struct_name: String,
}

impl StructPaths {
    pub(crate) fn new(context: &Context, class: Id) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mod_: Struct::mod_for(context, class)?,
            struct_name: Struct::name_for(context, class)?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Struct {
    pub rust: StructPaths,
    pub java: Class,
}

fn rust_id(id: &str) -> Result<String, Box<dyn Error>> {
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
    pub(crate) fn mod_for(_context: &Context, class: Id) -> Result<String, Box<dyn Error>> {
        let mut buf = String::new();
        for component in class {
            match component {
                IdPart::Namespace(id) => {
                    if !buf.is_empty() {
                        buf.push_str("::");
                    }
                    buf.push_str(&rust_id(id)?);
                }
                IdPart::ContainingClass(_) => {}
                IdPart::LeafClass(_) => {}
            }
        }
        Ok(buf)
    }

    pub(crate) fn name_for(context: &Context, class: Id) -> Result<String, Box<dyn Error>> {
        let rename_to = context
            .config
            .rename_classes
            .get(class.as_str())
            .map(|name| name.as_str())
            .ok_or(());
        let mut buf = String::new();
        for component in class.iter() {
            match component {
                IdPart::Namespace(_) => {}
                IdPart::ContainingClass(id) => write!(&mut buf, "{}_", rust_id(id)?)?,
                IdPart::LeafClass(id) => write!(
                    &mut buf,
                    "{}",
                    rename_to.map(ToString::to_string).or_else(|_| rust_id(id))?
                )?,
            }
        }
        Ok(buf)
    }

    pub(crate) fn new(context: &mut Context, java: Class) -> Result<Self, Box<dyn Error>> {
        let rust = StructPaths::new(context, java.path())?;

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
        let attributes = (if self.java.deprecated() { "#[deprecated] " } else { "" }).to_string();

        if let Some(url) = KnownDocsUrl::from_class(context, self.java.path()) {
            writeln!(out, "/// {} {} {}", visibility, keyword, url)?;
        } else {
            writeln!(out, "/// {} {} {}", visibility, keyword, self.java.path().as_str())?;
        }

        let rust_name = &self.rust.struct_name;
        writeln!(out, "{attributes}{visibility} enum {rust_name}{{}}")?;
        if !self.java.is_static() {
            writeln!(
                out,
                "unsafe impl ::java_spaghetti::ReferenceType for {rust_name} {{\
               \n    fn jni_get_class(__jni_env: ::java_spaghetti::Env) -> &'static ::java_spaghetti::JClass {{\
               \n        Self::__class_global_ref(__jni_env)\
               \n    }}\
               \n}}",
            )?;
        }
        writeln!(
            out,
            "unsafe impl ::java_spaghetti::JniType for {rust_name} {{\
           \n    fn static_with_jni_type<R>(callback: impl FnOnce(&str) -> R) -> R {{\
           \n        callback({})\
           \n    }}\
           \n}}",
            StrEmitter(self.java.path().as_str()),
        )?;

        // recursively visit all superclasses and superinterfaces.
        let mut queue = Vec::new();
        let mut visited = HashSet::new();
        queue.push(self.java.path());
        visited.insert(self.java.path());
        while let Some(path) = queue.pop() {
            let class = context.all_classes.get(path.as_str()).unwrap();
            for path2 in self.java.interfaces().map(|i| Id(i)).chain(class.java.super_path()) {
                if context.all_classes.contains_key(path2.as_str()) && !visited.contains(&path2) {
                    let rust_path = context.java_to_rust_path(path2, &self.rust.mod_).unwrap();
                    writeln!(
                        out,
                        "unsafe impl ::java_spaghetti::AssignableTo<{rust_path}> for {rust_name} {{}}"
                    )?;
                    queue.push(path2);
                    visited.insert(path2);
                }
            }
        }

        writeln!(out, "impl {rust_name} {{")?;

        writeln!(
            out,
            "\
          \nfn __class_global_ref(__jni_env: ::java_spaghetti::Env) -> &'static ::java_spaghetti::JClass {{\
          \n    static _CLASS: ::std::sync::OnceLock<::java_spaghetti::JClass> = ::std::sync::OnceLock::new();\
          \n    _CLASS.get_or_init(|| unsafe {{ __jni_env.require_class({}) }})\
          \n}}",
            StrEmitter(self.java.path().as_str()),
        )?;

        let mut id_repeats = HashMap::new();

        let mut methods: Vec<Method> = self
            .java
            .methods()
            .map(|m| Method::new(context, &self.java, m))
            .collect();
        let mut fields: Vec<Field> = self.java.fields().map(|f| Field::new(context, &self.java, f)).collect();

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
