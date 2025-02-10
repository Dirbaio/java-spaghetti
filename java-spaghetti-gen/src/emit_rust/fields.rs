use std::borrow::Cow;
use std::fmt::Write;
use std::io;

use cafebabe::constant_pool::LiteralConstant;
use cafebabe::descriptors::{FieldDescriptor, FieldType};

use super::known_docs_url::KnownDocsUrl;
use super::StrEmitter;
use crate::emit_rust::Context;
use crate::identifiers::{FieldMangling, IdentifierManglingError};
use crate::parser_util::{Class, ClassName, FieldSigWriter, IdBuf, IterableId, JavaField};

pub struct Field<'a> {
    pub class: &'a Class,
    pub java: JavaField<'a>,
    pub rust_names: Result<FieldMangling<'a>, IdentifierManglingError>,
    pub ignored: bool,
}

impl<'a> Field<'a> {
    pub fn new(context: &Context, class: &'a Class, java: &'a cafebabe::FieldInfo<'a>) -> Self {
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

    pub fn emit(&self, context: &Context, mod_: &str, out: &mut impl io::Write) -> io::Result<()> {
        let mut emit_reject_reasons = Vec::new();

        if !self.java.is_public() {
            emit_reject_reasons.push("Non-public field");
        }
        if self.ignored {
            emit_reject_reasons.push("[[ignore]]d");
        }

        let descriptor = &self.java.descriptor();
        let type_emitter = FieldTypeEmitter(descriptor);

        let rust_type = type_emitter
            .emit_rust_type(context, mod_, &mut emit_reject_reasons)
            .map_err(|_| io::Error::other("std::fmt::Error"))?;

        let (rust_set_type_buffer, rust_get_type_buffer);
        // `rust_set_type` and `rust_get_type` are ones used below
        let (rust_set_type, rust_get_type) = match (descriptor.dimensions, &descriptor.field_type) {
            (0, t) if !matches!(t, FieldType::Object(_)) => (rust_type.as_ref(), rust_type.as_ref()),
            (0, FieldType::Object(cls)) if self.java.is_constant() && ClassName::from(cls).is_string_class() => {
                ("&'static str", "&'static str")
            }
            _ => {
                rust_set_type_buffer = format!("impl ::java_spaghetti::AsArg<{}>", &rust_type);
                rust_get_type_buffer = format!("::std::option::Option<::java_spaghetti::Local<'env, {}>>", &rust_type);
                (rust_set_type_buffer.as_str(), rust_get_type_buffer.as_str())
            }
        };

        let field_fragment = type_emitter.emit_fragment_type();

        if self.rust_names.is_err() {
            emit_reject_reasons.push(match self.java.name() {
                "$VALUES" => "Failed to mangle field name: enum $VALUES", // Expected
                s if s.starts_with("this$") => "Failed to mangle field name: this$N outer class pointer", // Expected
                _ => "ERROR:  Failed to mangle field name(s)",
            });
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

        let keywords = format!(
            "{}{}{}{}",
            self.java.access().unwrap_or("???"),
            if self.java.is_static() { " static" } else { "" },
            if self.java.is_final() { " final" } else { "" },
            if self.java.is_volatile() { " volatile" } else { "" }
        );

        let attributes = if self.java.deprecated() { "#[deprecated] " } else { "" };

        writeln!(out)?;
        for reason in &emit_reject_reasons {
            writeln!(out, "{}// Not emitting: {}", indent, reason)?;
        }

        let env_param = if self.java.is_static() {
            "__jni_env: ::java_spaghetti::Env<'env>"
        } else {
            "self: &::java_spaghetti::Ref<'env, Self>"
        };

        let url = KnownDocsUrl::from_field(
            context,
            self.class.path().as_str(),
            self.java.name(),
            self.java.descriptor().clone(),
        );
        let url = url.as_ref();

        match self.rust_names.as_ref() {
            Ok(FieldMangling::ConstValue(constant, value)) => {
                let value_writer = ConstantWriter(value);
                if let Some(url) = url {
                    writeln!(out, "{indent}/// {keywords} {url}")?;
                }
                match descriptor.field_type {
                    FieldType::Char if descriptor.dimensions == 0 => writeln!(
                        out,
                        "{indent}{attributes}pub const {constant} : {rust_get_type} = {rust_get_type}({value_writer});",
                    )?,
                    FieldType::Boolean if descriptor.dimensions == 0 => writeln!(
                        out,
                        "{indent}{attributes}pub const {constant} : {rust_get_type} = {};",
                        if let LiteralConstant::Integer(0) = value {
                            "false"
                        } else {
                            "true"
                        }
                    )?,
                    _ => writeln!(
                        out,
                        "{indent}{attributes}pub const {constant} : {rust_get_type} = {value_writer};",
                    )?,
                }
            }
            Ok(FieldMangling::GetSet(get, set)) => {
                // Getter
                if let Some(url) = url {
                    writeln!(out, "{indent}/// **get** {keywords} {url}")?;
                } else {
                    writeln!(out, "{indent}/// **get** {keywords} {}", self.java.name())?;
                }
                writeln!(
                    out,
                    "{indent}{attributes}pub fn {get}<'env>({env_param}) -> {rust_get_type} {{",
                )?;
                writeln!(
                    out,
                    "{indent}    static __FIELD: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();"
                )?;
                writeln!(out, "{indent}    unsafe {{")?;
                if !self.java.is_static() {
                    writeln!(out, "{indent}        let __jni_env = self.env();")?;
                }
                writeln!(
                    out,
                    "{indent}        let __jni_class = Self::__class_global_ref(__jni_env);"
                )?;
                writeln!(
                    out,
                    "{indent}        \
                    let __jni_field = *__FIELD.get_or_init(|| \
                        __jni_env.require_{}field(__jni_class, {}, {}).addr()\
                    ) as ::java_spaghetti::sys::jfieldID;",
                    if self.java.is_static() { "static_" } else { "" },
                    StrEmitter(self.java.name()),
                    StrEmitter(FieldSigWriter(self.java.descriptor()))
                )?;
                if self.java.is_static() {
                    writeln!(
                        out,
                        "{indent}        __jni_env.get_static_{field_fragment}_field(__jni_class, __jni_field)",
                    )?;
                } else {
                    writeln!(
                        out,
                        "{indent}        __jni_env.get_{field_fragment}_field(self.as_raw(), __jni_field)",
                    )?;
                }
                writeln!(out, "{indent}    }}")?;
                writeln!(out, "{indent}}}")?;

                // Setter
                if !self.java.is_final() {
                    let lifetimes = if field_fragment == "object" {
                        "'env, 'obj"
                    } else {
                        "'env"
                    };

                    writeln!(out)?;
                    if let Some(url) = url {
                        writeln!(out, "{indent}/// **set** {keywords} {url}")?;
                    } else {
                        writeln!(out, "{indent}/// **set** {keywords} {}", self.java.name())?;
                    }
                    writeln!(
                        out,
                        "{indent}{attributes}pub fn {set}<{lifetimes}>({env_param}, value: {rust_set_type}) {{",
                    )?;
                    writeln!(
                        out,
                        "{indent}    static __FIELD: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();"
                    )?;
                    writeln!(out, "{indent}    unsafe {{")?;
                    if !self.java.is_static() {
                        writeln!(out, "{indent}        let __jni_env = self.env();")?;
                    }
                    writeln!(
                        out,
                        "{indent}        let __jni_class = Self::__class_global_ref(__jni_env);"
                    )?;
                    writeln!(
                        out,
                        "{indent}        \
                        let __jni_field = *__FIELD.get_or_init(|| \
                            __jni_env.require_{}field(__jni_class, {}, {}).addr()\
                        ) as ::java_spaghetti::sys::jfieldID;",
                        if self.java.is_static() { "static_" } else { "" },
                        StrEmitter(self.java.name()),
                        StrEmitter(FieldSigWriter(self.java.descriptor()))
                    )?;
                    if self.java.is_static() {
                        writeln!(
                            out,
                            "{indent}        \
                            __jni_env.set_static_{field_fragment}_field(__jni_class, __jni_field, value)",
                        )?;
                    } else {
                        writeln!(
                            out,
                            "{indent}        \
                            __jni_env.set_{field_fragment}_field(self.as_raw(), __jni_field, value)",
                        )?;
                    }
                    writeln!(out, "{indent}    }}")?;
                    writeln!(out, "{indent}}}")?;
                }
            }
            Err(_) => {
                writeln!(
                    out,
                    "{indent}{attributes}pub fn get_{:?}<'env>({env_param}) -> {rust_get_type} {{ ... }}",
                    self.java.name(),
                )?;
                if !self.java.is_final() {
                    writeln!(
                        out,
                        "{indent}{attributes}pub fn set_{:?}<'env>({env_param}) -> {rust_set_type} {{ ... }}",
                        self.java.name(),
                    )?;
                }
            }
        }

        Ok(())
    }
}

struct ConstantWriter<'a>(&'a LiteralConstant<'a>);

// Migrated from <https://docs.rs/jreflection/latest/src/jreflection/field.rs.html#53-73>,
// which seems like a bug for `java-spaghetti`.
impl std::fmt::Display for ConstantWriter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            LiteralConstant::Integer(value) => write!(f, "{}", value),
            LiteralConstant::Long(value) => write!(f, "{}i64", value),

            LiteralConstant::Float(value) if value.is_infinite() && *value < 0.0 => {
                write!(f, "::std::f32::NEG_INFINITY")
            }
            LiteralConstant::Float(value) if value.is_infinite() => write!(f, "::std::f32::INFINITY"),
            LiteralConstant::Float(value) if value.is_nan() => write!(f, "::std::f32::NAN"),
            LiteralConstant::Float(value) => write!(f, "{}f32", value),

            LiteralConstant::Double(value) if value.is_infinite() && *value < 0.0 => {
                write!(f, "::std::f64::NEG_INFINITY")
            }
            LiteralConstant::Double(value) if value.is_infinite() => write!(f, "::std::f64::INFINITY"),
            LiteralConstant::Double(value) if value.is_nan() => write!(f, "::std::f64::NAN"),
            LiteralConstant::Double(value) => write!(f, "{}f64", value),

            LiteralConstant::String(value) => std::fmt::Debug::fmt(value, f),
            LiteralConstant::StringBytes(_) => {
                write!(f, "panic!(\"Java string constant contains invalid 'Modified UTF8'\")")
            }
        }
    }
}

pub struct FieldTypeEmitter<'a>(pub &'a FieldDescriptor<'a>);

impl FieldTypeEmitter<'_> {
    /// Generates the corresponding Rust type for the Java field type.
    pub fn emit_rust_type(
        &self,
        context: &Context<'_>,
        mod_: &str,
        reject_reasons: &mut Vec<&'static str>,
    ) -> Result<Cow<'static, str>, std::fmt::Error> {
        use Cow::Borrowed;
        let descriptor = self.0;
        let cow = if descriptor.dimensions == 0 {
            match &descriptor.field_type {
                FieldType::Boolean => Borrowed("bool"),
                FieldType::Byte => Borrowed("i8"),
                FieldType::Char => Borrowed("u16"),
                FieldType::Short => Borrowed("i16"),
                FieldType::Integer => Borrowed("i32"),
                FieldType::Long => Borrowed("i64"),
                FieldType::Float => Borrowed("f32"),
                FieldType::Double => Borrowed("f64"),
                FieldType::Object(class_name) => {
                    let class = IdBuf::from(class_name);
                    if !context.all_classes.contains_key(class.as_str()) {
                        reject_reasons.push("ERROR:  missing class for field/argument type");
                    }
                    if let Ok(path) = context.java_to_rust_path(class.as_id(), mod_) {
                        path
                    } else {
                        reject_reasons.push("ERROR:  Failed to resolve JNI path to Rust path for class type");
                        format!("{:?}", class) // XXX
                    }
                    .into()
                }
            }
        } else {
            let mut out = String::new();
            for _ in 0..(descriptor.dimensions - 1) {
                write!(out, "::java_spaghetti::ObjectArray<")?;
            }
            match &descriptor.field_type {
                FieldType::Boolean => write!(out, "::java_spaghetti::BooleanArray"),
                FieldType::Byte => write!(out, "::java_spaghetti::ByteArray"),
                FieldType::Char => write!(out, "::java_spaghetti::CharArray"),
                FieldType::Short => write!(out, "::java_spaghetti::ShortArray"),
                FieldType::Integer => write!(out, "::java_spaghetti::IntArray"),
                FieldType::Long => write!(out, "::java_spaghetti::LongArray"),
                FieldType::Float => write!(out, "::java_spaghetti::FloatArray"),
                FieldType::Double => write!(out, "::java_spaghetti::DoubleArray"),
                FieldType::Object(class_name) => {
                    let class = IdBuf::from(class_name);

                    if !context.all_classes.contains_key(class.as_str()) {
                        reject_reasons.push("ERROR:  missing class for field type");
                    }

                    write!(out, "::java_spaghetti::ObjectArray<")?;
                    match context.java_to_rust_path(class.as_id(), mod_) {
                        Ok(path) => write!(out, "{path}"),
                        Err(_) => {
                            reject_reasons.push("ERROR:  Failed to resolve JNI path to Rust path for class type");
                            write!(out, "???")
                        }
                    }?;
                    write!(out, ", ")?;
                    write!(out, "{}", &context.throwable_rust_path(mod_))?;
                    write!(out, ">")
                }
            }?;
            for _ in 0..(descriptor.dimensions - 1) {
                // ObjectArray s
                write!(out, ", ")?;
                write!(out, "{}", &context.throwable_rust_path(mod_))?;
                write!(out, ">")?;
            }
            out.into()
        };
        Ok(cow)
    }

    /// Contents of {get,set}_[static_]..._field, call_..._method_a.
    pub fn emit_fragment_type(&self) -> &'static str {
        if self.0.dimensions == 0 {
            match self.0.field_type {
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
}
