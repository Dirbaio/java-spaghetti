use anyhow::bail;

/// Takes an arbitrary string and tries to treat it as a Rust identifier, doing minor escaping for keywords.
pub fn rust_ident(s: &str) -> Result<String, anyhow::Error> {
    match s {
        // [Strict keywords](https://doc.rust-lang.org/reference/keywords.html#strict-keywords) that *are not* valid
        // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
        "crate" | "extern" | "self" | "super" | "Self" => Ok(format!("{s}_")),

        // [Strict keywords](https://doc.rust-lang.org/reference/keywords.html#strict-keywords) that *are* valid
        // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
        "as" | "break" | "const" | "continue" | "else" | "enum" | "false" | "fn" | "for" | "if" | "impl" | "in"
        | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "static" | "struct"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "dyn" |

        // [Reserved keywords](https://doc.rust-lang.org/reference/keywords.html#reserved-keywords) that *are* valid
        // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
        "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof" | "unsized"
        | "virtual" | "yield" |

        // 2018 edition
        "async" | "await" | "try" |

        // 2024 edition
        "gen" |

        // [Weak keywords](https://doc.rust-lang.org/reference/keywords.html#weak-keywords) that *are* valid
        // [RAW_IDENTIFIER](https://doc.rust-lang.org/reference/identifiers.html)s
        "union" => Ok(format!("r#{s}")),

        s if is_rust_ident(s) => Ok(s.to_string()),
        s if is_rust_ident(&format!("_{s}")) => Ok(format!("_{s}")),
        s => bail!("invalid rust identifier '{s}'"),
    }
}

fn is_rust_ident(s: &str) -> bool {
    // https://doc.rust-lang.org/reference/identifiers.html
    !s.is_empty()
        && s != "_"
        && s.chars().enumerate().all(|(i, ch)| {
            if i == 0 {
                ch.is_ascii_alphabetic() || ch == '_'
            } else {
                ch.is_ascii_alphanumeric() || ch == '_'
            }
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rust_ident_test() {
        assert_eq!(rust_ident("foo").unwrap(), "foo");
        assert_eq!(rust_ident("crate").unwrap(), "crate_");
        assert_eq!(rust_ident("match").unwrap(), "r#match");
        assert!(rust_ident("föo").is_err());
        assert!(rust_ident("").is_err());
        assert_eq!(rust_ident("_").unwrap(), "__");
        assert_eq!(rust_ident("_f").unwrap(), "_f");
        assert_eq!(rust_ident("_1").unwrap(), "_1");
        assert_eq!(rust_ident("1_").unwrap(), "_1_");
        assert_eq!(rust_ident("1").unwrap(), "_1");
    }

    #[test]
    fn is_rust_ident_test() {
        // Valid identifiers
        assert!(is_rust_ident("foo"));
        assert!(is_rust_ident("_foo"));
        assert!(is_rust_ident("foo_bar"));
        assert!(is_rust_ident("foo123"));
        assert!(is_rust_ident("_123"));
        assert!(is_rust_ident("__"));
        assert!(is_rust_ident("a"));

        // Invalid identifiers
        assert!(!is_rust_ident("")); // empty string
        assert!(!is_rust_ident("_")); // single underscore
        assert!(!is_rust_ident("123")); // starts with digit
        assert!(!is_rust_ident("123foo")); // starts with digit
        assert!(!is_rust_ident("foo-bar")); // contains hyphen
        assert!(!is_rust_ident("foo.bar")); // contains dot
        assert!(!is_rust_ident("föo")); // non-ASCII character
        assert!(!is_rust_ident("foo bar")); // contains space
    }
}
