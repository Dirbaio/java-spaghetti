use std::collections::HashSet;
use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::cstring;
use super::fields::Field;
use super::known_docs_url::KnownDocsUrl;
use super::methods::Method;
use crate::emit::Context;
use crate::identifiers::{FieldMangling, rust_ident};
use crate::parser_util::{Id, IdPart, JavaClass};

#[derive(Debug, Default)]
pub(crate) struct StructPaths {
    pub mod_: String,
    pub struct_name: String,
}

impl StructPaths {
    pub(crate) fn new(class: Id) -> Result<Self, anyhow::Error> {
        Ok(Self {
            mod_: Class::mod_for(class)?,
            struct_name: Class::name_for(class)?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Class {
    pub rust: StructPaths,
    pub java: JavaClass,
}

impl Class {
    pub(crate) fn mod_for(class: Id) -> Result<String, anyhow::Error> {
        let mut buf = String::new();
        for component in class {
            match component {
                IdPart::Namespace(id) => {
                    if !buf.is_empty() {
                        buf.push_str("::");
                    }
                    buf.push_str(&rust_ident(id)?);
                }
                IdPart::ContainingClass(_) => {}
                IdPart::LeafClass(_) => {}
            }
        }
        Ok(buf)
    }

    pub(crate) fn name_for(class: Id) -> Result<String, anyhow::Error> {
        let mut buf = String::new();
        for component in class.iter() {
            match component {
                IdPart::Namespace(_) => {}
                IdPart::ContainingClass(id) => write!(&mut buf, "{}_", rust_ident(id)?)?,
                IdPart::LeafClass(id) => write!(&mut buf, "{}", rust_ident(id)?)?,
            }
        }
        Ok(buf)
    }

    pub(crate) fn new(java: JavaClass) -> Result<Self, anyhow::Error> {
        let rust = StructPaths::new(java.path())?;

        Ok(Self { rust, java })
    }

    pub(crate) fn write(&self, context: &Context) -> anyhow::Result<TokenStream> {
        let cc = context.config.resolve_class(self.java.path().as_str());

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

        let visibility = if self.java.is_public() || cc.include_private_classes {
            quote!(pub)
        } else {
            quote!()
        };
        let attributes = match self.java.deprecated() {
            true => quote!(#[deprecated] ),
            false => quote!(),
        };

        let docs = match KnownDocsUrl::from_class(&cc, self.java.path()) {
            Some(url) => format!("{keyword} {url}"),
            None => format!("{keyword} {}", self.java.path().as_str()),
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

        let mut methods: Vec<Method> = self
            .java
            .methods()
            .map(|m| Method::new(&self.java, m))
            .filter(|m| (m.java.is_public() || cc.include_private_methods) && !m.java.is_bridge())
            .collect();
        let mut fields: Vec<Field> = self
            .java
            .fields()
            .map(|f| Field::new(&self.java, f))
            .filter(|f| f.java.is_public() || cc.include_private_fields)
            .collect();

        self.resolve_collisions(&mut methods, &fields)?;

        for method in &mut methods {
            let res = method.emit(context, &cc, &self.rust.mod_).unwrap();
            contents.extend(res);
        }

        for field in &mut fields {
            let res = field.emit(context, &cc, &self.rust.mod_).unwrap();
            contents.extend(res);
        }

        out.extend(quote!(impl #rust_name { #contents }));

        if cc.proxy {
            out.extend(self.write_proxy(context, &methods)?);
        }

        Ok(out)
    }

    /// Fills the name_counts map with all field and method names
    fn fill_name_counts(&self, methods: &[Method], fields: &[Field]) -> std::collections::HashMap<String, usize> {
        use std::collections::HashMap;

        let mut name_counts = HashMap::new();

        // Fill name_counts with all names from fields
        for field in fields {
            match field.rust_names.as_ref() {
                Ok(FieldMangling::ConstValue(name, _)) => {
                    *name_counts.entry(name.clone()).or_insert(0) += 1;
                }
                Ok(FieldMangling::GetSet(get, set)) => {
                    *name_counts.entry(get.clone()).or_insert(0) += 1;
                    *name_counts.entry(set.clone()).or_insert(0) += 1;
                }
                Err(_) => {}
            }
        }

        // Fill name_counts with all names from methods
        for method in methods.iter() {
            if let Some(name) = method.rust_name() {
                *name_counts.entry(name.to_owned()).or_insert(0) += 1;
            }
        }

        name_counts
    }

    /// Resolves method name collisions using a hardcoded fallback strategy:
    /// Java -> JavaShortSignature -> JavaLongSignature
    /// Only colliding methods are upgraded to the next mangling level.
    fn resolve_collisions(&self, methods: &mut [Method], fields: &[Field]) -> anyhow::Result<()> {
        use crate::identifiers::MethodManglingStyle;

        // Start with all methods using Java style
        for method in methods.iter_mut() {
            method.set_mangling_style(MethodManglingStyle::Java);
        }

        for style in [
            MethodManglingStyle::JavaShortSignature,
            MethodManglingStyle::JavaLongSignature,
        ] {
            let name_counts = self.fill_name_counts(methods, fields);

            let has_collisions = name_counts.values().any(|&count| count >= 2);
            if !has_collisions {
                return Ok(()); // All names are unique, we're done
            }

            // Upgrade methods that have collisions to the next mangling style
            for method in methods.iter_mut() {
                if let Some(name) = method.rust_name()
                    && name_counts.get(name).unwrap_or(&0) >= &2
                {
                    method.set_mangling_style(style);
                }
            }
        }

        let name_counts = self.fill_name_counts(methods, fields);
        let has_collisions = name_counts.values().any(|&count| count >= 2);
        if !has_collisions {
            return Ok(()); // All names are unique, we're done
        }

        // we still have collisions, return an error
        let conflicting_names: Vec<String> = name_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(name, _)| name)
            .collect();

        Err(anyhow::anyhow!(
            "Unable to resolve method name collisions in class {}: {}",
            self.java.path().as_str(),
            conflicting_names.join(", ")
        ))
    }
}
