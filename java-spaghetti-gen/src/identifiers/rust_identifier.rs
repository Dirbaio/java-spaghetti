/// Categorizes a rust [identifier](https://doc.rust-lang.org/reference/identifiers.html) for use in rust codegen.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RustIdentifier {
    /// Meets the criteria for a Rust [NON_KEYWORD_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)
    Identifier(String),

    /// Not a rust-safe [identifier](https://doc.rust-lang.org/reference/identifiers.html).  Unicode, strange ASCII
    /// values, relatively normal ASCII values... you name it.
    NonIdentifier(String),

    /// A [keyword](https://doc.rust-lang.org/reference/keywords.html) that has had `r#` prepended to it, because it can
    /// be used as a [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)
    KeywordRawSafe(String),

    /// A [keyword](https://doc.rust-lang.org/reference/keywords.html) that has had `_` postpended to it, because it can
    /// *not* be used as a [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html).
    KeywordUnderscorePostfix(String),
}

impl RustIdentifier {
    /// Takes an arbitrary string and tries to treat it as a Rust identifier, doing minor escaping for keywords.
    pub fn from_str(s: &str) -> RustIdentifier {
        match s {
            // [Strict keywords](https://doc.rust-lang.org/reference/keywords.html#strict-keywords) that *are not* valid
            // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
            "crate" => RustIdentifier::KeywordUnderscorePostfix("crate_".to_string()),
            "extern" => RustIdentifier::KeywordUnderscorePostfix("extern_".to_string()),
            "self" => RustIdentifier::KeywordUnderscorePostfix("self_".to_string()),
            "super" => RustIdentifier::KeywordUnderscorePostfix("super_".to_string()),
            "Self" => RustIdentifier::KeywordUnderscorePostfix("Self_".to_string()),

            // [Strict keywords](https://doc.rust-lang.org/reference/keywords.html#strict-keywords) that *are* valid
            // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
            "as" => RustIdentifier::KeywordRawSafe("r#as".to_string()),
            "break" => RustIdentifier::KeywordRawSafe("r#break".to_string()),
            "const" => RustIdentifier::KeywordRawSafe("r#const".to_string()),
            "continue" => RustIdentifier::KeywordRawSafe("r#continue".to_string()),
            "else" => RustIdentifier::KeywordRawSafe("r#else".to_string()),
            "enum" => RustIdentifier::KeywordRawSafe("r#enum".to_string()),
            "false" => RustIdentifier::KeywordRawSafe("r#false".to_string()),
            "fn" => RustIdentifier::KeywordRawSafe("r#fn".to_string()),
            "for" => RustIdentifier::KeywordRawSafe("r#for".to_string()),
            "if" => RustIdentifier::KeywordRawSafe("r#if".to_string()),
            "impl" => RustIdentifier::KeywordRawSafe("r#impl".to_string()),
            "in" => RustIdentifier::KeywordRawSafe("r#in".to_string()),
            "let" => RustIdentifier::KeywordRawSafe("r#let".to_string()),
            "loop" => RustIdentifier::KeywordRawSafe("r#loop".to_string()),
            "match" => RustIdentifier::KeywordRawSafe("r#match".to_string()),
            "mod" => RustIdentifier::KeywordRawSafe("r#mod".to_string()),
            "move" => RustIdentifier::KeywordRawSafe("r#move".to_string()),
            "mut" => RustIdentifier::KeywordRawSafe("r#mut".to_string()),
            "pub" => RustIdentifier::KeywordRawSafe("r#pub".to_string()),
            "ref" => RustIdentifier::KeywordRawSafe("r#ref".to_string()),
            "return" => RustIdentifier::KeywordRawSafe("r#return".to_string()),
            "static" => RustIdentifier::KeywordRawSafe("r#static".to_string()),
            "struct" => RustIdentifier::KeywordRawSafe("r#struct".to_string()),
            "trait" => RustIdentifier::KeywordRawSafe("r#trait".to_string()),
            "true" => RustIdentifier::KeywordRawSafe("r#true".to_string()),
            "type" => RustIdentifier::KeywordRawSafe("r#type".to_string()),
            "unsafe" => RustIdentifier::KeywordRawSafe("r#unsafe".to_string()),
            "use" => RustIdentifier::KeywordRawSafe("r#use".to_string()),
            "where" => RustIdentifier::KeywordRawSafe("r#where".to_string()),
            "while" => RustIdentifier::KeywordRawSafe("r#while".to_string()),
            "dyn" => RustIdentifier::KeywordRawSafe("r#dyn".to_string()),

            // [Reserved keywords](https://doc.rust-lang.org/reference/keywords.html#reserved-keywords) that *are* valid
            // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
            "abstract" => RustIdentifier::KeywordRawSafe("r#abstract".to_string()),
            "become" => RustIdentifier::KeywordRawSafe("r#become".to_string()),
            "box" => RustIdentifier::KeywordRawSafe("r#box".to_string()),
            "do" => RustIdentifier::KeywordRawSafe("r#do".to_string()),
            "final" => RustIdentifier::KeywordRawSafe("r#final".to_string()),
            "macro" => RustIdentifier::KeywordRawSafe("r#macro".to_string()),
            "override" => RustIdentifier::KeywordRawSafe("r#override".to_string()),
            "priv" => RustIdentifier::KeywordRawSafe("r#priv".to_string()),
            "typeof" => RustIdentifier::KeywordRawSafe("r#typeof".to_string()),
            "unsized" => RustIdentifier::KeywordRawSafe("r#unsized".to_string()),
            "virtual" => RustIdentifier::KeywordRawSafe("r#virtual".to_string()),
            "yield" => RustIdentifier::KeywordRawSafe("r#yield".to_string()),
            // 2018 edition
            "async" => RustIdentifier::KeywordRawSafe("r#async".to_string()),
            "await" => RustIdentifier::KeywordRawSafe("r#await".to_string()),
            "try" => RustIdentifier::KeywordRawSafe("r#try".to_string()),

            // [Weak keywords](https://doc.rust-lang.org/reference/keywords.html#weak-keywords) that *are* valid
            // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
            "union" => RustIdentifier::KeywordRawSafe("r#union".to_string()),

            // Not a keyword, but not a valid [IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html) either.
            "" => RustIdentifier::NonIdentifier(s.to_string()),
            "_" => RustIdentifier::NonIdentifier(s.to_string()),
            s if is_rust_identifier(s) => RustIdentifier::Identifier(s.to_string()),
            s if is_rust_identifier(&format!("_{s}")) => RustIdentifier::Identifier(format!("_{s}")),
            s => RustIdentifier::NonIdentifier(s.to_string()),
        }
    }
}

#[test]
fn rust_identifier_from_str() {
    assert_eq!(
        RustIdentifier::from_str("foo"),
        RustIdentifier::Identifier("foo".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("crate"),
        RustIdentifier::KeywordUnderscorePostfix("crate_".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("match"),
        RustIdentifier::KeywordRawSafe("r#match".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("föo"),
        RustIdentifier::NonIdentifier("föo".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str(""),
        RustIdentifier::NonIdentifier("".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("_"),
        RustIdentifier::NonIdentifier("_".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("_f"),
        RustIdentifier::Identifier("_f".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("_1"),
        RustIdentifier::Identifier("_1".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("1_"),
        RustIdentifier::Identifier("_1_".to_string())
    );
    assert_eq!(
        RustIdentifier::from_str("1"),
        RustIdentifier::Identifier("_1".to_string())
    );
}

fn is_rust_identifier(s: &str) -> bool {
    // https://doc.rust-lang.org/reference/identifiers.html
    let mut chars = s.chars();

    // First char
    let first_char = if let Some(ch) = chars.next() { ch } else { return false };
    match first_char {
        'a'..='z' => {}
        'A'..='Z' => {}
        '_' => {
            if s == "_" {
                return false;
            }
        }
        _ => return false,
    }

    // Subsequent chars
    for ch in chars {
        match ch {
            'a'..='z' => {}
            'A'..='Z' => {}
            '0'..='9' => {}
            '_' => {}
            _ => return false,
        }
    }

    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdentifierManglingError {
    NotApplicable(&'static str),
    EmptyString,
    NotRustSafe,
    UnexpectedCharacter(char),
}

impl std::error::Error for IdentifierManglingError {}
impl std::fmt::Display for IdentifierManglingError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, fmt)
    }
}

pub fn javaify_identifier(name: &str) -> Result<String, IdentifierManglingError> {
    if name == "_" {
        Ok(String::from("__"))
    } else {
        let mut chars = name.chars();

        // First character
        if let Some(ch) = chars.next() {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '_' => {}
                _ => {
                    return Err(IdentifierManglingError::UnexpectedCharacter(ch));
                }
            }
        }

        // Subsequent characters
        for ch in chars {
            match ch {
                'a'..='z' => {}
                'A'..='Z' => {}
                '0'..='9' => {}
                '_' => {}
                _ => {
                    return Err(IdentifierManglingError::UnexpectedCharacter(ch));
                }
            }
        }

        match RustIdentifier::from_str(name) {
            RustIdentifier::Identifier(_) => Ok(name.to_owned()),
            RustIdentifier::NonIdentifier(_) => Err(IdentifierManglingError::NotRustSafe),
            RustIdentifier::KeywordRawSafe(s) => Ok(s.to_owned()),
            RustIdentifier::KeywordUnderscorePostfix(s) => Ok(s.to_owned()),
        }
    }
}

pub fn rustify_identifier(name: &str) -> Result<String, IdentifierManglingError> {
    if name == "_" {
        Ok(String::from("__"))
    } else {
        let mut chars = name.chars();
        let mut buffer = String::new();
        let mut uppercase = 0;

        // First character
        if let Some(ch) = chars.next() {
            match ch {
                'a'..='z' => buffer.push(ch),
                'A'..='Z' => {
                    buffer.push(ch.to_ascii_lowercase());
                    uppercase = 1;
                }
                '_' => buffer.push(ch),
                _ => {
                    return Err(IdentifierManglingError::UnexpectedCharacter(ch));
                }
            }
        }

        // Subsequent characters
        for ch in chars {
            if ch.is_ascii_uppercase() {
                if uppercase == 0 && !buffer.ends_with('_') {
                    buffer.push('_');
                }
                buffer.push(ch.to_ascii_lowercase());
                uppercase += 1;
            } else if ch.is_ascii_alphanumeric() {
                if uppercase > 1 {
                    buffer.insert(buffer.len() - 1, '_');
                }
                buffer.push(ch);
                uppercase = 0;
            } else if ch == '_' {
                buffer.push(ch);
                uppercase = 0;
            } else {
                return Err(IdentifierManglingError::UnexpectedCharacter(ch));
            }
        }

        match RustIdentifier::from_str(&buffer) {
            RustIdentifier::Identifier(_) => Ok(buffer),
            RustIdentifier::NonIdentifier(_) => Err(IdentifierManglingError::NotRustSafe),
            RustIdentifier::KeywordRawSafe(s) => Ok(s.to_owned()),
            RustIdentifier::KeywordUnderscorePostfix(s) => Ok(s.to_owned()),
        }
    }
}

pub fn constify_identifier(name: &str) -> Result<String, IdentifierManglingError> {
    if name == "_" {
        Ok(String::from("__"))
    } else {
        let mut chars = name.chars();
        let mut buffer = String::new();
        let mut uppercase = 0;
        let mut lowercase = 0;

        // First character
        if let Some(ch) = chars.next() {
            match ch {
                'a'..='z' => {
                    buffer.push(ch.to_ascii_uppercase());
                    lowercase = 1;
                }
                'A'..='Z' => {
                    buffer.push(ch);
                    uppercase = 1;
                }
                '_' => buffer.push(ch),
                _ => {
                    return Err(IdentifierManglingError::UnexpectedCharacter(ch));
                }
            }
        }

        // Subsequent characters
        for ch in chars {
            let is_upper = ch.is_ascii_uppercase();
            let is_lower = ch.is_ascii_lowercase();
            let is_numeric = ch.is_numeric();

            if is_lower && uppercase > 1 {
                buffer.insert(buffer.len() - 1, '_');
            } else if is_upper && lowercase > 0 {
                buffer.push('_');
            }

            uppercase = if is_upper { uppercase + 1 } else { 0 };
            lowercase = if is_lower { lowercase + 1 } else { 0 };

            if is_lower {
                buffer.push(ch.to_ascii_uppercase());
            } else if is_upper || is_numeric || ch == '_' {
                buffer.push(ch);
            } else {
                return Err(IdentifierManglingError::UnexpectedCharacter(ch));
            }
        }

        match RustIdentifier::from_str(&buffer) {
            RustIdentifier::Identifier(_) => Ok(buffer),
            RustIdentifier::NonIdentifier(_) => Err(IdentifierManglingError::NotRustSafe),
            RustIdentifier::KeywordRawSafe(s) => Ok(s.to_owned()),
            RustIdentifier::KeywordUnderscorePostfix(s) => Ok(s.to_owned()),
        }
    }
}
