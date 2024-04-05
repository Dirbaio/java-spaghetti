use std::collections::BTreeMap;
use std::io::{self, Write};
use std::rc::Rc;

use super::structs::Struct;
use crate::emit_rust::Context;

#[derive(Debug, Default)]
pub(crate) struct Module {
    // For consistent diffs / printing order, these should *not* be HashMaps
    pub(crate) structs: BTreeMap<String, Rc<Struct>>,
    pub(crate) modules: BTreeMap<String, Module>,
}

impl Module {
    pub(crate) fn write(&self, context: &Context, out: &mut impl Write) -> io::Result<()> {
        for (name, module) in self.modules.iter() {
            writeln!(out)?;

            writeln!(out, "pub mod {} {{", name)?;
            module.write(context, out)?;
            writeln!(out, "}}")?;
        }

        for (_, structure) in self.structs.iter() {
            structure.write(context, out)?;
        }

        Ok(())
    }
}
