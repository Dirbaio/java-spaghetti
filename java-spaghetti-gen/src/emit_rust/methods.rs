use cafebabe::descriptors::{FieldType, ReturnDescriptor};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::cstring;
use super::fields::{emit_fragment_type, emit_rust_type};
use super::known_docs_url::KnownDocsUrl;
use crate::emit_rust::Context;
use crate::identifiers::MethodManglingStyle;
use crate::parser_util::{emit_method_descriptor, JavaClass, JavaMethod};

pub struct Method<'a> {
    pub class: &'a JavaClass,
    pub java: JavaMethod<'a>,
    rust_name: Option<String>,
    mangling_style: MethodManglingStyle,
}

impl<'a> Method<'a> {
    pub fn new(context: &Context, class: &'a JavaClass, java: &'a cafebabe::MethodInfo<'a>) -> Self {
        let mut result = Self {
            class,
            java: JavaMethod::from(java),
            rust_name: None,
            mangling_style: MethodManglingStyle::Java, // Immediately overwritten below
        };
        result.set_mangling_style(context.config.codegen.method_naming_style); // rust_name + mangling_style
        result
    }

    pub fn rust_name(&self) -> Option<&str> {
        self.rust_name.as_deref()
    }

    pub fn set_mangling_style(&mut self, style: MethodManglingStyle) {
        self.mangling_style = style;
        self.rust_name = self
            .mangling_style
            .mangle(self.java.name(), self.java.descriptor())
            .ok()
    }

    pub fn emit(&self, context: &Context, mod_: &str) -> anyhow::Result<TokenStream> {
        let mut emit_reject_reasons = Vec::new();

        let java_class_method = format!("{}\x1f{}", self.class.path().as_str(), self.java.name());
        let java_class_method_sig = format!(
            "{}\x1f{}\x1f{}",
            self.class.path().as_str(),
            self.java.name(),
            emit_method_descriptor(self.java.descriptor())
        );

        let ignored = context.config.ignore_class_methods.contains(&java_class_method)
            || context.config.ignore_class_method_sigs.contains(&java_class_method_sig);

        let renamed_to = context
            .config
            .rename_class_methods
            .get(&java_class_method)
            .or_else(|| context.config.rename_class_method_sigs.get(&java_class_method_sig));

        let descriptor = self.java.descriptor();

        let method_name = if let Some(renamed_to) = renamed_to {
            renamed_to.clone()
        } else if let Some(name) = self.rust_name() {
            name.to_owned()
        } else {
            emit_reject_reasons.push("ERROR:  Failed to mangle method name");
            self.java.name().to_owned()
        };

        if !self.java.is_public() {
            emit_reject_reasons.push("Non-public method");
        }
        if self.java.is_bridge() {
            emit_reject_reasons.push("Bridge method - type erasure");
        }
        if self.java.is_static_init() {
            emit_reject_reasons.push("Static class constructor - never needs to be called by Rust.");
        }
        if ignored {
            emit_reject_reasons.push("[[ignore]]d");
        }

        // Parameter names may or may not be available as extra debug information.  Example:
        // https://docs.oracle.com/javase/tutorial/reflect/member/methodparameterreflection.html

        let mut params_array = TokenStream::new(); // Contents of let __jni_args = [...];

        // Contents of fn name<'env>(...) {
        let mut params_decl = if self.java.is_constructor() || self.java.is_static() {
            quote!(__jni_env: ::java_spaghetti::Env<'env>)
        } else {
            quote!(self: &::java_spaghetti::Ref<'env, Self>)
        };

        for (arg_idx, arg) in descriptor.parameters.iter().enumerate() {
            let arg_name = format_ident!("arg{}", arg_idx);

            let param_is_object = matches!(arg.field_type, FieldType::Object(_)) || arg.dimensions > 0;

            let rust_type = emit_rust_type(arg, context, mod_, &mut emit_reject_reasons)?;

            let arg_type = if arg.dimensions == 0 && !param_is_object {
                rust_type
            } else {
                quote!(impl ::java_spaghetti::AsArg<#rust_type>)
            };

            if !params_array.is_empty() {
                params_array.extend(quote!(,));
            }

            if param_is_object {
                params_array.extend(quote!(#arg_name.as_arg_jvalue()));
            } else {
                params_array.extend(quote!(::java_spaghetti::AsJValue::as_jvalue(&#arg_name)));
            }

            if !params_decl.is_empty() {
                params_decl.extend(quote!(,));
            }

            params_decl.extend(quote!(#arg_name: #arg_type));
        }

        let mut ret_decl = if let ReturnDescriptor::Return(desc) = &descriptor.return_type {
            let rust_type = emit_rust_type(desc, context, mod_, &mut emit_reject_reasons)?;

            let param_is_object = matches!(desc.field_type, FieldType::Object(_));
            if desc.dimensions == 0 && !param_is_object {
                rust_type
            } else {
                quote!(::std::option::Option<::java_spaghetti::Local<'env, #rust_type>>)
            }
        } else {
            quote!(())
        };

        let mut ret_method_fragment = if let ReturnDescriptor::Return(desc) = &descriptor.return_type {
            emit_fragment_type(desc)
        } else {
            "void"
        };

        if self.java.is_constructor() {
            if descriptor.return_type == ReturnDescriptor::Void {
                ret_method_fragment = "object";
                ret_decl = quote!(::java_spaghetti::Local<'env, Self>);
            } else {
                emit_reject_reasons.push("ERROR:  Constructor should've returned void");
            }
        }

        if !emit_reject_reasons.is_empty() {
            // TODO log
            return Ok(TokenStream::new());
        }

        let mut out = TokenStream::new();

        let access = if self.java.is_public() { quote!(pub) } else { quote!() };
        let attributes = if self.java.deprecated() {
            quote!(#[deprecated])
        } else {
            quote!()
        };

        let docs = match KnownDocsUrl::from_method(context, self) {
            Some(url) => format!("{url}"),
            None => format!("{}", self.java.name()),
        };

        let throwable = context.throwable_rust_path(mod_);

        let env_let = match !self.java.is_constructor() && !self.java.is_static() {
            true => quote!(let __jni_env = self.env();),
            false => quote!(),
        };
        let require_method = match self.java.is_static() {
            false => quote!(require_method),
            true => quote!(require_static_method),
        };

        let java_name = cstring(self.java.name());
        let descriptor = cstring(&emit_method_descriptor(self.java.descriptor()));
        let method_name = format_ident!("{method_name}");

        let call = if self.java.is_constructor() {
            quote!(__jni_env.new_object_a(__jni_class, __jni_method, __jni_args.as_ptr()))
        } else if self.java.is_static() {
            let call = format_ident!("call_static_{ret_method_fragment}_method_a");
            quote!(    __jni_env.#call(__jni_class, __jni_method, __jni_args.as_ptr()))
        } else {
            let call = format_ident!("call_{ret_method_fragment}_method_a");
            quote!(    __jni_env.#call(self.as_raw(), __jni_method, __jni_args.as_ptr()))
        };

        out.extend(quote!(
            #[doc = #docs]
            #attributes
            #access fn #method_name<'env>(#params_decl) -> ::std::result::Result<#ret_decl, ::java_spaghetti::Local<'env, #throwable>> {
                static __METHOD: ::std::sync::OnceLock<::java_spaghetti::JMethodID> = ::std::sync::OnceLock::new();
                unsafe {
                    let __jni_args = [#params_array];
                    #env_let
                    let __jni_class = Self::__class_global_ref(__jni_env);
                    let __jni_method = __METHOD.get_or_init(||
                        ::java_spaghetti::JMethodID::from_raw(__jni_env.#require_method(__jni_class, #java_name, #descriptor))
                    ).as_raw();

                    #call
                }
            }
        ));

        Ok(out)
    }
}
