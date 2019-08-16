use super::*;

use java::class;

use std::collections::*;
use std::error::Error;
use std::io;
use std::time::*;
use std::sync::Mutex;

pub struct Context<'a> {
    pub(crate) config:      &'a config::runtime::Config,
    pub(crate) module:      Module,
    pub(crate) features:    BTreeMap<String, BTreeSet<String>>,
    pub(crate) progress:    Mutex<util::Progress>,
}

impl<'a> Context<'a> {
    pub fn new(config: &'a config::runtime::Config) -> Self {
        Self {
            config,
            module: Default::default(),
            features: BTreeMap::new(),
            progress: Mutex::new(util::Progress::with_duration(Duration::from_millis(if config.logging_verbose { 0 } else { 300 }))),
        }
    }

    pub fn java_to_rust_path(&self, java_class: class::Id) -> Result<String, Box<dyn Error>> {
        let m = Struct::mod_for(self, java_class)?;
        let s = Struct::name_for(self, java_class)?;
        Ok(format!("{}::{}", m, s))
    }

    pub fn add_struct(&mut self, class: java::Class) -> Result<(), Box<dyn Error>> {
        let s = Struct::new(self, class)?;
        let scope = if let Some(s) = s.rust.local_scope() { s } else { /* !local_scope = not part of this module, skip! */ return Ok(()); };

        if self.config.codegen.feature_per_struct {
            if let Ok(feature) = Struct::feature_for(self, s.java.path.as_id()) {
                let mut subfeature = None;
                if let Some(parent) = s.java.super_path.as_ref() {
                    if let Ok(sf) = Struct::feature_for(self, parent.as_id()) {
                        subfeature = Some(sf);
                    }
                }
                let subfeatures = self.features.entry(feature).or_insert(BTreeSet::new());
                if let Some(subfeature) = subfeature {
                    subfeatures.insert(subfeature);
                }
            }
        }

        let mut rust_mod = &mut self.module;
        for fragment in scope {
            rust_mod = rust_mod.modules.entry(fragment.to_owned()).or_insert(Default::default());
        }
        if rust_mod.structs.contains_key(&s.rust.struct_name) {
            return io_data_err!("Unable to add_struct(): java class name {:?} was already added", &s.rust.struct_name)?
        }
        rust_mod.structs.insert(s.rust.struct_name.clone(), s);

        Ok(())
    }

    pub fn write(&self, out: &mut impl io::Write) -> io::Result<()> {
        write_preamble(out)?;
        self.module.write(self, "", out)
    }

    pub fn completed_file(&self, file: util::GeneratedFile) -> io::Result<()> {
        // TODO:  Add options to check if the file changed instead of clobbering the old file.
        file.clobber()?;
        Ok(())
    }
}
