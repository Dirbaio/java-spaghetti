//! java-spaghetti.yaml configuration file structures and parsing APIs.

use std::path::{Path, PathBuf};
use std::{fs, io};

use serde_derive::Deserialize;

fn default_proxy_package() -> String {
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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ClassMatch {
    One(String),
    Many(Vec<String>),
}

impl Default for ClassMatch {
    fn default() -> Self {
        Self::One("**".to_string())
    }
}
impl ClassMatch {
    fn matches(&self, class: &str) -> bool {
        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };

        match self {
            Self::One(p) => {
                let pattern = glob::Pattern::new(p).unwrap_or_else(|e| panic!("Invalid glob pattern '{p}': {e}"));
                pattern.matches_with(class, options)
            }
            Self::Many(pp) => pp.iter().any(|p| {
                let pattern = glob::Pattern::new(p).unwrap_or_else(|e| panic!("Invalid glob pattern '{p}': {e}"));
                pattern.matches_with(class, options)
            }),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Rule {
    /// What java class(es) to match against.  This takes the form of a glob pattern matching JNI paths.
    ///
    /// Glob patterns are case-sensitive and require literal path separators (/ cannot be matched by *).
    /// Use ** to match across directory boundaries.
    ///
    /// | To Match:                 | Use a glob pattern:                   |
    /// | ------------------------- | ------------------------------------- |
    /// | *                         | "*"
    /// | java.lang.*               | "java/lang/**"
    /// | name.spaces.OuterClass.*  | "name/spaces/OuterClass$*"
    /// | Specific class            | "com/example/MyClass"
    /// | Multiple specific classes | ["com/example/Class1", "com/example/Class2"]
    #[serde(rename = "match")]
    pub matches: ClassMatch,

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
    pub input: Vec<PathBuf>,
    pub output: PathBuf,

    #[serde(default = "default_proxy_package")]
    pub proxy_package: String,
    #[serde(default)]
    pub proxy_output: Option<PathBuf>,

    #[serde(default)]
    pub logging_verbose: bool,

    #[serde(default)]
    pub rules: Vec<Rule>,
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
                matches: ClassMatch::default(),
                include: Some(true),
                ..Default::default()
            })
        }

        config.output = resolve_file(&config.output, dir);
        if let Some(proxy_output) = &mut config.proxy_output {
            *proxy_output = resolve_file(proxy_output, dir);
        }
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

    /// Read configuration from a specific file path.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let mut file = fs::File::open(path)?;
        let config_dir = path.parent().unwrap_or(Path::new("."));
        Self::read(&mut file, config_dir)
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
            if r.matches.matches(class) {
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

    #[test]
    fn test_class_match_glob_patterns() {
        // Test exact match
        let match_exact = ClassMatch::One("com/example/MyClass".to_string());
        assert!(match_exact.matches("com/example/MyClass"));
        assert!(!match_exact.matches("com/example/MyOtherClass"));

        // Test wildcard patterns
        let match_wildcard = ClassMatch::One("com/example/*".to_string());
        assert!(match_wildcard.matches("com/example/MyClass"));
        assert!(match_wildcard.matches("com/example/MyOtherClass"));
        assert!(!match_wildcard.matches("com/other/MyClass"));

        // Test question mark pattern
        let match_question = ClassMatch::One("com/example/MyClass?".to_string());
        assert!(match_question.matches("com/example/MyClass1"));
        assert!(match_question.matches("com/example/MyClassA"));
        assert!(!match_question.matches("com/example/MyClass"));
        assert!(!match_question.matches("com/example/MyClass12"));

        // Test multiple patterns
        let match_many = ClassMatch::Many(vec!["com/example/*".to_string(), "org/test/specific/Class".to_string()]);
        assert!(match_many.matches("com/example/MyClass"));
        assert!(match_many.matches("org/test/specific/Class"));
        assert!(!match_many.matches("org/other/MyClass"));
    }

    #[test]
    #[should_panic(expected = "Invalid glob pattern")]
    fn test_class_match_invalid_pattern_panics() {
        let match_invalid = ClassMatch::One("[invalid".to_string());
        match_invalid.matches("any_class");
    }
    #[test]
    fn test_class_match_literal_separator() {
        // Test that require_literal_separator: true prevents * from matching /
        let match_pattern = ClassMatch::One("com/example*".to_string());
        assert!(match_pattern.matches("com/example"));
        assert!(match_pattern.matches("com/exampleClass"));
        assert!(!match_pattern.matches("com/example/SubClass")); // * should not match /

        // Test that we can use ** to match across directories
        let match_recursive = ClassMatch::One("com/**/MyClass".to_string());
        assert!(match_recursive.matches("com/example/MyClass"));
        assert!(match_recursive.matches("com/deep/nested/path/MyClass"));
        assert!(match_recursive.matches("com/MyClass")); // ** can match zero directories too

        // Test that * within a directory works
        let match_single_dir = ClassMatch::One("com/*/MyClass".to_string());
        assert!(match_single_dir.matches("com/example/MyClass"));
        assert!(!match_single_dir.matches("com/deep/nested/MyClass")); // single * doesn't cross /
    }
}
