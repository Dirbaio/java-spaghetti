use core::fmt;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::io;
use std::rc::Rc;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::classes::Class;
use crate::emit::Context;

#[derive(Debug, Default)]
pub(crate) struct Module {
    // For consistent diffs / printing order, these should *not* be HashMaps
    pub(crate) classes: BTreeMap<String, Rc<Class>>,
    pub(crate) modules: BTreeMap<String, Module>,
}

impl Module {
    pub(crate) fn write(&self, context: &Context, out: &mut impl io::Write) -> anyhow::Result<()> {
        for (name, module) in self.modules.iter() {
            writeln!(out)?;

            writeln!(out, "pub mod {name} {{")?;
            module.write(context, out)?;
            writeln!(out, "}}")?;
        }

        for (_, class) in self.classes.iter() {
            let res = class.write(context)?;
            out.write_all(dumb_format(res).as_bytes())?;
        }

        Ok(())
    }
}

/// Convert tokenstream to string, doing a best-effort formatting
/// inserting newlines at `;` and `{}`.
///
/// The user is supposed to run the output through `rustfmt`, this is
/// intended just to prevent the output from being a single huge line
/// to make debugging syntax errors easier.
fn dumb_format(ts: TokenStream) -> String {
    let mut f = DumbFormatter {
        space: false,
        after_newline: true,
        indent: 1,
        f: String::new(),
    };
    f.format_tokenstream(ts);
    f.f
}

struct DumbFormatter {
    space: bool,
    after_newline: bool,
    indent: usize,
    f: String,
}

impl DumbFormatter {
    fn newline(&mut self) {
        self.f.push('\n');
        for _ in 0..self.indent {
            self.f.push_str("    ");
        }

        self.after_newline = true;
    }

    fn pre_write(&mut self) {
        if self.space && !self.after_newline {
            self.f.push(' ');
        }
        self.space = false;
        self.after_newline = false;
    }

    fn write_str(&mut self, s: &str) {
        self.pre_write();
        self.f.push_str(s);
    }

    fn write_display(&mut self, d: impl fmt::Display) {
        self.pre_write();
        write!(&mut self.f, "{d}").unwrap();
    }

    fn format_tokenstream(&mut self, ts: TokenStream) {
        for tt in ts {
            match tt {
                TokenTree::Group(tt) => {
                    let (open, close) = match tt.delimiter() {
                        Delimiter::Parenthesis => ("(", ")"),
                        Delimiter::Brace => ("{ ", "}"),
                        Delimiter::Bracket => ("[", "]"),
                        Delimiter::None => ("", ""),
                    };

                    self.write_str(open);
                    let ts = tt.stream();
                    let empty = ts.is_empty();
                    if tt.delimiter() == Delimiter::Brace && !empty {
                        self.indent += 1;
                        self.newline();
                    }
                    self.format_tokenstream(ts);
                    if tt.delimiter() == Delimiter::Brace && !empty {
                        self.write_str(" ");
                        self.indent -= 1;
                    }
                    self.write_str(close);
                    if tt.delimiter() == Delimiter::Brace {
                        self.newline();
                    }
                }
                TokenTree::Ident(tt) => {
                    self.write_display(tt);
                    self.space = true;
                }
                TokenTree::Punct(tt) => {
                    self.write_display(&tt);
                    if tt.spacing() == Spacing::Alone {
                        self.space = true;
                    }

                    if tt.as_char() == ';' {
                        self.newline();
                    }
                }
                TokenTree::Literal(tt) => {
                    self.write_display(tt);
                    self.space = true;
                }
            };
        }
    }
}
