use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

use jreflection::class;

use super::modules::Module;
use super::preamble::write_preamble;
use super::structs::Struct;
use crate::{config, util};

pub struct Context<'a> {
    pub(crate) config: &'a config::runtime::Config,
    pub(crate) module: Module,
    pub(crate) all_classes: HashSet<String>,
    pub(crate) progress: Mutex<util::Progress>,
}

impl<'a> Context<'a> {
    pub fn new(config: &'a config::runtime::Config) -> Self {
        Self {
            config,
            module: Default::default(),
            all_classes: HashSet::new(),
            progress: Mutex::new(util::Progress::with_duration(Duration::from_millis(
                if config.logging_verbose { 0 } else { 300 },
            ))),
        }
    }

    pub(crate) fn throwable_rust_path(&self, mod_: &str) -> String {
        self.java_to_rust_path(class::Id("java/lang/Throwable"), mod_).unwrap()
    }

    pub fn java_to_rust_path(&self, java_class: class::Id, mod_: &str) -> Result<String, Box<dyn Error>> {
        let m = Struct::mod_for(self, java_class)?;
        let s = Struct::name_for(self, java_class)?;
        let fqn = format!("{}::{}", m, s);

        // Calculate relative path from B to A.
        let b: Vec<&str> = mod_.split("::").collect();
        let a: Vec<&str> = fqn.split("::").collect();

        let mut ma = &a[..a.len() - 1];
        let mut mb = &b[..];
        while !ma.is_empty() && !mb.is_empty() && ma[0] == mb[0] {
            ma = &ma[1..];
            mb = &mb[1..];
        }

        let mut res = String::new();

        // for each item left in b, append a `super`
        for _ in mb {
            res.push_str("super::");
        }

        // for each item in a, append it
        for ident in ma {
            res.push_str(ident);
            res.push_str("::");
        }

        res.push_str(a[a.len() - 1]);

        Ok(res)
    }

    fn struct_included(&self, path: &str) -> bool {
        if self.config.include_classes.contains(path) {
            return true;
        }
        if self.config.include_classes.contains("*") {
            return true;
        }

        let mut pat = String::new();
        for p in path.split('/') {
            pat.push_str(p);
            if pat.len() == path.len() {
                break;
            }

            pat.push('/');
            pat.push('*');
            if self.config.include_classes.contains(&pat) {
                return true;
            }
            pat.pop();
        }

        return false;
    }

    pub fn add_struct(&mut self, class: jreflection::Class) -> Result<(), Box<dyn Error>> {
        if self.config.ignore_classes.contains(class.path.as_str()) {
            return Ok(());
        }
        if !self.struct_included(class.path.as_str()) {
            return Ok(());
        }

        self.all_classes.insert(class.path.as_str().to_string());

        let s = Struct::new(self, class)?;

        let mut rust_mod = &mut self.module;
        for fragment in s.rust.mod_.split("::") {
            rust_mod = rust_mod.modules.entry(fragment.to_owned()).or_default();
        }
        if rust_mod.structs.contains_key(&s.rust.struct_name) {
            return io_data_err!(
                "Unable to add_struct(): java class name {:?} was already added",
                &s.rust.struct_name
            )?;
        }
        rust_mod.structs.insert(s.rust.struct_name.clone(), s);

        Ok(())
    }

    pub fn write(&self, out: &mut impl io::Write) -> io::Result<()> {
        write_preamble(out)?;
        self.module.write(self, "", out)
    }
}
