//! java-spaghetti.yaml configuration file structures and parsing APIs.

use std::path::{Path, PathBuf};
use std::{fs, io};

use serde_derive::Deserialize;

fn default_proxy_path_prefix() -> String {
    "java_spaghetti/proxy".to_string()
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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Rule {
    /// What java class(es) to match against.  This takes the form of a simple prefix to a JNI path with no wildcards.
    ///
    /// | To Match:                 | Use a JNI Prefix:                     |
    /// | ------------------------- | ------------------------------------- |
    /// | *                         | jni_prefix = ""
    /// | java.lang.*               | jni_prefix = "java/lang/"
    /// | name.spaces.OuterClass.*  | jni_prefix = "name/spaces/OuterClass$"
    #[serde(default)]
    #[serde(rename = "match")]
    pub matches: Vec<String>,

    #[serde(default)]
    pub include: Option<bool>,

    #[serde(default)]
    pub include_private_classes: Option<bool>,
    #[serde(default)]
    pub include_private_methods: Option<bool>,
    #[serde(default)]
    pub include_private_fields: Option<bool>,

    #[serde(default)]
    pub proxy: Option<bool>,

    #[serde(default)]
    pub doc_pattern: Option<DocPattern>,
}

#[derive(Debug, Clone)]
pub struct ClassConfig<'a> {
    pub include: bool,

    pub include_private_classes: bool,
    pub include_private_methods: bool,
    pub include_private_fields: bool,
    pub proxy: bool,
    pub doc_pattern: Option<&'a DocPattern>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_proxy_path_prefix")]
    pub proxy_path_prefix: String,

    pub(crate) input: Vec<PathBuf>,
    pub(crate) output: PathBuf,

    #[serde(default)]
    pub(crate) logging_verbose: bool,

    #[serde(default)]
    pub(crate) rules: Vec<Rule>,
}

impl Config {
    /// Read from I/O, under the assumption that it's in the "java-spaghetti.yaml" file format.
    /// `directory` is the directory that contained the `java-spaghetti.yaml` file, against which paths should be resolved.
    pub fn read(file: &mut impl io::Read, dir: &Path) -> io::Result<Self> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?; // Apparently yaml can't stream.
        Self::read_str(&buffer[..], dir)
    }

    /// Read from a memory buffer, under the assumption that it's in the "java-spaghetti.yaml" file format.
    /// `directory` is the directory that contained the `java-spaghetti.yaml` file, against which paths should be resolved.
    pub fn read_str(buffer: &str, dir: &Path) -> io::Result<Self> {
        let mut config: Config =
            serde_yaml::from_str(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if config.rules.is_empty() {
            config.rules.push(Rule {
                matches: vec!["".to_string()],
                ..Default::default()
            })
        }

        config.output = resolve_file(&config.output, dir);
        for f in &mut config.input {
            *f = resolve_file(f, dir);
        }

        Ok(config)
    }

    /// Search the current directory - or failing that, it's ancestors - until we find "java-spaghetti.yaml" or reach the
    /// root of the filesystem and cannot continue.
    #[allow(dead_code)]
    pub fn from_current_directory() -> io::Result<Self> {
        Self::from_directory(std::env::current_dir()?.as_path())
    }

    /// Search the specified directory - or failing that, it's ancestors - until we find "java-spaghetti.yaml" or reach the
    /// root of the filesystem and cannot continue.
    pub fn from_directory(path: &Path) -> io::Result<Self> {
        let original = path;
        let mut path = path.to_owned();
        loop {
            path.push("java-spaghetti.yaml");
            println!("cargo:rerun-if-changed={}", path.display());
            if path.exists() {
                return Config::read(&mut fs::File::open(&path)?, path.parent().unwrap());
            }
            if !path.pop() || !path.pop() {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Failed to find java-spaghetti.yaml in \"{}\" or any of it's parent directories.",
                        original.display()
                    ),
                ))?;
            }
        }
    }

    pub fn resolve_class(&self, class: &str) -> ClassConfig<'_> {
        let mut res = ClassConfig {
            include: false,
            include_private_classes: false,
            include_private_methods: false,
            include_private_fields: false,
            proxy: false,
            doc_pattern: None,
        };

        for r in &self.rules {
            if r.matches.iter().any(|p| class.starts_with(p)) {
                if let Some(include) = r.include {
                    res.include = include;
                }
                if let Some(include_private_classes) = r.include_private_classes {
                    res.include_private_classes = include_private_classes;
                }
                if let Some(include_private_methods) = r.include_private_methods {
                    res.include_private_methods = include_private_methods;
                }
                if let Some(include_private_fields) = r.include_private_fields {
                    res.include_private_fields = include_private_fields;
                }
                if let Some(proxy) = r.proxy {
                    res.proxy = proxy;
                }
                if let Some(doc_pattern) = &r.doc_pattern {
                    res.doc_pattern = Some(doc_pattern);
                }
            }
        }

        res
    }
}

fn resolve_file(path: &Path, dir: &Path) -> PathBuf {
    let path = expand_vars(&path.to_string_lossy());
    let path: PathBuf = path.into();
    if path.is_relative() { dir.join(path) } else { path }
}

fn expand_vars(string: &str) -> String {
    let mut result = String::new();
    let mut chars = string.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    // ${VAR} format
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    let mut found_close = false;

                    for var_ch in chars.by_ref() {
                        if var_ch == '}' {
                            found_close = true;
                            break;
                        }
                        var_name.push(var_ch);
                    }

                    if found_close && !var_name.is_empty() {
                        if let Ok(value) = std::env::var(&var_name) {
                            result.push_str(&value);
                        } else {
                            // If variable not found, panic
                            panic!("Environment variable '{var_name}' not found");
                        }
                    } else {
                        // Malformed ${...}, keep as is
                        result.push('$');
                        result.push('{');
                        result.push_str(&var_name);
                        if found_close {
                            result.push('}');
                        }
                    }
                } else if next_ch.is_ascii_alphabetic() || next_ch == '_' {
                    // $VAR format (alphanumeric and underscore)
                    let mut var_name = String::new();

                    while let Some(&var_ch) = chars.peek() {
                        if var_ch.is_ascii_alphanumeric() || var_ch == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    if !var_name.is_empty() {
                        if let Ok(value) = std::env::var(&var_name) {
                            result.push_str(&value);
                        } else {
                            // If variable not found, panic
                            panic!("Environment variable '{var_name}' not found");
                        }
                    } else {
                        result.push('$');
                    }
                } else {
                    // $ followed by something else, keep as is
                    result.push('$');
                }
            } else {
                // $ at end of string
                result.push('$');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_vars() {
        // Test ${VAR} format
        unsafe { std::env::set_var("TEST_VAR", "hello") };
        assert_eq!(expand_vars("${TEST_VAR}"), "hello");
        assert_eq!(expand_vars("prefix_${TEST_VAR}_suffix"), "prefix_hello_suffix");

        // Test $VAR format
        assert_eq!(expand_vars("$TEST_VAR"), "hello");
        // Note: In shell, $VAR_suffix would try to expand VAR_suffix, not VAR + _suffix
        // This is correct behavior - use ${VAR}_suffix for the latter
        assert_eq!(expand_vars("prefix_${TEST_VAR}/suffix"), "prefix_hello/suffix");

        // Test literal $ characters
        assert_eq!(expand_vars("$"), "$");
        assert_eq!(expand_vars("$$"), "$$");
        assert_eq!(expand_vars("$123"), "$123");

        // Test malformed syntax
        assert_eq!(expand_vars("${"), "${");
        assert_eq!(expand_vars("${unclosed"), "${unclosed");

        // Clean up
        unsafe { std::env::remove_var("TEST_VAR") };
    }

    #[test]
    #[should_panic(expected = "Environment variable 'NONEXISTENT' not found")]
    fn test_expand_vars_panic_on_missing_var_braces() {
        expand_vars("${NONEXISTENT}");
    }

    #[test]
    #[should_panic(expected = "Environment variable 'NONEXISTENT' not found")]
    fn test_expand_vars_panic_on_missing_var_dollar() {
        expand_vars("$NONEXISTENT");
    }
}
