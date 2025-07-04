use anyhow::anyhow;
use cafebabe::constant_pool::LiteralConstant;
use cafebabe::descriptors::{FieldDescriptor, FieldType};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use super::cstring;
use super::known_docs_url::KnownDocsUrl;
use crate::config::ClassConfig;
use crate::emit::Context;
use crate::identifiers::{FieldMangling, mangle_field};
use crate::parser_util::{Id, JavaClass, JavaField};

pub struct Field<'a> {
    pub class: &'a JavaClass,
    pub java: JavaField<'a>,
    pub rust_names: Result<FieldMangling<'a>, anyhow::Error>,
}

impl<'a> Field<'a> {
    pub fn new(class: &'a JavaClass, java: &'a cafebabe::FieldInfo<'a>) -> Self {
        Self {
            class,
            java: JavaField::from(java),
            rust_names: mangle_field(JavaField::from(java)),
        }
    }

    pub fn emit(&self, context: &Context, cc: &ClassConfig, mod_: &str) -> anyhow::Result<TokenStream> {
        let mut emit_reject_reasons = Vec::new();

        let descriptor = &self.java.descriptor();

        let rust_set_type = emit_type(
            descriptor,
            context,
            mod_,
            RustTypeFlavor::ImplAsArg,
            &mut emit_reject_reasons,
        )?;
        let rust_get_type = emit_type(
            descriptor,
            context,
            mod_,
            RustTypeFlavor::OptionLocal,
            &mut emit_reject_reasons,
        )?;

        let static_fragment = match self.java.is_static() {
            false => "",
            true => "_static",
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
            cc,
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
                let value = emit_constant(value, descriptor);
                let ty = if descriptor.dimensions == 0
                    && let FieldType::Object(cls) = &descriptor.field_type
                    && Id::from(cls).is_string_class()
                {
                    quote!(&'static str)
                } else {
                    rust_get_type
                };

                out.extend(quote!(
                    #[doc = #docs]
                    #attributes
                    pub const #constant: #ty = #value;
                ));
            }
            FieldMangling::GetSet(get, set) => {
                let get = format_ident!("{get}");
                let set = format_ident!("{set}");

                let env_let = match self.java.is_static() {
                    false => quote!(let __jni_env = self.env();),
                    true => quote!(),
                };
                let require_field = format_ident!("require{static_fragment}_field");
                let get_field = format_ident!("get{static_fragment}_{field_fragment}_field");
                let set_field = format_ident!("set{static_fragment}_{field_fragment}_field");

                let this_or_class = match self.java.is_static() {
                    false => quote!(self.as_raw()),
                    true => quote!(__jni_class),
                };

                let java_name = cstring(self.java.name());
                let descriptor = cstring(&self.java.descriptor().to_string());

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
                            __jni_env.#get_field(#this_or_class, __jni_field)
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
                                __jni_env.#set_field(#this_or_class, __jni_field, value);
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
                let value = *value as u16;
                quote!(#value)
            }
            _ => panic!("invalid constant for char {constant:?}"),
        };
    }
    if descriptor.field_type == FieldType::Boolean && descriptor.dimensions == 0 {
        return match constant {
            LiteralConstant::Integer(0) => quote!(false),
            LiteralConstant::Integer(1) => quote!(true),
            _ => panic!("invalid constant for boolean {constant:?}"),
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

pub enum RustTypeFlavor {
    ImplAsArg,
    OptionLocal,
    OptionRef,
    Arg,
    Return,
}

fn flavorify(ty: TokenStream, flavor: RustTypeFlavor) -> TokenStream {
    match flavor {
        RustTypeFlavor::ImplAsArg => quote!(impl ::java_spaghetti::AsArg<#ty>),
        RustTypeFlavor::OptionLocal => quote!(::std::option::Option<::java_spaghetti::Local<'env, #ty>>),
        RustTypeFlavor::OptionRef => quote!(::std::option::Option<::java_spaghetti::Ref<'env, #ty>>),
        RustTypeFlavor::Arg => quote!(::java_spaghetti::Arg<#ty>),
        RustTypeFlavor::Return => quote!(::java_spaghetti::Return<'env, #ty>),
    }
}

/// Generates the corresponding Rust type for the Java field type.
pub fn emit_type(
    descriptor: &FieldDescriptor,
    context: &Context<'_>,
    mod_: &str,
    flavor: RustTypeFlavor,
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
                let class = Id::from(class_name);
                if !context.all_classes.contains_key(class.as_str()) {
                    reject_reasons.push("ERROR:  missing class for field/argument type");
                }
                if let Ok(path) = context.java_to_rust_path(class, mod_) {
                    flavorify(path, flavor)
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
                let class = Id::from(class_name);

                if !context.all_classes.contains_key(class.as_str()) {
                    reject_reasons.push("ERROR:  missing class for field type");
                }

                let path = match context.java_to_rust_path(class, mod_) {
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

        flavorify(res, flavor)
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
