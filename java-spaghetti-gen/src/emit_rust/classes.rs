use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::cstring;
use super::fields::Field;
use super::known_docs_url::KnownDocsUrl;
use super::methods::Method;
use crate::emit_rust::Context;
use crate::identifiers::{FieldMangling, RustIdentifier};
use crate::parser_util::{Id, IdPart, JavaClass};

#[derive(Debug, Default)]
pub(crate) struct StructPaths {
    pub mod_: String,
    pub struct_name: String,
}

impl StructPaths {
    pub(crate) fn new(context: &Context, class: Id) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            mod_: Class::mod_for(context, class)?,
            struct_name: Class::name_for(context, class)?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Class {
    pub rust: StructPaths,
    pub java: JavaClass,
}

fn rust_id(id: &str) -> Result<String, Box<dyn Error>> {
    Ok(match RustIdentifier::from_str(id) {
        RustIdentifier::Identifier(id) => id,
        RustIdentifier::KeywordRawSafe(id) => id,
        RustIdentifier::KeywordUnderscorePostfix(id) => id,
        RustIdentifier::NonIdentifier(id) => io_data_err!(
            "Unable to add_class(): java identifier {:?} has no rust equivalent (yet?)",
            id
        )?,
    })
}

impl Class {
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

    pub(crate) fn new(context: &mut Context, java: JavaClass) -> Result<Self, Box<dyn Error>> {
        let rust = StructPaths::new(context, java.path())?;

        Ok(Self { rust, java })
    }

    pub(crate) fn write(&self, context: &Context) -> anyhow::Result<TokenStream> {
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

        let visibility = if self.java.is_public() { quote!(pub) } else { quote!() };
        let attributes = match self.java.deprecated() {
            true => quote!(#[deprecated] ),
            false => quote!(),
        };

        let docs = match KnownDocsUrl::from_class(context, self.java.path()) {
            Some(url) => format!("{} {} {}", visibility, keyword, url),
            None => format!("{} {} {}", visibility, keyword, self.java.path().as_str()),
        };

        let rust_name = format_ident!("{}", &self.rust.struct_name);

        let referencetype_impl = match self.java.is_static() {
            true => quote!(),
            false => quote!(unsafe impl ::java_spaghetti::ReferenceType for #rust_name {}),
        };

        let mut out = TokenStream::new();

        let java_path = cstring(self.java.path().as_str());

        out.extend(quote!(
            #[doc = #docs]
            #attributes
            #visibility enum #rust_name {}

            #referencetype_impl

            unsafe impl ::java_spaghetti::JniType for #rust_name {
                fn static_with_jni_type<R>(callback: impl FnOnce(&::std::ffi::CStr) -> R) -> R {
                    callback(#java_path)
                }
            }
        ));

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
                    out.extend(quote!(
                        unsafe impl ::java_spaghetti::AssignableTo<#rust_path> for #rust_name {}
                    ));
                    queue.push(path2);
                    visited.insert(path2);
                }
            }
        }

        let mut contents = TokenStream::new();

        let object = context
            .java_to_rust_path(Id("java/lang/Object"), &self.rust.mod_)
            .unwrap();

        let class = cstring(self.java.path().as_str());

        contents.extend(quote!(
            fn __class_global_ref(__jni_env: ::java_spaghetti::Env) -> ::java_spaghetti::sys::jobject {
                static __CLASS: ::std::sync::OnceLock<::java_spaghetti::Global<#object>> = ::std::sync::OnceLock::new();
                __CLASS
                    .get_or_init(|| unsafe {
                        ::java_spaghetti::Local::from_raw(__jni_env, __jni_env.require_class(#class)).as_global()
                    })
                    .as_raw()
            }
        ));

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

            let res = method.emit(context, &self.rust.mod_).unwrap();
            contents.extend(res);
        }

        for field in &mut fields {
            let res = field.emit(context, &self.rust.mod_).unwrap();
            contents.extend(res);
        }

        out.extend(quote!(impl #rust_name { #contents }));

        if context.proxy_included(&self.java.path().as_str()) {
            out.extend(self.write_proxy(context, &methods)?);
        }

        Ok(out)
    }
}
