use std::fmt::{self, Display, Formatter};

use cafebabe::descriptors::{FieldDescriptor, FieldType};

use super::methods::Method;
use crate::config::ClassConfig;
use crate::parser_util::Id;

pub(crate) struct KnownDocsUrl {
    pub(crate) label: String,
    pub(crate) url: String,
}

impl Display for KnownDocsUrl {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "[{}]({})", &self.label, &self.url)
    }
}

impl KnownDocsUrl {
    pub(crate) fn from_class(config: &ClassConfig, java_class: Id) -> Option<KnownDocsUrl> {
        let java_class = java_class.as_str();
        let pattern = config.doc_pattern?;

        for ch in java_class.chars() {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '0'..='9' => {}
                '_' | '$' | '/' => {}
                _ch => return None,
            }
        }

        let last_slash = java_class.rfind('/');
        let no_namespace = if let Some(last_slash) = last_slash {
            &java_class[(last_slash + 1)..]
        } else {
            java_class
        };

        let java_class = java_class
            .replace('/', pattern.class_namespace_separator.as_str())
            .replace('$', pattern.class_inner_class_seperator.as_str());

        Some(KnownDocsUrl {
            label: no_namespace.to_owned().replace('$', "."),
            url: pattern
                .class_url_pattern
                .replace("{CLASS}", java_class.as_str())
                .replace("{CLASS.LOWER}", java_class.to_ascii_lowercase().as_str()),
        })
    }

    pub(crate) fn from_method(config: &ClassConfig, method: &Method) -> Option<KnownDocsUrl> {
        let is_constructor = method.java.is_constructor();

        let pattern = config.doc_pattern?;
        let url_pattern = if is_constructor {
            pattern
                .constructor_url_pattern
                .as_ref()
                .or(pattern.method_url_pattern.as_ref())?
        } else {
            pattern.method_url_pattern.as_ref()?
        };

        for ch in method.class.path().as_str().chars() {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '0'..='9' => {}
                '_' | '$' | '/' => {}
                _ch => return None,
            }
        }

        let java_class = method
            .class
            .path()
            .as_str()
            .replace('/', pattern.class_namespace_separator.as_str())
            .replace('$', pattern.class_inner_class_seperator.as_str());

        let java_outer_class = method
            .class
            .path()
            .as_str()
            .rsplit('/')
            .next()
            .unwrap()
            .replace('$', pattern.class_inner_class_seperator.as_str());

        let java_inner_class = method
            .class
            .path()
            .as_str()
            .rsplit('/')
            .next()
            .unwrap()
            .rsplit('$')
            .next()
            .unwrap();

        let label = if is_constructor {
            java_inner_class
        } else {
            for ch in method.java.name().chars() {
                match ch {
                    'a'..='z' => {}
                    'A'..='Z' => {}
                    '0'..='9' => {}
                    '_' => {}
                    _ch => return None,
                }
            }
            method.java.name()
        };

        let mut java_args = String::new();

        let mut prev_was_array = false;
        for arg in method.java.descriptor().parameters.iter() {
            if prev_was_array {
                prev_was_array = false;
                java_args.push_str("[]");
            }

            if !java_args.is_empty() {
                java_args.push_str(&pattern.argument_seperator[..]);
            }

            let obj_arg;
            java_args.push_str(match arg.field_type {
                FieldType::Boolean => "boolean",
                FieldType::Byte => "byte",
                FieldType::Char => "char",
                FieldType::Short => "short",
                FieldType::Integer => "int",
                FieldType::Long => "long",
                FieldType::Float => "float",
                FieldType::Double => "double",
                FieldType::Object(ref class_name) => {
                    let class = Id::from(class_name);
                    obj_arg = class
                        .as_str()
                        .replace('/', pattern.argument_namespace_separator.as_str())
                        .replace('$', pattern.argument_inner_class_seperator.as_str());
                    obj_arg.as_str()
                }
            });
            if arg.dimensions > 0 {
                for _ in 1..arg.dimensions {
                    java_args.push_str("[]");
                }
                prev_was_array = true; // level 0
            }
        }

        if prev_was_array {
            if method.java.is_varargs() {
                java_args.push_str("...");
            } else {
                java_args.push_str("[]");
            }
        }

        // No {RETURN} support... yet?

        Some(KnownDocsUrl {
            label: label.to_owned(),
            url: url_pattern
                .replace("{CLASS}", java_class.as_str())
                .replace("{CLASS.LOWER}", java_class.to_ascii_lowercase().as_str())
                .replace("{CLASS.OUTER}", java_outer_class.as_str())
                .replace("{CLASS.INNER}", java_inner_class)
                .replace("{METHOD}", label)
                .replace("{ARGUMENTS}", java_args.as_str()),
        })
    }

    pub(crate) fn from_field(
        config: &ClassConfig,
        java_class: &str,
        java_field: &str,
        _java_descriptor: FieldDescriptor,
    ) -> Option<KnownDocsUrl> {
        let pattern = config.doc_pattern?;
        let field_url_pattern = pattern.field_url_pattern.as_ref()?;

        for ch in java_class.chars() {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '0'..='9' => {}
                '_' | '$' | '/' => {}
                _ch => return None,
            }
        }

        for ch in java_field.chars() {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '0'..='9' => {}
                '_' => {}
                _ch => return None,
            }
        }

        let java_class = java_class
            .replace('/', pattern.class_namespace_separator.as_str())
            .replace('$', pattern.class_inner_class_seperator.as_str());

        // No {RETURN} support... yet?

        Some(KnownDocsUrl {
            label: java_field.to_owned(),
            url: field_url_pattern
                .replace("{CLASS}", java_class.as_str())
                .replace("{CLASS.LOWER}", java_class.to_ascii_lowercase().as_str())
                .replace("{FIELD}", java_field),
        })
    }
}
