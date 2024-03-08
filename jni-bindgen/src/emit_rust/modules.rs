use std::collections::BTreeMap;
use std::io::{self, Write};

use super::structs::Struct;
use crate::emit_rust::Context;

#[derive(Debug, Default)]
pub(crate) struct Module {
    // For consistent diffs / printing order, these should *not* be HashMaps
    pub(crate) structs: BTreeMap<String, Struct>,
    pub(crate) modules: BTreeMap<String, Module>,
}

impl Module {
    pub(crate) fn write(&self, context: &Context, indent: &str, out: &mut impl Write) -> io::Result<()> {
        let next_indent = format!("{}    ", indent);

        for (name, module) in self.modules.iter() {
            writeln!(out)?;

            writeln!(out, "{}pub mod {} {{", indent, name)?;
            writeln!(out, "{}    use super::__jni_bindgen;", indent)?;
            module.write(context, &next_indent[..], out)?;
            writeln!(out, "{}}}", indent)?;
        }

        for (_, structure) in self.structs.iter() {
            structure.write(context, indent, out)?;
        }

        Ok(())
    }
}
