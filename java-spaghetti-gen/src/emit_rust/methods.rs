use std::io;

use cafebabe::descriptors::{FieldType, ReturnDescriptor};

use super::fields::FieldTypeEmitter;
use super::known_docs_url::KnownDocsUrl;
use super::StrEmitter;
use crate::emit_rust::Context;
use crate::identifiers::MethodManglingStyle;
use crate::parser_util::{Class, JavaMethod, MethodSigWriter};

pub struct Method<'a> {
    pub class: &'a Class,
    pub java: JavaMethod<'a>,
    rust_name: Option<String>,
    mangling_style: MethodManglingStyle,
}

impl<'a> Method<'a> {
    pub fn new(context: &Context, class: &'a Class, java: &'a cafebabe::MethodInfo<'a>) -> Self {
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

    pub fn emit(&self, context: &Context, mod_: &str, out: &mut impl io::Write) -> io::Result<()> {
        let mut emit_reject_reasons = Vec::new();

        let java_class_method = format!("{}\x1f{}", self.class.path().as_str(), self.java.name());
        let java_class_method_sig = format!(
            "{}\x1f{}\x1f{}",
            self.class.path().as_str(),
            self.java.name(),
            MethodSigWriter(self.java.descriptor())
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
            return Ok(());
        }
        if ignored {
            emit_reject_reasons.push("[[ignore]]d");
        }

        // Parameter names may or may not be available as extra debug information.  Example:
        // https://docs.oracle.com/javase/tutorial/reflect/member/methodparameterreflection.html

        let mut params_array = String::new(); // Contents of let __jni_args = [...];

        // Contents of fn name<'env>(...) {
        let mut params_decl = if self.java.is_constructor() || self.java.is_static() {
            String::from("__jni_env: ::java_spaghetti::Env<'env>")
        } else {
            String::from("self: &::java_spaghetti::Ref<'env, Self>")
        };

        for (arg_idx, arg) in descriptor.parameters.iter().enumerate() {
            let arg_name = format!("arg{}", arg_idx);

            let param_is_object = matches!(arg.field_type, FieldType::Object(_)) || arg.dimensions > 0;

            let rust_type = FieldTypeEmitter(arg)
                .emit_rust_type(context, mod_, &mut emit_reject_reasons)
                .map_err(|_| io::Error::other("std::fmt::Error"))?;

            let arg_type = if arg.dimensions == 0 && !param_is_object {
                rust_type.into_owned()
            } else {
                let mut rust_type = rust_type.into_owned();
                rust_type.insert_str(0, "impl ::java_spaghetti::AsArg<");
                rust_type.push('>');
                rust_type
            };

            if !params_array.is_empty() {
                params_array.push_str(", ");
            }

            if param_is_object {
                params_array.push_str(arg_name.as_str());
                params_array.push_str(".as_arg_jvalue()");
            } else {
                params_array.push_str("::java_spaghetti::AsJValue::as_jvalue(&");
                params_array.push_str(arg_name.as_str());
                params_array.push(')');
            }

            if !params_decl.is_empty() {
                params_decl.push_str(", ");
            }

            params_decl.push_str(arg_name.as_str());
            params_decl.push_str(": ");
            params_decl.push_str(arg_type.as_str());
        }

        let mut ret_decl = if let ReturnDescriptor::Return(desc) = &descriptor.return_type {
            let rust_type = FieldTypeEmitter(desc)
                .emit_rust_type(context, mod_, &mut emit_reject_reasons)
                .map_err(|_| io::Error::other("std::fmt::Error"))?;

            let param_is_object = matches!(desc.field_type, FieldType::Object(_));
            if desc.dimensions == 0 && !param_is_object {
                rust_type
            } else {
                let mut rust_type = rust_type.into_owned();
                rust_type.insert_str(0, "::std::option::Option<::java_spaghetti::Local<'env, ");
                rust_type.push_str(">>");
                rust_type.into()
            }
        } else {
            "()".into()
        };

        let mut ret_method_fragment = if let ReturnDescriptor::Return(desc) = &descriptor.return_type {
            FieldTypeEmitter(desc).emit_fragment_type()
        } else {
            "void"
        };

        if self.java.is_constructor() {
            if descriptor.return_type == ReturnDescriptor::Void {
                ret_method_fragment = "object";
                ret_decl = "::java_spaghetti::Local<'env, Self>".into();
            } else {
                emit_reject_reasons.push("ERROR:  Constructor should've returned void");
            }
        }

        let emit_reject_reasons = emit_reject_reasons; // Freeze
        let indent = if emit_reject_reasons.is_empty() {
            ""
        } else {
            if !context.config.codegen.keep_rejected_emits {
                return Ok(());
            }
            "// "
        };
        let access = if self.java.is_public() { "pub " } else { "" };
        let attributes = if self.java.deprecated() { "#[deprecated] " } else { "" };

        writeln!(out)?;
        for reason in &emit_reject_reasons {
            writeln!(out, "{indent}// Not emitting: {reason}")?;
        }
        if let Some(url) = KnownDocsUrl::from_method(context, self) {
            writeln!(out, "{indent}/// {url}")?;
        } else {
            writeln!(out, "{indent}/// {}", self.java.name())?;
        }
        writeln!(
            out,
            "{indent}{attributes}{access}fn {method_name}<'env>({params_decl}) -> \
            ::std::result::Result<{ret_decl}, ::java_spaghetti::Local<'env, {}>> {{",
            context.throwable_rust_path(mod_)
        )?;
        writeln!(
            out,
            "{}    // class.path == {:?}, java.flags == {:?}, .name == {:?}, .descriptor == \"{}\"",
            indent,
            self.class.path().as_str(),
            self.java.access_flags,
            self.java.name(),
            MethodSigWriter(self.java.descriptor())
        )?;

        writeln!(
            out,
            "{indent}    static __METHOD: ::std::sync::OnceLock<::java_spaghetti::JMethodID> \
                = ::std::sync::OnceLock::new();"
        )?;
        writeln!(out, "{indent}    unsafe {{")?;
        writeln!(out, "{indent}        let __jni_args = [{params_array}];")?;
        if !self.java.is_constructor() && !self.java.is_static() {
            writeln!(out, "{indent}        let __jni_env = self.env();")?;
        }
        writeln!(
            out,
            "{indent}        let __jni_class = Self::__class_global_ref(__jni_env);"
        )?;
        writeln!(
            out,
            "{indent}        \
            let __jni_method = *__METHOD.get_or_init(|| __jni_env.require_{}method(__jni_class, {}, {}));",
            if self.java.is_static() { "static_" } else { "" },
            StrEmitter(self.java.name()),
            StrEmitter(MethodSigWriter(self.java.descriptor()))
        )?;

        if self.java.is_constructor() {
            writeln!(
                out,
                "{indent}        \
                __jni_env.new_object_a(__jni_class, __jni_method, __jni_args.as_ptr())",
            )?;
        } else if self.java.is_static() {
            writeln!(
                out,
                "{indent}        \
                __jni_env.call_static_{}_method_a(__jni_class, __jni_method, __jni_args.as_ptr())",
                ret_method_fragment
            )?;
        } else {
            writeln!(
                out,
                "{indent}        \
                __jni_env.call_{}_method_a(self.as_raw(), __jni_method, __jni_args.as_ptr())",
                ret_method_fragment
            )?;
        }
        writeln!(out, "{indent}    }}")?;
        writeln!(out, "{indent}}}")?;
        Ok(())
    }
}
