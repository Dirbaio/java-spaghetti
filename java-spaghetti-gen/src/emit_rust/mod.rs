//! Rust generation logic

mod classes;
mod fields;
mod known_docs_url;
mod methods;
mod modules;
mod preamble;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use self::classes::Class;
use self::modules::Module;
use self::preamble::write_preamble;
use crate::{config, parser_util, util};

pub struct Context<'a> {
    pub(crate) config: &'a config::runtime::Config,
    pub(crate) module: Module,
    pub(crate) all_classes: HashMap<String, Rc<Class>>,
    pub(crate) progress: Mutex<util::Progress>,
}

impl<'a> Context<'a> {
    pub fn new(config: &'a config::runtime::Config) -> Self {
        Self {
            config,
            module: Default::default(),
            all_classes: HashMap::new(),
            progress: Mutex::new(util::Progress::with_duration(Duration::from_millis(
                if config.logging_verbose { 0 } else { 300 },
            ))),
        }
    }

    pub(crate) fn throwable_rust_path(&self, mod_: &str) -> String {
        self.java_to_rust_path(parser_util::Id("java/lang/Throwable"), mod_)
            .unwrap()
    }

    pub fn java_to_rust_path(&self, java_class: parser_util::Id, mod_: &str) -> Result<String, Box<dyn Error>> {
        let m = Class::mod_for(self, java_class)?;
        let s = Class::name_for(self, java_class)?;
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

    fn class_included(&self, path: &str) -> bool {
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

        false
    }

    pub fn add_class(&mut self, class: parser_util::JavaClass) -> Result<(), Box<dyn Error>> {
        if self.config.ignore_classes.contains(class.path().as_str()) {
            return Ok(());
        }
        if !self.class_included(class.path().as_str()) {
            return Ok(());
        }

        let java_path = class.path().as_str().to_string();
        let s = Rc::new(Class::new(self, class)?);

        self.all_classes.insert(java_path, s.clone());

        let mut rust_mod = &mut self.module;
        for fragment in s.rust.mod_.split("::") {
            rust_mod = rust_mod.modules.entry(fragment.to_owned()).or_default();
        }
        if rust_mod.classes.contains_key(&s.rust.struct_name) {
            return io_data_err!(
                "Unable to add_class(): java class name {:?} was already added",
                &s.rust.struct_name
            )?;
        }
        rust_mod.classes.insert(s.rust.struct_name.clone(), s);

        Ok(())
    }

    pub fn write(&self, out: &mut impl io::Write) -> io::Result<()> {
        write_preamble(out)?;
        self.module.write(self, out)
    }
}

/// Writes the string as a C string literal.
///
/// XXX: This implementation (as well as `Env` methods in `java-spaghetti` crate)
/// should probably use byte slices so that full Unicode support can be made possible:
/// JNI `GetFieldID` and `GetMethodID` expects *modified* UTF-8 string name and signature.
/// Note: `cafebabe` converts modified UTF-8 string to real UTF-8 string at first hand.
struct CStrEmitter<T: std::fmt::Display>(pub T);

impl<T: std::fmt::Display> std::fmt::Display for CStrEmitter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("c\"")?;
        self.0.fmt(f)?;
        f.write_str("\"")
    }
}
