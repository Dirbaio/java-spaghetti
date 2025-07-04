use crate::identifiers::rust_ident;
use crate::parser_util::JavaField;

pub enum FieldMangling<'a> {
    ConstValue(String, cafebabe::constant_pool::LiteralConstant<'a>),
    GetSet(String, String),
}

pub fn mangle_field<'a>(field: JavaField<'a>) -> Result<FieldMangling<'a>, anyhow::Error> {
    let field_name = field.name();
    if let Some(value) = field.constant().as_ref() {
        let name = rust_ident(field_name)?;
        Ok(FieldMangling::ConstValue(name, value.clone()))
    } else {
        Ok(FieldMangling::GetSet(
            rust_ident(field_name)?,
            rust_ident(&format!("set_{field_name}"))?,
        ))
    }
}
