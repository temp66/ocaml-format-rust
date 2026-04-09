//! A partial Rust port of the [OCaml Format module](https://ocaml.org/manual/5.4/api/Format.html).
//!
//! See the link above or [Formatting and Wrapping Text · OCaml Documentation](https://ocaml.org/docs/formatting-text) for documentation.
//!
//! Differences from the original OCaml implementation:
//!
//! * The OCaml implementation outputs incrementally, whereas this port builds the full document tree and outputs all at once.
//!
//! * The OCaml implementation treats the number of Unicode scalar values as the text width, whereas this port uses the [`unicode_width`](https://crates.io/crates/unicode-width) crate to get a more accurate measurement.
//!
//! * The OCaml implementation does not allow setting `max_indent` to less than or equal to 1, whereas this port does not impose this restriction.
//!
//! # Examples
//!
//! ```
//! use std::fmt::{self, Display, Formatter};
//!
//! use ocaml_format::{Doc, FormattingOptions};
//!
//! enum Lambda {
//!     Var(Box<str>),
//!     Abs(Box<str>, Box<Lambda>),
//!     App(Box<Lambda>, Box<Lambda>),
//! }
//!
//! struct Ident<'a>(&'a str);
//!
//! impl<'a> Display for Ident<'a> {
//!     fn fmt(&self, f: &mut Formatter) -> fmt::Result {
//!         write!(f, "{}", self.0)
//!     }
//! }
//!
//! struct Keyword<'a>(&'a str);
//!
//! impl<'a> Display for Keyword<'a> {
//!     fn fmt(&self, f: &mut Formatter) -> fmt::Result {
//!         write!(f, "{}", self.0)
//!     }
//! }
//!
//! impl Lambda {
//!     fn build_doc<'a>(&'a self, doc: &mut Doc<'a>) {
//!         match self {
//!             Lambda::Var(ident) => doc.atom(Ident(ident)),
//!             Lambda::Abs(param, body) => doc.sbox(1, |doc| {
//!                 doc.atom_fn(|f| write!(f, "({}{}{}", Keyword("λ"), Ident(param), Keyword(".")))
//!                     .space();
//!                 body.build_doc(doc);
//!                 doc.atom(")");
//!             }),
//!             Lambda::App(left, right) => doc.sbox(1, |doc| {
//!                 doc.atom("(");
//!                 left.build_doc(doc);
//!                 doc.space();
//!                 right.build_doc(doc);
//!                 doc.atom(")");
//!             }),
//!         };
//!         ()
//!     }
//! }
//!
//! fn main() {
//!     let x: Box<str> = "x".into();
//!     let expr = Lambda::Abs(
//!         x.clone(),
//!         Box::new(Lambda::App(
//!             Box::new(Lambda::Abs(x.clone(), Box::new(Lambda::Var(x.clone())))),
//!             Box::new(Lambda::Var(x.clone())),
//!         )),
//!     );
//!
//!     let mut doc = Doc::new();
//!     expr.build_doc(&mut doc);
//!     assert_eq!(
//!         format!("{}", doc.display(&FormattingOptions::new().set_width(10))),
//!         "\
//! (λx.
//!  ((λx. x)
//!   x))",
//!     );
//! }
//! ```

use fmt_width::FmtFnWrapper;

use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
};

mod convert;
mod fmt_width;

#[derive(Clone, Debug)]
pub struct FormattingOptions {
    width: usize,
    max_indent: usize,
}

impl FormattingOptions {
    /// Creates a `FormattingOptions` with defaults that align with the OCaml implementation.
    ///
    /// `width` is `78`, `max_indent` is `68`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reflects the `margin` in the OCaml implementation.
    ///
    /// Sets line width limit.
    pub fn set_width(mut self, width: usize) -> Self {
        if width == 0 {
            return self;
        }

        if self.max_indent > width {
            self.max_indent = (self.max_indent + width)
                .saturating_sub(self.width)
                .max(width / 2)
                .max(1);
        }
        self.width = width;
        self
    }

    /// Reflects the `max_indent` in the OCaml implementation.
    pub fn set_max_indent(mut self, max_indent: usize) -> Self {
        self.max_indent = max_indent;
        self
    }
}

/// Aligns with the defaults of the OCaml implementation.
///
/// `width` is `78`, `max_indent` is `68`.
impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            width: 78,
            max_indent: 68,
        }
    }
}

type FmtFn<'a> = dyn Fn(&mut Formatter) -> fmt::Result + 'a;

type FmtFnSend<'a> = dyn Fn(&mut Formatter) -> fmt::Result + Send + 'a;

type FmtFnSync<'a> = dyn Fn(&mut Formatter) -> fmt::Result + Sync + 'a;

type FmtFnSendSync<'a> = dyn Fn(&mut Formatter) -> fmt::Result + Send + Sync + 'a;

/// A sequence of formatting directives and content, representing a formatted document or a fragment of it.
pub struct Doc<'a, F: ?Sized + 'a = FmtFn<'a>> {
    items: Vec<DocItem<'a, F>>,
    flat_width: usize,
    head_segment_flat_width: usize,
    last_format_break_index: Option<usize>,
    _marker: PhantomData<&'a ()>,
}

/// [`Doc`] that implements [`Send`].
pub type DocSend<'a> = Doc<'a, FmtFnSend<'a>>;

/// [`Doc`] that implements [`Sync`].
pub type DocSync<'a> = Doc<'a, FmtFnSync<'a>>;

/// [`Doc`] that implements [`Send`] and [`Sync`].
pub type DocSendSync<'a> = Doc<'a, FmtFnSendSync<'a>>;

enum DocItem<'a, F: ?Sized + 'a> {
    FormatBox(FormatBox<'a, F>),
    Atom(Atom<F>),
    FormatBreak(FormatBreak),
}

struct FormatBox<'a, F: ?Sized + 'a> {
    kind: FormatBoxKind,
    indent: usize,
    doc: Doc<'a, F>,
}

#[derive(Clone, Copy)]
enum FormatBoxKind {
    H,
    V,
    Hv,
    HovP,
    HovS,
}

struct Atom<F: ?Sized> {
    fmt_fn: Box<F>,
    width: usize,
}

struct FormatBreak {
    spaces: usize,
    indent: usize,
    segment_flat_width: usize,
}

/// Builder pattern methods.
impl<'a, F: ?Sized + 'a> Doc<'a, F> {
    /// Creates an empty `Doc`.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            flat_width: 0,
            head_segment_flat_width: 0,
            last_format_break_index: None,
            _marker: PhantomData,
        }
    }

    fn add_segment_flat_width(&mut self, delta: usize) {
        match self.last_format_break_index {
            None => self.head_segment_flat_width += delta,
            Some(index) => {
                let DocItem::FormatBreak(FormatBreak {
                    segment_flat_width, ..
                }) = &mut self.items[index]
                else {
                    unreachable!();
                };
                *segment_flat_width += delta;
            }
        }
    }

    /// Appends a horizontal box (h box, `hbox`) to the document.
    pub fn hbox(&mut self, build_doc: impl FnOnce(&mut Self)) -> &mut Self {
        self.format_box(FormatBoxKind::H, 0, build_doc)
    }

    /// Appends a vertical box (v box, `vbox`) to the document.
    pub fn vbox(&mut self, indent: usize, build_doc: impl FnOnce(&mut Self)) -> &mut Self {
        self.format_box(FormatBoxKind::V, indent, build_doc)
    }

    /// Appends a horizontal/vertical box (hv box, `hvbox`) to the document.
    pub fn hvbox(&mut self, indent: usize, build_doc: impl FnOnce(&mut Self)) -> &mut Self {
        self.format_box(FormatBoxKind::Hv, indent, build_doc)
    }

    /// Appends a horizontal-or-vertical packing box (hov packing box, `hovbox`) to the document.
    pub fn hovbox(&mut self, indent: usize, build_doc: impl FnOnce(&mut Self)) -> &mut Self {
        self.format_box(FormatBoxKind::HovP, indent, build_doc)
    }

    /// Appends a horizontal-or-vertical structural box (hov structural box, `box`) to the document.
    pub fn sbox(&mut self, indent: usize, build_doc: impl FnOnce(&mut Self)) -> &mut Self {
        self.format_box(FormatBoxKind::HovS, indent, build_doc)
    }

    fn format_box(
        &mut self,
        kind: FormatBoxKind,
        indent: usize,
        build_doc: impl FnOnce(&mut Self),
    ) -> &mut Self {
        let mut doc = Self::new();
        build_doc(&mut doc);
        self.flat_width += doc.flat_width;
        self.add_segment_flat_width(doc.flat_width);
        self.items
            .push(DocItem::FormatBox(FormatBox { kind, indent, doc }));
        self
    }

    fn atom_inner(&mut self, atom: Atom<F>) -> &mut Self {
        self.flat_width += atom.width;
        self.add_segment_flat_width(atom.width);
        self.items.push(DocItem::Atom(atom));
        self
    }

    /// Appends a break hint to the document.
    pub fn format_break(&mut self, spaces: usize, indent: usize) -> &mut Self {
        self.flat_width += spaces;
        self.last_format_break_index = Some(self.items.len());
        self.items.push(DocItem::FormatBreak(FormatBreak {
            spaces,
            indent,
            segment_flat_width: spaces,
        }));
        self
    }

    /// Appends a breaking space to the document.
    ///
    /// Convenience method for `format_break(1, 0)`.
    pub fn space(&mut self) -> &mut Self {
        self.format_break(1, 0)
    }

    /// Appends a newline hint to the document.
    ///
    /// Convenience method for `format_break(0, 0)`.
    pub fn cut(&mut self) -> &mut Self {
        self.format_break(0, 0)
    }

    /// Extends the document with the items of another `Doc`.
    pub fn extend(&mut self, doc: impl Into<Self>) -> &mut Self {
        let doc = doc.into();
        self.flat_width += doc.flat_width;
        self.add_segment_flat_width(doc.head_segment_flat_width);
        if doc.last_format_break_index.is_some() {
            self.last_format_break_index = doc.last_format_break_index;
        }
        self.items.extend(doc.items);
        self
    }

    /// Returns a value that implements [`Display`] to format the document with the given options.
    pub fn display(&self, options: &'a FormattingOptions) -> DocDisplay<'_, F> {
        DocDisplay { doc: self, options }
    }
}

/// Creates an empty `Doc`.
impl<'a, F: ?Sized + 'a> Default for Doc<'a, F> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern methods.
impl<'a> Doc<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(&mut self, fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + 'a) -> &mut Self {
        let width = fmt_width::width_of(FmtFnWrapper::new(&fmt_fn));
        self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        })
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(&mut self, d: impl Display + 'a) -> &mut Self {
        let width = fmt_width::width_of(&d);
        self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        })
    }
}

/// Builder pattern methods.
impl<'a> DocSend<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        &mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + 'a,
    ) -> &mut Self {
        let width = fmt_width::width_of(FmtFnWrapper::new(&fmt_fn));
        self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        })
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(&mut self, d: impl Display + Send + 'a) -> &mut Self {
        let width = fmt_width::width_of(&d);
        self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        })
    }
}

/// Builder pattern methods.
impl<'a> DocSync<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        &mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Sync + 'a,
    ) -> &mut Self {
        let width = fmt_width::width_of(FmtFnWrapper::new(&fmt_fn));
        self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        })
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(&mut self, d: impl Display + Sync + 'a) -> &mut Self {
        let width = fmt_width::width_of(&d);
        self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        })
    }
}

/// Builder pattern methods.
impl<'a> DocSendSync<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        &mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + Sync + 'a,
    ) -> &mut Self {
        let width = fmt_width::width_of(FmtFnWrapper::new(&fmt_fn));
        self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        })
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(&mut self, d: impl Display + Send + Sync + 'a) -> &mut Self {
        let width = fmt_width::width_of(&d);
        self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        })
    }
}

/// A helper type created by [`Doc::display`] that implements [`Display`].
#[derive(Clone, Copy)]
pub struct DocDisplay<'a, F: ?Sized + 'a> {
    doc: &'a Doc<'a, F>,
    options: &'a FormattingOptions,
}

impl<'a, F: ?Sized + Fn(&mut Formatter) -> fmt::Result + 'a> Display for DocDisplay<'a, F> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // As per `pp_rinit`, `pp_open_sys_box`, and `pp_make_formatter`.
        // Note `self` is wrapped in an "hovbox(0)".
        Engine {
            options: self.options,
            f,
            caret_pos: 0,
            just_newline: true,
            prev_indent: 0,
        }
        .fmt(FormatBoxKind::HovP, 0, self.doc)
    }
}

struct Engine<'a, 'b> {
    options: &'a FormattingOptions,
    f: &'a mut Formatter<'b>,
    caret_pos: usize,
    just_newline: bool,
    prev_indent: usize,
}

impl<'a, 'b> Engine<'a, 'b> {
    fn fmt(
        &mut self,
        format_box_kind: FormatBoxKind,
        format_box_indent: usize,
        doc: &Doc<impl ?Sized + Fn(&mut Formatter) -> fmt::Result>,
    ) -> fmt::Result {
        let curr_indent = self.caret_pos + format_box_indent;
        let fmt_newline = |engine: &mut Self, format_break_indent| {
            engine.fmt_newline(curr_indent + format_break_indent)
        };

        let fmt_format_break_spaces =
            |engine: &mut Self, format_break: &FormatBreak| engine.fmt_spaces(format_break.spaces);
        let fmt_format_break_newline = |engine: &mut Self, format_break: &FormatBreak| {
            fmt_newline(engine, format_break.indent)
        };
        let fmt_format_break: Box<dyn Fn(&mut Self, &FormatBreak) -> _> = match format_box_kind {
            FormatBoxKind::H => Box::new(fmt_format_break_spaces),
            FormatBoxKind::V => Box::new(fmt_format_break_newline),
            FormatBoxKind::Hv => {
                if self.caret_pos + doc.flat_width <= self.options.width {
                    Box::new(fmt_format_break_spaces)
                } else {
                    Box::new(fmt_format_break_newline)
                }
            }
            FormatBoxKind::HovP => Box::new(|engine, format_break| {
                if engine.caret_pos + format_break.segment_flat_width <= engine.options.width {
                    engine.fmt_spaces(format_break.spaces)
                } else {
                    fmt_newline(engine, format_break.indent)
                }
            }),
            FormatBoxKind::HovS => {
                if self.caret_pos + doc.flat_width <= self.options.width {
                    Box::new(fmt_format_break_spaces)
                } else {
                    Box::new(|engine, format_break| {
                        if engine.just_newline {
                            engine.fmt_spaces(format_break.spaces)
                        } else if engine.caret_pos + format_break.segment_flat_width
                            > engine.options.width
                            || curr_indent + format_break.indent < engine.prev_indent
                        {
                            fmt_newline(engine, format_break.indent)
                        } else {
                            engine.fmt_spaces(format_break.spaces)
                        }
                    })
                }
            }
        };
        doc.items.iter().try_for_each(|doc_item| match doc_item {
            DocItem::FormatBox(FormatBox { kind, indent, doc }) => {
                if self.caret_pos > self.options.max_indent && self.caret_pos > curr_indent {
                    fmt_newline(self, 0)?;
                }
                self.fmt(*kind, *indent, doc)
            }
            DocItem::Atom(atom) => self.fmt_atom(atom),
            DocItem::FormatBreak(format_break) => fmt_format_break(self, format_break),
        })
    }

    fn fmt_atom(
        &mut self,
        atom: &Atom<impl ?Sized + Fn(&mut Formatter) -> fmt::Result>,
    ) -> fmt::Result {
        self.caret_pos += atom.width;
        self.just_newline = false;
        (atom.fmt_fn)(self.f)
    }

    fn fmt_spaces(&mut self, n: usize) -> fmt::Result {
        self.caret_pos += n;
        // self.just_newline = false;
        write!(self.f, "{}", " ".repeat(n))
    }

    fn fmt_newline(&mut self, indent: usize) -> fmt::Result {
        let indent = indent.min(self.options.max_indent);
        self.caret_pos = indent;
        self.just_newline = true;
        self.prev_indent = indent;
        write!(self.f, "\n{}", " ".repeat(indent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hbox() {
        let mut doc: Doc = Doc::new();
        doc.hbox(|doc| {
            doc.atom_fn(|f| write!(f, "--"))
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"));
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 5,
                    max_indent: 5,
                }),
            ),
            "-- -- --",
        );
    }

    #[test]
    fn test_vbox() {
        let mut doc: Doc = Doc::new();
        doc.vbox(1, |doc| {
            doc.atom_fn(|f| write!(f, "--"))
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"));
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 5,
                    max_indent: 5,
                }),
            ),
            "\
--
 --
 --",
        );
    }

    #[test]
    fn test_hvbox_h() {
        let mut doc: Doc = Doc::new();
        doc.hvbox(1, |doc| {
            doc.atom("--").space().atom("--").space().atom("--");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 10,
                }),
            ),
            "-- -- --",
        );
    }

    #[test]
    fn test_hvbox_v() {
        let mut doc: Doc = Doc::new();
        doc.hvbox(1, |doc| {
            doc.atom("---").space().atom("---").space().atom("---");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 10,
                }),
            ),
            "\
---
 ---
 ---",
        );
    }

    #[test]
    fn test_hovbox_0() {
        let mut doc: Doc = Doc::new();
        doc.hovbox(2, |doc| {
            doc.atom("---").space().atom("---").space().atom("---");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 10,
                }),
            ),
            "\
--- ---
  ---",
        );
    }

    #[test]
    fn test_hovbox_1() {
        let mut doc: Doc = Doc::new();
        doc.hovbox(2, |doc| {
            doc.atom("---").space().atom("---").space().atom("---");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 6,
                    max_indent: 6,
                }),
            ),
            "\
---
  ---
  ---",
        );
    }

    #[test]
    fn test_box_indent() {
        let mut doc: Doc = Doc::new();
        doc.atom("---[").hovbox(2, |doc| {
            doc.atom("--")
                .space()
                .atom("--")
                .space()
                .atom("--")
                .space()
                .atom("--");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 11,
                    max_indent: 11,
                }),
            ),
            "\
---[-- --
      -- --",
        );
    }

    #[test]
    fn test_break_indent() {
        let mut doc: Doc = Doc::new();
        doc.atom("---").hovbox(1, |doc| {
            doc.atom("[--")
                .format_break(1, 2)
                .atom("--")
                .format_break(1, 2)
                .atom("--")
                .format_break(1, 2)
                .atom("--");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 10,
                }),
            ),
            "\
---[-- --
      --
      --",
        );
    }

    #[test]
    fn test_sbox() {
        let mut doc: Doc = Doc::new();
        doc.sbox(0, |doc| {
            doc.atom("(---")
                .format_break(0, 1)
                .sbox(0, |doc| {
                    doc.atom("(----")
                        .format_break(0, 1)
                        .sbox(0, |doc| {
                            doc.atom("(---").cut().atom(")");
                        })
                        .cut()
                        .atom(")");
                })
                .cut()
                .atom(")");
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 10,
                }),
            ),
            "\
(---
 (----
  (---)
 )
)",
        );
    }

    #[test]
    fn test_max_indent() {
        let mut doc: Doc = Doc::new();
        doc.vbox(2, |doc| {
            doc.atom("v").cut().vbox(2, |doc| {
                doc.atom("v").cut().sbox(1, |doc| {
                    doc.atom("s1").sbox(0, |doc| {
                        doc.atom("s0").format_break(10, 2).atom("bla");
                    });
                });
            });
        });
        assert_eq!(
            format!(
                "{}",
                doc.display(&FormattingOptions {
                    width: 10,
                    max_indent: 5,
                }),
            ),
            "\
v
  v
    s1
     s0
     bla",
        );
    }
}
