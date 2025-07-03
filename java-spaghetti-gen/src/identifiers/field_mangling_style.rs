use serde_derive::Deserialize;

use crate::identifiers::{IdentifierManglingError, javaify_identifier};
use crate::parser_util::JavaField;

pub enum FieldMangling<'a> {
    ConstValue(String, cafebabe::constant_pool::LiteralConstant<'a>),
    GetSet(String, String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct FieldManglingStyle {
    pub const_finals: bool,     // Default: true
    pub getter_pattern: String, // Default: "{NAME}", might consider "get_{NAME}"
    pub setter_pattern: String, // Default: "set_{NAME}"
}

impl Default for FieldManglingStyle {
    fn default() -> Self {
        Self {
            const_finals: true,
            getter_pattern: String::from("{NAME}"),
            setter_pattern: String::from("set_{NAME}"),
        }
    }
}

impl FieldManglingStyle {
    pub fn mangle<'a>(
        &self,
        field: JavaField<'a>,
        renamed_to: Option<&str>,
    ) -> Result<FieldMangling<'a>, IdentifierManglingError> {
        let field_name = renamed_to.unwrap_or(field.name());
        if let (Some(value), true) = (field.constant().as_ref(), self.const_finals) {
            let name = if renamed_to.is_some() {
                Ok(field_name.to_owned()) // Don't remangle renames
            } else {
                javaify_identifier(field_name)
            }?;

            Ok(FieldMangling::ConstValue(name, value.clone()))
        } else {
            Ok(FieldMangling::GetSet(
                self.mangle_identifier(self.getter_pattern.replace("{NAME}", field_name).as_str())?,
                self.mangle_identifier(self.setter_pattern.replace("{NAME}", field_name).as_str())?,
            ))
        }
    }

    fn mangle_identifier(&self, ident: &str) -> Result<String, IdentifierManglingError> {
        javaify_identifier(ident)
    }
}
