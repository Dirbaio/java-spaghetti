use anyhow::anyhow;
use cafebabe::constant_pool::LiteralConstant;
use cafebabe::descriptors::{FieldDescriptor, FieldType};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use super::cstring;
use super::known_docs_url::KnownDocsUrl;
use crate::emit_rust::Context;
use crate::identifiers::{FieldMangling, IdentifierManglingError};
use crate::parser_util::{emit_field_descriptor, ClassName, IdBuf, IterableId, JavaClass, JavaField};

pub struct Field<'a> {
    pub class: &'a JavaClass,
    pub java: JavaField<'a>,
    pub rust_names: Result<FieldMangling<'a>, IdentifierManglingError>,
    pub ignored: bool,
}

impl<'a> Field<'a> {
    pub fn new(context: &Context, class: &'a JavaClass, java: &'a cafebabe::FieldInfo<'a>) -> Self {
        let java_class_field = format!("{}\x1f{}", class.path().as_str(), &java.name);
        let ignored = context.config.ignore_class_fields.contains(&java_class_field);
        let renamed_to = context
            .config
            .rename_class_fields
            .get(&java_class_field)
            .map(|s| s.as_str());

        Self {
            class,
            java: JavaField::from(java),
            rust_names: context
                .config
                .codegen
                .field_naming_style
                .mangle(JavaField::from(java), renamed_to),
            ignored,
        }
    }

    pub fn emit(&self, context: &Context, mod_: &str) -> anyhow::Result<TokenStream> {
        let mut emit_reject_reasons = Vec::new();

        if !self.java.is_public() {
            emit_reject_reasons.push("Non-public field");
        }
        if self.ignored {
            emit_reject_reasons.push("[[ignore]]d");
        }

        let descriptor = &self.java.descriptor();

        let rust_type = emit_rust_type(descriptor, context, mod_, &mut emit_reject_reasons)?;

        // `rust_set_type` and `rust_get_type` are ones used below
        let (rust_set_type, rust_get_type) = match (descriptor.dimensions, &descriptor.field_type) {
            (0, t) if !matches!(t, FieldType::Object(_)) => (rust_type.clone(), rust_type),
            (0, FieldType::Object(cls)) if self.java.is_constant() && ClassName::from(cls).is_string_class() => {
                (quote!(&'static str), quote!(&'static str))
            }
            _ => (
                quote!(impl ::java_spaghetti::AsArg<#rust_type>),
                quote!(::std::option::Option<::java_spaghetti::Local<'env, #rust_type>>),
            ),
        };

        let field_fragment = emit_fragment_type(descriptor);

        if self.rust_names.is_err() {
            emit_reject_reasons.push(match self.java.name() {
                "$VALUES" => "Failed to mangle field name: enum $VALUES", // Expected
                s if s.starts_with("this$") => "Failed to mangle field name: this$N outer class pointer", // Expected
                _ => "ERROR:  Failed to mangle field name(s)",
            });
        }

        if !emit_reject_reasons.is_empty() {
            // TODO log
            return Ok(TokenStream::new());
        }

        let keywords = format!(
            "{}{}{}{}",
            self.java.access().unwrap_or("???"),
            if self.java.is_static() { " static" } else { "" },
            if self.java.is_final() { " final" } else { "" },
            if self.java.is_volatile() { " volatile" } else { "" }
        );

        let attributes = if self.java.deprecated() {
            quote!(#[deprecated])
        } else {
            quote!()
        };

        let mut out = TokenStream::new();

        let env_param = if self.java.is_static() {
            quote!(__jni_env: ::java_spaghetti::Env<'env>)
        } else {
            quote!(self: &::java_spaghetti::Ref<'env, Self>)
        };

        let docs = match KnownDocsUrl::from_field(
            context,
            self.class.path().as_str(),
            self.java.name(),
            self.java.descriptor().clone(),
        ) {
            Some(url) => format!("{keywords} {url}"),
            None => format!("{keywords} {}", self.java.name()),
        };

        match self.rust_names.as_ref().map_err(|e| anyhow!("bad mangling: {e}"))? {
            FieldMangling::ConstValue(constant, value) => {
                let constant = format_ident!("{}", constant);
                let value = emit_constant(&value, descriptor);

                out.extend(quote!(
                    #[doc = #docs]
                    #attributes
                    pub const #constant: #rust_get_type = #value;
                ));
            }
            FieldMangling::GetSet(get, set) => {
                let get = format_ident!("{get}");
                let set = format_ident!("{set}");

                let env_let = match self.java.is_static() {
                    false => quote!(let __jni_env = self.env();),
                    true => quote!(),
                };
                let require_field = match self.java.is_static() {
                    false => quote!(require_field),
                    true => quote!(require_static_field),
                };
                let get_field = match self.java.is_static() {
                    false => format_ident!("get_{field_fragment}_field"),
                    true => format_ident!("get_static_{field_fragment}_field"),
                };
                let set_field = match self.java.is_static() {
                    false => format_ident!("set_{field_fragment}_field"),
                    true => format_ident!("set_static_{field_fragment}_field"),
                };

                let java_name = cstring(self.java.name());
                let descriptor = cstring(&emit_field_descriptor(self.java.descriptor()));

                let get_docs = format!("**get** {docs}");
                let set_docs = format!("**set** {docs}");
                out.extend(quote!(
                    #[doc = #get_docs]
                    #attributes
                    pub fn #get<'env>(#env_param) -> #rust_get_type {
                        static __FIELD: ::std::sync::OnceLock<::java_spaghetti::JFieldID> = ::std::sync::OnceLock::new();
                        #env_let
                        let __jni_class = Self::__class_global_ref(__jni_env);
                        unsafe {
                            let __jni_field = __FIELD.get_or_init(|| ::java_spaghetti::JFieldID::from_raw(__jni_env.#require_field(__jni_class, #java_name, #descriptor))).as_raw();
                            __jni_env.#get_field(__jni_class, __jni_field)
                        }
                    }
                ));

                // Setter
                if !self.java.is_final() {
                    let lifetimes = if field_fragment == "object" {
                        quote!('env, 'obj)
                    } else {
                        quote!('env)
                    };

                    out.extend(quote!(
                        #[doc = #set_docs]
                        #attributes
                        pub fn #set<#lifetimes>(#env_param, value: #rust_set_type) {
                            static __FIELD: ::std::sync::OnceLock<::java_spaghetti::JFieldID> = ::std::sync::OnceLock::new();
                            #env_let
                            let __jni_class = Self::__class_global_ref(__jni_env);
                            unsafe {
                                let __jni_field = __FIELD.get_or_init(|| ::java_spaghetti::JFieldID::from_raw(__jni_env.#require_field(__jni_class, #java_name, #descriptor))).as_raw();
                                __jni_env.#set_field(__jni_class, __jni_field, value);
                            }
                        }
                    ));
                }
            }
        }

        Ok(out)
    }
}

pub fn emit_constant(constant: &LiteralConstant<'_>, descriptor: &FieldDescriptor) -> TokenStream {
    if descriptor.field_type == FieldType::Char && descriptor.dimensions == 0 {
        return match constant {
            LiteralConstant::Integer(value) => {
                let value = *value as i16;
                quote!(#value)
            }
            _ => panic!("invalid constant for char {:?}", constant),
        };
    }
    if descriptor.field_type == FieldType::Boolean && descriptor.dimensions == 0 {
        return match constant {
            LiteralConstant::Integer(0) => quote!(false),
            LiteralConstant::Integer(1) => quote!(true),
            _ => panic!("invalid constant for boolean {:?}", constant),
        };
    }

    match constant {
        LiteralConstant::Integer(value) => {
            let value = Literal::i32_unsuffixed(*value);
            quote!(#value)
        }
        LiteralConstant::Long(value) => {
            let value = Literal::i64_unsuffixed(*value);
            quote!(#value)
        }

        LiteralConstant::Float(value) if value.is_infinite() && *value < 0.0 => {
            quote!(::std::f32::NEG_INFINITY)
        }
        LiteralConstant::Float(value) if value.is_infinite() => quote!(::std::f32::INFINITY),
        LiteralConstant::Float(value) if value.is_nan() => quote!(::std::f32::NAN),
        LiteralConstant::Float(value) => quote!(#value),

        LiteralConstant::Double(value) if value.is_infinite() && *value < 0.0 => {
            quote!(::std::f64::NEG_INFINITY)
        }
        LiteralConstant::Double(value) if value.is_infinite() => quote!(::std::f64::INFINITY),
        LiteralConstant::Double(value) if value.is_nan() => quote!(::std::f64::NAN),
        LiteralConstant::Double(value) => quote!(#value),

        LiteralConstant::String(value) => quote! {#value},
        LiteralConstant::StringBytes(_) => {
            quote!(panic!("Java string constant contains invalid 'Modified UTF8'"))
        }
    }
}

/// Generates the corresponding Rust type for the Java field type.
pub fn emit_rust_type(
    descriptor: &FieldDescriptor,
    context: &Context<'_>,
    mod_: &str,
    reject_reasons: &mut Vec<&'static str>,
) -> Result<TokenStream, std::fmt::Error> {
    let res = if descriptor.dimensions == 0 {
        match &descriptor.field_type {
            FieldType::Boolean => quote!(bool),
            FieldType::Byte => quote!(i8),
            FieldType::Char => quote!(u16),
            FieldType::Short => quote!(i16),
            FieldType::Integer => quote!(i32),
            FieldType::Long => quote!(i64),
            FieldType::Float => quote!(f32),
            FieldType::Double => quote!(f64),
            FieldType::Object(class_name) => {
                let class = IdBuf::from(class_name);
                if !context.all_classes.contains_key(class.as_str()) {
                    reject_reasons.push("ERROR:  missing class for field/argument type");
                }
                if let Ok(path) = context.java_to_rust_path(class.as_id(), mod_) {
                    path
                } else {
                    reject_reasons.push("ERROR:  Failed to resolve JNI path to Rust path for class type");
                    let class = class.as_str();
                    quote!(#class) // XXX
                }
            }
        }
    } else {
        let throwable = context.throwable_rust_path(mod_);

        let mut res = match &descriptor.field_type {
            FieldType::Boolean => quote!(::java_spaghetti::BooleanArray),
            FieldType::Byte => quote!(::java_spaghetti::ByteArray),
            FieldType::Char => quote!(::java_spaghetti::CharArray),
            FieldType::Short => quote!(::java_spaghetti::ShortArray),
            FieldType::Integer => quote!(::java_spaghetti::IntArray),
            FieldType::Long => quote!(::java_spaghetti::LongArray),
            FieldType::Float => quote!(::java_spaghetti::FloatArray),
            FieldType::Double => quote!(::java_spaghetti::DoubleArray),
            FieldType::Object(class_name) => {
                let class = IdBuf::from(class_name);

                if !context.all_classes.contains_key(class.as_str()) {
                    reject_reasons.push("ERROR:  missing class for field type");
                }

                let path = match context.java_to_rust_path(class.as_id(), mod_) {
                    Ok(path) => path,
                    Err(_) => {
                        reject_reasons.push("ERROR:  Failed to resolve JNI path to Rust path for class type");
                        quote!(???)
                    }
                };

                quote!(::java_spaghetti::ObjectArray<#path, #throwable>)
            }
        };
        for _ in 0..(descriptor.dimensions - 1) {
            res = quote!(::java_spaghetti::ObjectArray<#res, #throwable>)
        }
        res
    };
    Ok(res)
}

/// Contents of {get,set}_[static_]..._field, call_..._method_a.
pub fn emit_fragment_type(descriptor: &FieldDescriptor) -> &'static str {
    if descriptor.dimensions == 0 {
        match descriptor.field_type {
            FieldType::Boolean => "boolean",
            FieldType::Byte => "byte",
            FieldType::Char => "char",
            FieldType::Short => "short",
            FieldType::Integer => "int",
            FieldType::Long => "long",
            FieldType::Float => "float",
            FieldType::Double => "double",
            FieldType::Object(_) => "object",
        }
    } else {
        "object"
    }
}
