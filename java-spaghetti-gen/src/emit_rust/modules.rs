use std::collections::BTreeMap;
use std::io::{self, Write};
use std::rc::Rc;

use super::classes::Class;
use crate::emit_rust::Context;

#[derive(Debug, Default)]
pub(crate) struct Module {
    // For consistent diffs / printing order, these should *not* be HashMaps
    pub(crate) classes: BTreeMap<String, Rc<Class>>,
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

        for (_, structure) in self.classes.iter() {
            structure.write(context, out)?;
        }

        Ok(())
    }
}
