use crate::identifiers::{IdentifierManglingError, javaify_identifier};
use crate::parser_util::JavaField;

pub enum FieldMangling<'a> {
    ConstValue(String, cafebabe::constant_pool::LiteralConstant<'a>),
    GetSet(String, String),
}

pub fn mangle_field<'a>(field: JavaField<'a>) -> Result<FieldMangling<'a>, IdentifierManglingError> {
    let field_name = field.name();
    if let Some(value) = field.constant().as_ref() {
        let name = javaify_identifier(field_name)?;
        Ok(FieldMangling::ConstValue(name, value.clone()))
    } else {
        Ok(FieldMangling::GetSet(
            javaify_identifier(field_name)?,
            javaify_identifier(&format!("set_{field_name}"))?,
        ))
    }
}
