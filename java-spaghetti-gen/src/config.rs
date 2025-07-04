//! java-spaghetti.toml configuration file structures and parsing APIs.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use serde_derive::Deserialize;

fn default_proxy_path_prefix() -> String {
    "java_spaghetti/proxy".to_string()
}

fn default_empty() -> String {
    String::new()
}
fn default_slash() -> String {
    String::from("/")
}
fn default_period() -> String {
    String::from(".")
}
fn default_comma() -> String {
    String::from(",")
}

/// A \[\[documentation.pattern\]\] section.
#[derive(Debug, Clone, Deserialize)]
pub struct DocPattern {
    /// The URL to use for documenting a given class.  `{CLASS}` will be replaced with everything *after* the JNI prefix.
    ///
    /// | Given:                | Use this if you want android documentation:   |
    /// | --------------------- | --------------------------------------------- |
    /// | jni_prefix = "java/"  | class_url_pattern = "https://developer.android.com/reference/java/{CLASS}.html"
    /// | jni_prefix = ""       | class_url_pattern = "https://developer.android.com/reference/{CLASS}.html"
    pub class_url_pattern: String,

    /// The URL to use for documenting a given class method.
    ///
    /// * `{CLASS}` will be replaced with everything *after* the JNI prefix.
    /// * `{METHOD}` will be replaced with the method name.
    /// * `{ARGUMENTS}` will be replaced with the method arguments.
    ///
    /// | Given:                | Use this if you want android documentation:   |
    /// | --------------------- | --------------------------------------------- |
    /// | jni_prefix = "java/"  | method_url_pattern = "https://developer.android.com/reference/java/{CLASS}.html#{METHOD}({ARGUMENTS})"
    /// | jni_prefix = ""       | method_url_pattern = "https://developer.android.com/reference/{CLASS}.html#{METHOD}({ARGUMENTS})"
    pub method_url_pattern: Option<String>,

    /// The URL to use for documenting a given class constructor.
    ///
    /// * `{CLASS}` will be replaced with everything *after* the JNI prefix.
    /// * `{CLASS.OUTER}` will be replaced with just the class name, including the outer class(es)
    /// * `{CLASS.INNER}` will be replaced with just the class name, excluding the outer class(es)
    /// * `{METHOD}` aliases `{CLASS.INNER}`
    /// * `{ARGUMENTS}` will be replaced with the method arguments.
    ///
    /// Defaults to method_url_pattern
    ///
    /// | Given:                | Use this if you want android documentation:   |
    /// | --------------------- | --------------------------------------------- |
    /// | jni_prefix = "java/"  | constructor_url_pattern = "https://developer.android.com/reference/java/{CLASS}.html#{CLASS.INNER}({ARGUMENTS})"
    /// | jni_prefix = ""       | constructor_url_pattern = "https://developer.android.com/reference/{CLASS}.html#{CLASS.INNER}({ARGUMENTS})"
    pub constructor_url_pattern: Option<String>,

    /// The URL to use for documenting a given class field.
    ///
    /// * `{CLASS}` will be replaced with everything *after* the JNI prefix.
    /// * `{FIELD}` will be replaced with the field name.
    ///
    /// | Given:                | Use this if you want android documentation:   |
    /// | --------------------- | --------------------------------------------- |
    /// | jni_prefix = "java/"  | field_url_pattern = "https://developer.android.com/reference/java/{CLASS}.html#{FIELD}"
    /// | jni_prefix = ""       | field_url_pattern = "https://developer.android.com/reference/{CLASS}.html#{FIELD}"
    pub field_url_pattern: Option<String>,

    /// What java class(es) to match against.  This takes the form of a simple prefix to a JNI path with no wildcards.
    ///
    /// | To Match:                 | Use a JNI Prefix:                     |
    /// | ------------------------- | ------------------------------------- |
    /// | *                         | jni_prefix = ""
    /// | java.lang.*               | jni_prefix = "java/lang/"
    /// | name.spaces.OuterClass.*  | jni_prefix = "name/spaces/OuterClass$"
    #[serde(default = "default_empty")]
    pub jni_prefix: String,

    /// What to use in the {CLASS} portion of URLs to separate namespaces.  Defaults to "/".
    #[serde(default = "default_slash")]
    pub class_namespace_separator: String,

    /// What to use in the {CLASS} portion of URLs to separate inner classes from outer classes.  Defaults to ".".
    #[serde(default = "default_period")]
    pub class_inner_class_seperator: String,

    /// What to use in the {ARGUMENTS} portion of URLs to separate namespaces.  Defaults to ".".
    #[serde(default = "default_period")]
    pub argument_namespace_separator: String,

    /// What to use in the {ARGUMENTS} portion of URLs to separate inner classes from outer classes.  Defaults to ".".
    #[serde(default = "default_period")]
    pub argument_inner_class_seperator: String,

    /// What to use in the {ARGUMENTS} portion of URLs to separate inner classes from outer classes.  Defaults to ",".
    #[serde(default = "default_comma")]
    pub argument_seperator: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_proxy_path_prefix")]
    pub proxy_path_prefix: String,

    pub(crate) input: Vec<PathBuf>,
    pub(crate) output: PathBuf,

    #[serde(default)]
    pub(crate) doc_patterns: Vec<DocPattern>,
    #[serde(default)]
    pub(crate) logging_verbose: bool,

    #[serde(rename = "include")]
    #[serde(default)]
    pub(crate) include_classes: HashSet<String>,

    #[serde(rename = "include_proxy")]
    #[serde(default = "HashSet::new")]
    pub(crate) include_proxies: HashSet<String>,
}

impl Config {
    /// Read from I/O, under the assumption that it's in the "java-spaghetti.toml" file format.
    /// `directory` is the directory that contained the `java-spaghetti.toml` file, against which paths should be resolved.
    pub fn read(file: &mut impl io::Read, dir: &Path) -> io::Result<Self> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?; // Apparently toml can't stream.
        Self::read_str(&buffer[..], dir)
    }

    /// Read from a memory buffer, under the assumption that it's in the "java-spaghetti.toml" file format.
    /// `directory` is the directory that contained the `java-spaghetti.toml` file, against which paths should be resolved.
    pub fn read_str(buffer: &str, dir: &Path) -> io::Result<Self> {
        let mut config: Config = toml::from_str(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if config.include_classes.is_empty() {
            config.include_classes.insert("*".to_string());
        }

        config.output = resolve_file(&config.output, dir);
        for f in &mut config.input {
            *f = resolve_file(f, dir);
        }

        Ok(config)
    }

    /// Search the current directory - or failing that, it's ancestors - until we find "java-spaghetti.toml" or reach the
    /// root of the filesystem and cannot continue.
    #[allow(dead_code)]
    pub fn from_current_directory() -> io::Result<Self> {
        Self::from_directory(std::env::current_dir()?.as_path())
    }

    /// Search the specified directory - or failing that, it's ancestors - until we find "java-spaghetti.toml" or reach the
    /// root of the filesystem and cannot continue.
    pub fn from_directory(path: &Path) -> io::Result<Self> {
        let original = path;
        let mut path = path.to_owned();
        loop {
            path.push("java-spaghetti.toml");
            println!("cargo:rerun-if-changed={}", path.display());
            if path.exists() {
                return Config::read(&mut fs::File::open(&path)?, path.parent().unwrap());
            }
            if !path.pop() || !path.pop() {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Failed to find java-spaghetti.toml in \"{}\" or any of it's parent directories.",
                        original.display()
                    ),
                ))?;
            }
        }
    }
}

fn resolve_file(path: &Path, dir: &Path) -> PathBuf {
    let path = expand_vars(&path.to_string_lossy());
    let path: PathBuf = path.into();
    if path.is_relative() { dir.join(path) } else { path }
}

fn expand_vars(string: &str) -> String {
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
