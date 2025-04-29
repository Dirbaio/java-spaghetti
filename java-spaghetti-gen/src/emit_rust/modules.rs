use std::collections::BTreeMap;
use std::io::Write;
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
    pub(crate) fn write(&self, context: &Context, out: &mut impl Write) -> anyhow::Result<()> {
        for (name, module) in self.modules.iter() {
            writeln!(out)?;

            writeln!(out, "pub mod {} {{", name)?;
            module.write(context, out)?;
            writeln!(out, "}}")?;
        }

        for (_, class) in self.classes.iter() {
            let res = class.write(context)?;
            out.write(res.to_string().as_bytes())?;
        }

        Ok(())
    }
}
