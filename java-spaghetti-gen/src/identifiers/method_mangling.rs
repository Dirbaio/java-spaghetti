use anyhow::bail;
use cafebabe::descriptors::{FieldType, MethodDescriptor};
use serde_derive::Deserialize;

use super::rust_identifier::rust_ident;
use crate::parser_util::{Id, IdPart};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MethodManglingStyle {
    /// Leave the original method name alone as much as possible.
    /// Constructors will still be renamed from "\<init>" to "new".
    ///
    /// # Examples:
    ///
    /// | Java      | Rust      |
    /// | --------- | --------- |
    /// | getFoo    | getFoo    |
    /// | \<init\>  | new       |
    Java,

    /// Leave the original method name alone as much as possible... with unqualified typenames appended for disambiguation.
    /// Constructors will still be renamed from "\<init>" to "new".
    ///
    /// # Examples:
    ///
    /// | Java      | Rust          |
    /// | --------- | ------------- |
    /// | getFoo    | getFoo_int    |
    /// | \<init\>  | new_Object    |
    JavaShortSignature,

    /// Leave the original method name alone as much as possible... with qualified typenames appended for disambiguation.
    /// Constructors will still be renamed from "\<init>" to "new".
    ///
    /// # Examples:
    ///
    /// | Java      | Rust                  |
    /// | --------- | --------------------- |
    /// | getFoo    | getFoo_int            |
    /// | \<init\>  | new_java_lang_Object  |
    JavaLongSignature,
}

#[test]
fn method_mangling_style_mangle_test() {
    use std::borrow::Cow;

    use cafebabe::descriptors::{ClassName, FieldDescriptor, ReturnDescriptor};

    let desc_no_arg_ret_v = MethodDescriptor {
        parameters: Vec::new(),
        return_type: ReturnDescriptor::Void,
    };

    let desc_arg_i_ret_v = MethodDescriptor {
        parameters: vec![FieldDescriptor {
            dimensions: 0,
            field_type: FieldType::Integer,
        }],
        return_type: ReturnDescriptor::Void,
    };

    let desc_arg_obj_ret_v = MethodDescriptor {
        parameters: vec![FieldDescriptor {
            dimensions: 0,
            field_type: FieldType::Object(ClassName::try_from(Cow::Borrowed("java/lang/Object")).unwrap()),
        }],
        return_type: ReturnDescriptor::Void,
    };

    for &(name, sig, java, java_short, java_long) in &[
        ("getFoo", &desc_no_arg_ret_v, "getFoo", "getFoo", "getFoo"),
        ("getFoo", &desc_arg_i_ret_v, "getFoo", "getFoo_int", "getFoo_int"),
        (
            "getFoo",
            &desc_arg_obj_ret_v,
            "getFoo",
            "getFoo_Object",
            "getFoo_java_lang_Object",
        ),
        ("<init>", &desc_no_arg_ret_v, "new", "new", "new"),
        ("<init>", &desc_arg_i_ret_v, "new", "new_int", "new_int"),
        (
            "<init>",
            &desc_arg_obj_ret_v,
            "new",
            "new_Object",
            "new_java_lang_Object",
        ),
        // TODO: get1DFoo
        // TODO: array types (primitive + non-primitive)
    ] {
        assert_eq!(MethodManglingStyle::Java.mangle(name, sig).unwrap(), java);
        assert_eq!(
            MethodManglingStyle::JavaShortSignature.mangle(name, sig).unwrap(),
            java_short
        );
        assert_eq!(
            MethodManglingStyle::JavaLongSignature.mangle(name, sig).unwrap(),
            java_long
        );
    }
}

#[test]
fn mangle_method_name_test() {
    use cafebabe::descriptors::{MethodDescriptor, ReturnDescriptor};

    let desc = MethodDescriptor {
        parameters: Vec::new(),
        return_type: ReturnDescriptor::Void,
    };

    assert_eq!(MethodManglingStyle::Java.mangle("isFooBar", &desc).unwrap(), "isFooBar");
    assert_eq!(
        MethodManglingStyle::Java.mangle("XMLHttpRequest", &desc).unwrap(),
        "XMLHttpRequest"
    );
    assert_eq!(
        MethodManglingStyle::Java.mangle("getFieldID_Input", &desc).unwrap(),
        "getFieldID_Input"
    );
}

impl MethodManglingStyle {
    pub fn mangle(&self, name: &str, descriptor: &MethodDescriptor) -> Result<String, anyhow::Error> {
        let name = match name {
            "" => {
                bail!("empty string")
            }
            "<init>" => "new",
            "<clinit>" => {
                bail!("not applicable: Static type ctor")
            }
            name => name,
        };

        let long_sig = match self {
            MethodManglingStyle::Java => return rust_ident(name),
            MethodManglingStyle::JavaShortSignature => false,
            MethodManglingStyle::JavaLongSignature => true,
        };

        let mut buffer = name.to_string();

        for arg in descriptor.parameters.iter() {
            match &arg.field_type {
                FieldType::Boolean => buffer.push_str("_boolean"),
                FieldType::Byte => buffer.push_str("_byte"),
                FieldType::Char => buffer.push_str("_char"),
                FieldType::Short => buffer.push_str("_short"),
                FieldType::Integer => buffer.push_str("_int"),
                FieldType::Long => buffer.push_str("_long"),
                FieldType::Float => buffer.push_str("_float"),
                FieldType::Double => buffer.push_str("_double"),
                FieldType::Object(class_name) => {
                    let class = Id::from(class_name);

                    if long_sig {
                        for component in class.iter() {
                            buffer.push('_');
                            match component {
                                IdPart::Namespace(namespace) => {
                                    buffer.push_str(namespace);
                                }
                                IdPart::ContainingClass(cls) => {
                                    buffer.push_str(cls);
                                }
                                IdPart::LeafClass(cls) => {
                                    buffer.push_str(cls);
                                }
                            }
                        }
                    } else {
                        // short style
                        if let Some(IdPart::LeafClass(leaf)) = class.iter().last() {
                            buffer.push('_');
                            buffer.push_str(leaf);
                        } else if arg.dimensions == 0 {
                            // XXX: `if arg.dimensions == 0` is just keeping the behaviour
                            // before porting to cafebabe, is it a bug?
                            buffer.push_str("_unknown");
                        }
                    }
                }
            };
            for _ in 0..arg.dimensions {
                buffer.push_str("_array");
            }
        }

        rust_ident(&buffer)
    }
}
