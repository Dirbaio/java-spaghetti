//! java-spaghetti.toml configuration file structures and parsing APIs.

use std::path::{Path, PathBuf};
use std::{fs, io};

use serde_derive::Deserialize;

use crate::identifiers::FieldManglingStyle;

fn default_proxy_path_prefix() -> String {
    "java_spaghetti/proxy".to_string()
}

/// The \[codegen\] section.
#[derive(Debug, Clone, Deserialize)]
pub struct CodeGen {
    /// How fields should be named.
    #[serde(default = "Default::default")]
    pub field_naming_style: FieldManglingStyle,

    #[serde(default = "default_proxy_path_prefix")]
    pub proxy_path_prefix: String,
}

impl Default for CodeGen {
    fn default() -> Self {
        Self {
            field_naming_style: Default::default(),
            proxy_path_prefix: default_proxy_path_prefix(),
        }
    }
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
pub struct DocumentationPattern {
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

/// The \[documentation\] section.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Documentation {
    /// Documentation sources.  Processed from top to bottom.
    #[serde(rename = "pattern")]
    #[serde(default = "Vec::new")]
    pub patterns: Vec<DocumentationPattern>,
}

/// The \[input\] section.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Input {
    /// `.jar` or `.class` files to scan for JVM class info.
    ///
    /// May in the future add support for `.apk`s, `.aab`s, etc.
    pub files: Vec<PathBuf>,
}

/// The \[output\] section.
#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    /// Target `.rs` file to generate.
    pub path: PathBuf,
}

/// The \[logging\] section.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Logging {
    #[serde(default = "Default::default")]
    pub verbose: bool,
}

/// A \[[rename\]] section.
/// Format for a `java-spaghetti.toml` file or in-memory settings.
///
/// # Example File
///
/// ```toml
/// # For system libraries, you probably only want/need a single documentation URL... but as an example, I have
/// # overridden java.* to use the Oracle Java SE 7 docs instead of the android docs.  More useful if you have a
/// # slew of .jar s from different sources you want to bind all at once, or if the platform documentation is broken
/// # up by top level modules in strange ways.
///
/// [codegen]
/// static_env                      = "implicit"
///
/// [logging]
/// verbose = true
///
/// [[documentation.pattern]]
/// class_url_pattern               = "https://docs.oracle.com/javase/7/docs/api/index.html?java/{PATH}.html"
/// jni_prefix                      = "java/"
/// class_namespace_separator       = "/"
/// class_inner_class_seperator     = "."
/// argument_seperator              = ",%20"
///
/// [[documentation.pattern]]
/// class_url_pattern               = "https://developer.android.com/reference/kotlin/{PATH}.html"
/// jni_prefix                      = ""
/// class_namespace_separator       = "/"
/// class_inner_class_seperator     = "."
///
/// [input]
/// files = [
///     "%LOCALAPPDATA%/Android/Sdk/platforms/android-28/android.jar"
/// ]
///
/// [output]
/// path = "android28.rs"
///
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct File {
    #[serde(default = "Default::default")]
    pub codegen: CodeGen,

    /// Documentation settings.
    #[serde(default = "Default::default")]
    pub documentation: Documentation,

    /// Input(s) into the java-spaghetti-gen process.
    pub input: Input,

    /// Logging settings
    #[serde(default = "Default::default")]
    pub logging: Logging,

    /// Output(s) from the java-spaghetti-gen process.
    pub output: Output,

    /// Classes and class methods to include.
    #[serde(rename = "include")]
    #[serde(default = "Vec::new")]
    pub includes: Vec<String>,

    /// Proxies to include.
    #[serde(rename = "include_proxy")]
    #[serde(default = "Vec::new")]
    pub include_proxies: Vec<String>,
}

impl File {
    /// Read from I/O, under the assumption that it's in the "java-spaghetti.toml" file format.
    pub fn read(file: &mut impl io::Read) -> io::Result<Self> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?; // Apparently toml can't stream.
        Self::read_str(&buffer[..])
    }

    /// Read from a memory buffer, under the assumption that it's in the "java-spaghetti.toml" file format.
    pub fn read_str(buffer: &str) -> io::Result<Self> {
        let file: File = toml::from_str(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(file)
    }

    /// Search the current directory - or failing that, it's ancestors - until we find "java-spaghetti.toml" or reach the
    /// root of the filesystem and cannot continue.
    #[allow(dead_code)]
    pub fn from_current_directory() -> io::Result<FileWithContext> {
        Self::from_directory(std::env::current_dir()?.as_path())
    }

    /// Search the specified directory - or failing that, it's ancestors - until we find "java-spaghetti.toml" or reach the
    /// root of the filesystem and cannot continue.
    pub fn from_directory(path: &Path) -> io::Result<FileWithContext> {
        let original = path;
        let mut path = path.to_owned();
        loop {
            path.push("java-spaghetti.toml");
            println!("cargo:rerun-if-changed={}", path.display());
            if path.exists() {
                let file = File::read(&mut fs::File::open(&path)?)?;
                path.pop();
                return Ok(FileWithContext { file, directory: path });
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

#[test]
fn load_well_configured_toml() {
    let well_configured_toml = r#"
        # For system libraries, you probably only want/need a single documentation URL... but as an example, I have
        # overridden java.* to use the Oracle Java SE 7 docs instead of the android docs.  More useful if you have a
        # slew of .jar s from different sources you want to bind all at once, or if the platform documentation is broken
        # up by top level modules in strange ways.

        [codegen]
        static_env                      = "explicit"

        [logging]
        verbose = true

        [[documentation.pattern]]
        class_url_pattern               = "https://docs.oracle.com/javase/7/docs/api/index.html?java/{CLASS}.html"
        jni_prefix                      = "java/"
        class_namespace_separator       = "/"
        class_inner_class_seperator     = "."
        argument_namespace_separator    = "."
        argument_inner_class_seperator  = "."
        argument_seperator              = ",%20"

        [[documentation.pattern]]
        class_url_pattern               = "https://developer.android.com/reference/kotlin/{CLASS}.html"

        [input]
        files = [
            "%LOCALAPPDATA%/Android/Sdk/platforms/android-28/android.jar"
        ]

        [output]
        path = "android28.rs"


        [[rename]]
        class = "some/java/Class"
        to    = "class"

        [[rename]]
        class  = "some/java/Class"
        method = "someMethod"
        to     = "some_method"

        [[rename]]
        class     = "some/java/Class"
        method    = "someOtherMethod"
        signature = "()V"
        to        = "some_other_method"
    "#;
    let file = File::read_str(well_configured_toml).unwrap();

    assert!(file.logging.verbose);

    assert_eq!(file.documentation.patterns.len(), 2);

    assert_eq!(
        file.documentation.patterns[0].class_url_pattern,
        "https://docs.oracle.com/javase/7/docs/api/index.html?java/{CLASS}.html"
    );
    assert_eq!(file.documentation.patterns[0].jni_prefix, "java/");
    assert_eq!(file.documentation.patterns[0].class_namespace_separator, "/");
    assert_eq!(file.documentation.patterns[0].class_inner_class_seperator, ".");
    assert_eq!(file.documentation.patterns[0].argument_namespace_separator, ".");
    assert_eq!(file.documentation.patterns[0].argument_inner_class_seperator, ".");
    assert_eq!(file.documentation.patterns[0].argument_seperator, ",%20");

    assert_eq!(
        file.documentation.patterns[1].class_url_pattern,
        "https://developer.android.com/reference/kotlin/{CLASS}.html"
    );
    assert_eq!(file.documentation.patterns[1].jni_prefix, "");
    assert_eq!(file.documentation.patterns[1].class_namespace_separator, "/");
    assert_eq!(file.documentation.patterns[1].class_inner_class_seperator, ".");
    assert_eq!(file.documentation.patterns[1].argument_namespace_separator, ".");
    assert_eq!(file.documentation.patterns[1].argument_inner_class_seperator, ".");
    assert_eq!(file.documentation.patterns[1].argument_seperator, ",");

    assert_eq!(
        file.input.files,
        &[Path::new("%LOCALAPPDATA%/Android/Sdk/platforms/android-28/android.jar")]
    );
    assert_eq!(file.output.path, Path::new("android28.rs"));
}

#[test]
fn load_minimal_toml() {
    let minimal_toml = r#"
        [input]
        files = ["%LOCALAPPDATA%/Android/Sdk/platforms/android-28/android.jar"]

        [output]
        path = "android28.rs"
    "#;
    let file = File::read_str(minimal_toml).unwrap();

    assert!(!file.logging.verbose);
    assert_eq!(file.documentation.patterns.len(), 0);
    assert_eq!(
        file.input.files,
        &[Path::new("%LOCALAPPDATA%/Android/Sdk/platforms/android-28/android.jar")]
    );
    assert_eq!(file.output.path, Path::new("android28.rs"));
}

/// A [File] + context (directory path continaing the [File]).
///
/// [File]:         struct.File.html
#[derive(Debug, Clone)]
pub struct FileWithContext {
    /// The parsed `java-spaghetti.toml` configuration file.
    pub file: File,

    /// The directory that contained the `java-spaghetti.toml` file, against which paths should be resolved.
    pub directory: PathBuf,
}
