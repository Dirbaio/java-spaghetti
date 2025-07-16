use std::fmt::Write;

use cafebabe::descriptors::{FieldDescriptor, FieldType, ReturnDescriptor};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::classes::Class;
use super::cstring;
use super::fields::RustTypeFlavor;
use super::methods::Method;
use crate::emit::Context;
use crate::emit::fields::emit_type;

impl Class {
    #[allow(clippy::vec_init_then_push)]
    pub(crate) fn write_proxy(&self, context: &Context, methods: &[Method]) -> anyhow::Result<TokenStream> {
        let mut emit_reject_reasons = Vec::new();

        let mut out = TokenStream::new();
        let mut contents = TokenStream::new();

        let rust_name = format_ident!("{}", &self.rust.struct_name);

        let throwable = context.throwable_rust_path(&self.rust.mod_);
        let rust_proxy_name = format_ident!("{}Proxy", &self.rust.struct_name);

        let mut trait_methods = TokenStream::new();

        let java_proxy_path = format!(
            "{}/{}",
            context.config.proxy_package,
            self.java.path().as_str().replace("$", "_")
        );

        let mut native_regs = Vec::new();
        for method in methods {
            let Some(rust_name) = method.rust_name() else { continue };
            if method.java.is_static()
                || method.java.is_static_init()
                || method.java.is_constructor()
                || method.java.is_final()
                || method.java.is_private()
            {
                continue;
            }

            let mut native_params = Vec::new();
            native_params.push(FieldDescriptor {
                dimensions: 0,
                field_type: FieldType::Long,
            });
            native_params.extend(method.java.descriptor.parameters.iter().cloned());
            let native_name = mangle_native_method(
                &java_proxy_path,
                &format!("native_{}", method.java.name()),
                &native_params,
            );
            let native_name = format_ident!("{native_name}");
            let rust_name = format_ident!("{rust_name}");

            let mut native_method_desc = method.java.descriptor().to_string();
            native_method_desc.insert(1, 'J');
            native_regs.push((
                cstring(&format!("native_{}", method.java.name())),
                cstring(&native_method_desc),
                native_name.clone(),
            ));

            let ret = match &method.java.descriptor.return_type {
                ReturnDescriptor::Void => quote!(()),
                ReturnDescriptor::Return(desc) => emit_type(
                    desc,
                    context,
                    &self.rust.mod_,
                    RustTypeFlavor::Return,
                    &mut emit_reject_reasons,
                )?,
            };

            let mut trait_args = TokenStream::new();
            let mut native_args = TokenStream::new();
            let mut native_convert_args = TokenStream::new();

            for (arg_idx, arg) in method.java.descriptor.parameters.iter().enumerate() {
                let arg_name = format_ident!("arg{}", arg_idx);

                let trait_arg_type = emit_type(
                    arg,
                    context,
                    &self.rust.mod_,
                    RustTypeFlavor::OptionRef,
                    &mut emit_reject_reasons,
                )?;
                trait_args.extend(quote!(#arg_name: #trait_arg_type,));

                let native_arg_type = emit_type(
                    arg,
                    context,
                    &self.rust.mod_,
                    RustTypeFlavor::Arg,
                    &mut emit_reject_reasons,
                )?;
                native_args.extend(quote!(#arg_name: #native_arg_type,));
                if matches!(arg.field_type, FieldType::Object(_)) || arg.dimensions > 0 {
                    native_convert_args.extend(quote!(#arg_name.into_ref(__jni_env),));
                } else {
                    native_convert_args.extend(quote!(#arg_name,));
                }
            }

            trait_methods.extend(quote!(
                fn #rust_name<'env>(
                    &self,
                    env: ::java_spaghetti::Env<'env>,
                    #trait_args
                ) -> #ret;
            ));

            out.extend(quote!(
                #[unsafe(no_mangle)]
                extern "system" fn #native_name<'env>(
                    __jni_env: ::java_spaghetti::Env<'env>,
                    _class: *mut (), // self class, ignore
                    ptr: i64,
                    #native_args
                ) -> #ret {
                    let ptr: *const std::sync::Arc<dyn #rust_proxy_name> = ::std::ptr::with_exposed_provenance(ptr as usize);
                    unsafe {
                        (*ptr).#rust_name(__jni_env, #native_convert_args )
                    }
                }
            ));
        }

        let mut native_params = Vec::new();
        native_params.push(FieldDescriptor {
            dimensions: 0,
            field_type: FieldType::Long,
        });
        let native_name = mangle_native_method(&java_proxy_path, "native_finalize", &native_params);
        let native_name = format_ident!("{native_name}");

        out.extend(quote!(
            pub trait #rust_proxy_name: ::std::marker::Send + ::std::marker::Sync + 'static {
                #trait_methods
            }

            #[unsafe(no_mangle)]
            extern "system" fn #native_name(
                __jni_env: ::java_spaghetti::Env<'_>,
                _class: *mut (), // self class, ignore
                ptr: i64,
            ) {
                let ptr: *mut std::sync::Arc<dyn #rust_proxy_name> = ::std::ptr::with_exposed_provenance_mut(ptr as usize);
                let _ = unsafe { Box::from_raw(ptr) };
            }
        ));

        let java_proxy_path = cstring(&java_proxy_path);

        contents.extend(quote!(
            pub fn new_proxy<'env>(
                env: ::java_spaghetti::Env<'env>,
                proxy: ::std::sync::Arc<dyn #rust_proxy_name>,
                proxy_class: ::std::option::Option<::java_spaghetti::JClass>,
            ) -> Result<::java_spaghetti::Local<'env, Self>, ::java_spaghetti::Local<'env, #throwable>> {
                static __CLASS: ::std::sync::OnceLock<::java_spaghetti::JClass> = ::std::sync::OnceLock::new();
                let __jni_class = __CLASS
                    .get_or_init(|| unsafe {
                        let required = env.require_class(#java_proxy_path);
                        if let Ok(proxy_class) = required {
                            proxy_class
                        } else if let Some(proxy_class) = proxy_class {
                            let bin_name = env.get_class_name(&proxy_class).replace('.', "/");
                            let expected = #java_proxy_path.to_string_lossy();
                            if bin_name != expected {
                                panic!("wrong proxy_class, expected: {}, provided: {}", expected, bin_name)
                            }
                            Self::register_proxy_methods(env, &proxy_class);
                            proxy_class
                        } else {
                            panic!("{}", required.unwrap_err())
                        }
                    });

                let b = ::std::boxed::Box::new(proxy);
                let ptr = ::std::boxed::Box::into_raw(b);

                static __METHOD: ::std::sync::OnceLock<::java_spaghetti::JMethodID> = ::std::sync::OnceLock::new();
                unsafe {
                    let __jni_args = &[::java_spaghetti::sys::jvalue {
                        j: ptr.expose_provenance() as i64,
                    }];
                    let __jni_method = *__METHOD.get_or_init(|| env.require_method(__jni_class, c"<init>", c"(J)V"));
                    env.new_object_a(__jni_class, __jni_method, __jni_args)
                }
            }
        ));

        let mut register_calls = TokenStream::new();
        for (native_method_name, descriptor, extern_name) in native_regs {
            register_calls.extend(quote!(
                {
                    let method_name = #native_method_name;
                    let descriptor = #descriptor;
                    let fn_ptr = #extern_name as *mut _;
                    let _ = env.register_native_method(proxy_class, method_name, descriptor, fn_ptr);
                }
            ));
        }
        contents.extend(quote!(
            fn register_proxy_methods<'env>(
                env: ::java_spaghetti::Env<'env>,
                proxy_class: &::java_spaghetti::JClass,
            ) {
                unsafe {
                    #register_calls
                }
            }
        ));

        out.extend(quote!(impl #rust_name { #contents }));

        if !emit_reject_reasons.is_empty() {
            // TODO log
            return Ok(TokenStream::new());
        }

        Ok(out)
    }
}

fn mangle_native_method(path: &str, name: &str, args: &[FieldDescriptor]) -> String {
    let mut res = String::new();
    res.push_str("Java_");
    res.push_str(&mangle_native(path));
    res.push('_');
    res.push_str(&mangle_native(name));
    res.push_str("__");
    for d in args {
        res.push_str(&mangle_native(&d.to_string()));
    }

    res
}

fn mangle_native(s: &str) -> String {
    let mut res = String::new();
    for c in s.chars() {
        match c {
            '0'..='9' | 'a'..='z' | 'A'..='Z' => res.push(c),
            '/' => res.push('_'),
            '_' => res.push_str("_1"),
            ';' => res.push_str("_2"),
            '[' => res.push_str("_3"),
            _ => write!(&mut res, "_0{:04x}", c as u16).unwrap(),
        }
    }
    res
}
