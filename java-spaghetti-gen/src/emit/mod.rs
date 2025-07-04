//! Rust generation logic

mod class_proxy;
mod classes;
mod fields;
pub mod java_proxy;
mod known_docs_url;
mod methods;
mod modules;
mod preamble;

use std::collections::HashMap;
use std::ffi::CString;
use std::io;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

use proc_macro2::{Literal, TokenStream};
use quote::{TokenStreamExt, format_ident, quote};

use self::classes::Class;
use self::modules::Module;
use self::preamble::write_preamble;
use crate::{config, parser_util, util};

pub struct Context<'a> {
    pub(crate) config: &'a config::Config,
    pub(crate) module: Module,
    pub(crate) all_classes: HashMap<String, Rc<Class>>,
    pub(crate) progress: Mutex<util::Progress>,
}

impl<'a> Context<'a> {
    pub fn new(config: &'a config::Config) -> Self {
        Self {
            config,
            module: Default::default(),
            all_classes: HashMap::new(),
            progress: Mutex::new(util::Progress::with_duration(Duration::from_millis(
                if config.logging_verbose { 0 } else { 300 },
            ))),
        }
    }

    pub(crate) fn throwable_rust_path(&self, mod_: &str) -> TokenStream {
        self.java_to_rust_path(parser_util::Id("java/lang/Throwable"), mod_)
            .unwrap()
    }

    pub fn java_to_rust_path(&self, java_class: parser_util::Id, mod_: &str) -> Result<TokenStream, anyhow::Error> {
        let m = Class::mod_for(java_class)?;
        let s = Class::name_for(java_class)?;
        let fqn = format!("{m}::{s}");

        // Calculate relative path from B to A.
        let b: Vec<&str> = mod_.split("::").collect();
        let a: Vec<&str> = fqn.split("::").collect();

        let mut ma = &a[..a.len() - 1];
        let mut mb = &b[..];
        while !ma.is_empty() && !mb.is_empty() && ma[0] == mb[0] {
            ma = &ma[1..];
            mb = &mb[1..];
        }

        let mut res = TokenStream::new();

        // for each item left in b, append a `super`
        for _ in mb {
            res.extend(quote!(super::));
        }

        // for each item in a, append it
        for ident in ma {
            let ident = format_ident!("{}", ident);
            res.extend(quote!(#ident::));
        }

        let ident = format_ident!("{}", a[a.len() - 1]);
        res.append(ident);

        Ok(res)
    }

    pub fn add_class(&mut self, class: parser_util::JavaClass) -> Result<(), anyhow::Error> {
        let cc = self.config.resolve_class(class.path().as_str());
        if !cc.include {
            return Ok(());
        }

        let java_path = class.path().as_str().to_string();
        let s = Rc::new(Class::new(class)?);

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

    pub fn write(&self, out: &mut impl io::Write) -> anyhow::Result<()> {
        write_preamble(out)?;
        self.module.write(self, out)
    }
}

fn cstring(s: &str) -> Literal {
    Literal::c_string(&CString::from_str(s).unwrap())
}
