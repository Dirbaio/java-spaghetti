//! Runtime configuration formats.  By design, this is mostly opaque - create these from tomls instead.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::config::toml;

pub(crate) struct DocPattern {
    pub(crate) class_url_pattern: String,
    pub(crate) method_url_pattern: Option<String>,
    pub(crate) constructor_url_pattern: Option<String>,
    pub(crate) field_url_pattern: Option<String>,
    pub(crate) jni_prefix: String,
    pub(crate) class_namespace_separator: String,
    pub(crate) class_inner_class_seperator: String,
    pub(crate) argument_namespace_separator: String,
    pub(crate) argument_inner_class_seperator: String,
    pub(crate) argument_seperator: String,
}

impl From<toml::DocumentationPattern> for DocPattern {
    fn from(file: toml::DocumentationPattern) -> Self {
        Self {
            class_url_pattern: file.class_url_pattern,
            method_url_pattern: file.method_url_pattern,
            constructor_url_pattern: file.constructor_url_pattern,
            field_url_pattern: file.field_url_pattern,
            jni_prefix: file.jni_prefix,
            class_namespace_separator: file.class_namespace_separator,
            class_inner_class_seperator: file.class_inner_class_seperator,
            argument_namespace_separator: file.argument_namespace_separator,
            argument_inner_class_seperator: file.argument_inner_class_seperator,
            argument_seperator: file.argument_seperator,
        }
    }
}

/// Runtime configuration.  Create from a toml::File.
pub struct Config {
    pub(crate) codegen: toml::CodeGen,
    pub(crate) doc_patterns: Vec<DocPattern>,
    pub(crate) input_files: Vec<PathBuf>,
    pub(crate) output_path: PathBuf,
    pub(crate) logging_verbose: bool,

    pub(crate) include_classes: HashSet<String>,

    pub(crate) include_proxies: HashSet<String>,
}

impl From<toml::FileWithContext> for Config {
    fn from(fwc: toml::FileWithContext) -> Self {
        let file = fwc.file;
        let dir = fwc.directory;

        let documentation = file.documentation;
        let logging = file.logging;

        let mut include_classes: HashSet<String> = HashSet::new();
        for include in file.includes {
            include_classes.insert(include);
        }
        if include_classes.is_empty() {
            include_classes.insert("*".to_string());
        }

        let mut include_proxies: HashSet<String> = HashSet::new();
        for include in file.include_proxies {
            include_proxies.insert(include);
        }

        let output_path = resolve_file(file.output.path, &dir);

        Self {
            codegen: file.codegen.clone(),
            doc_patterns: documentation.patterns.into_iter().map(|pat| pat.into()).collect(),
            input_files: file
                .input
                .files
                .into_iter()
                .map(|file| resolve_file(file, &dir))
                .collect(),
            output_path,
            logging_verbose: logging.verbose,
            include_classes,
            include_proxies,
        }
    }
}

fn resolve_file(path: PathBuf, dir: &Path) -> PathBuf {
    let path: PathBuf = match path.into_os_string().into_string() {
        Ok(string) => OsString::from(expand_vars(string)),
        Err(os_string) => os_string,
    }
    .into();

    if path.is_relative() { dir.join(path) } else { path }
}

fn expand_vars(string: String) -> String {
    let mut buf = String::new();

    let mut expanding = false;
    for segment in string.split('%') {
        if expanding {
            if let Ok(replacement) = std::env::var(segment) {
                buf.push_str(&replacement[..]);
            } else {
                println!("cargo:rerun-if-env-changed={segment}");
                buf.push('%');
                buf.push_str(segment);
                buf.push('%');
            }
        } else {
            buf.push_str(segment);
        }
        expanding = !expanding;
    }
    assert!(
        expanding,
        "Uneven number of %s in path: {:?}, would mis-expand into: {:?}",
        &string, &buf
    );
    buf
}
