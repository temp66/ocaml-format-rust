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
//! # Examples
//!
//! ```
//! use std::fmt::{self, Display, Formatter, Write as _};
//!
//! use ocaml_format::{Doc, FormattingOptions, doc, sbox};
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
//!     fn to_doc(&self) -> Result<Doc<'_>, fmt::Error> {
//!         Ok(match self {
//!             Lambda::Var(ident) => doc().atom(Ident(ident))?,
//!             Lambda::Abs(param, body) => doc().format_box(
//!                 sbox(1)
//!                     .atom_fn(|f| write!(f, "({}{}{}", Keyword("λ"), Ident(param), Keyword(".")))?
//!                     .space()
//!                     .extend(body.to_doc()?)
//!                     .atom(")")?,
//!             ),
//!             Lambda::App(left, right) => doc().format_box(
//!                 sbox(1)
//!                     .atom("(")?
//!                     .extend(left.to_doc()?)
//!                     .space()
//!                     .extend(right.to_doc()?)
//!                     .atom(")")?,
//!             ),
//!         })
//!     }
//! }
//!
//! fn main() -> fmt::Result {
//!     let x: Box<str> = "x".into();
//!     let expr = Lambda::Abs(
//!         x.clone(),
//!         Box::new(Lambda::App(
//!             Box::new(Lambda::Abs(x.clone(), Box::new(Lambda::Var(x.clone())))),
//!             Box::new(Lambda::Var(x.clone())),
//!         )),
//!     );
//!
//!     let mut buf = String::new();
//!     let doc = expr.to_doc()?;
//!     write!(
//!         buf,
//!         "{}",
//!         doc.display(&FormattingOptions {
//!             width: 10,
//!             max_indent: 10,
//!         }),
//!     )?;
//!     assert_eq!(
//!         buf,
//!         "\
//! (λx.
//!  ((λx. x)
//!   x))",
//!     );
//!     Ok(())
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
    /// Reflects the `margin` in the OCaml implementation.
    ///
    /// Desired line width limit.
    pub width: usize,
    /// Reflects the `max_indent` in the OCaml implementation.
    pub max_indent: usize,
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
    Atom(Atom<F>),
    FormatBox(FormatBox<'a, F>),
    FormatBreak(FormatBreak),
}

/// A box.
pub struct FormatBox<'a, F: ?Sized + 'a = FmtFn<'a>> {
    kind: FormatBoxKind,
    indent: usize,
    doc: Doc<'a, F>,
    flat_width: usize,
}

/// [`FormatBox`] that implements [`Send`].
pub type FormatBoxSend<'a> = FormatBox<'a, FmtFnSend<'a>>;

/// [`FormatBox`] that implements [`Sync`].
pub type FormatBoxSync<'a> = FormatBox<'a, FmtFnSync<'a>>;

/// [`FormatBox`] that implements [`Send`] and [`Sync`].
pub type FormatBoxSendSync<'a> = FormatBox<'a, FmtFnSendSync<'a>>;

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
    fn new() -> Self {
        Self {
            items: Vec::new(),
            head_segment_flat_width: 0,
            last_format_break_index: None,
            _marker: PhantomData,
        }
    }

    fn add_width(&mut self, delta: usize) {
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

    fn atom_inner(mut self, atom: Atom<F>) -> Self {
        self.add_width(atom.width);
        self.items.push(DocItem::Atom(atom));
        self
    }

    /// Appends a box to the document.
    pub fn format_box(mut self, format_box: FormatBox<'a, F>) -> Self {
        self.add_width(format_box.flat_width);
        self.items.push(DocItem::FormatBox(format_box));
        self
    }

    /// Appends a break hint to the document.
    pub fn format_break(mut self, spaces: usize, indent: usize) -> Self {
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
    pub fn space(self) -> Self {
        self.format_break(1, 0)
    }

    /// Appends a newline hint to the document.
    ///
    /// Convenience method for `format_break(0, 0)`.
    pub fn cut(self) -> Self {
        self.format_break(0, 0)
    }

    /// Extends the document with the items of another `Doc`.
    pub fn extend(mut self, doc: impl Into<Doc<'a, F>>) -> Self {
        let doc = doc.into();
        self.add_width(doc.head_segment_flat_width);
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

/// Builder pattern methods.
impl<'a> Doc<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        }))
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(self, d: impl Display + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        }))
    }
}

/// Constructs an empty [`Doc`].
pub fn doc<'a>() -> Doc<'a> {
    Doc::new()
}

/// Builder pattern methods.
impl<'a> DocSend<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        }))
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(self, d: impl Display + Send + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        }))
    }
}

/// Constructs an empty [`DocSend`].
pub fn doc_send<'a>() -> DocSend<'a> {
    Doc::new()
}

/// Builder pattern methods.
impl<'a> DocSync<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Sync + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        }))
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(self, d: impl Display + Sync + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        }))
    }
}

/// Constructs an empty [`DocSync`].
pub fn doc_sync<'a>() -> DocSync<'a> {
    Doc::new()
}

/// Builder pattern methods.
impl<'a> DocSendSync<'a> {
    /// Appends indivisible content to the document, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + Sync + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        }))
    }

    /// Appends indivisible content to the document, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(self, d: impl Display + Send + Sync + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        Ok(self.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        }))
    }
}

/// Constructs an empty [`DocSendSync`].
pub fn doc_send_sync<'a>() -> DocSendSync<'a> {
    Doc::new()
}

/// Builder pattern methods.
impl<'a, F: ?Sized + 'a> FormatBox<'a, F> {
    fn new(kind: FormatBoxKind, indent: usize) -> Self {
        Self {
            kind,
            indent,
            doc: Doc {
                items: Vec::new(),
                head_segment_flat_width: 0,
                last_format_break_index: None,
                _marker: PhantomData,
            },
            flat_width: 0,
        }
    }

    /// Appends a nested box to the box.
    pub fn format_box(mut self, format_box: FormatBox<'a, F>) -> Self {
        self.flat_width += format_box.flat_width;
        self.doc = self.doc.format_box(format_box);
        self
    }

    /// Appends a break hint to the box.
    pub fn format_break(mut self, spaces: usize, indent: usize) -> Self {
        self.flat_width += spaces;
        self.doc = self.doc.format_break(spaces, indent);
        self
    }

    /// Appends a breaking space to the box.
    ///
    /// Convenience method for `format_break(1, 0)`.
    pub fn space(self) -> Self {
        self.format_break(1, 0)
    }

    /// Appends a newline hint to the box.
    ///
    /// Convenience method for `format_break(0, 0)`.
    pub fn cut(self) -> Self {
        self.format_break(0, 0)
    }

    /// Extends the box with the items of a [`Doc`].
    pub fn extend(mut self, doc: impl Into<Doc<'a, F>>) -> Self {
        let doc = doc.into();
        self.flat_width += doc
            .items
            .iter()
            .fold(0, |flat_width, doc_item| match doc_item {
                DocItem::Atom(atom) => flat_width + atom.width,
                DocItem::FormatBox(format_box) => flat_width + format_box.flat_width,
                DocItem::FormatBreak(format_break) => flat_width + format_break.spaces,
            });
        self.doc = self.doc.extend(doc);
        self
    }
}

/// Builder pattern methods.
impl<'a> FormatBox<'a> {
    /// Appends indivisible content to the box, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        });
        Ok(self)
    }

    /// Appends indivisible content to the box, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(mut self, d: impl Display + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        });
        Ok(self)
    }
}

/// Constructs an empty horizontal box (h box, `hbox`).
pub fn hbox<'a>() -> FormatBox<'a> {
    FormatBox::new(FormatBoxKind::H, 0)
}

/// Constructs an empty vertical box (v box, `vbox`).
pub fn vbox<'a>(indent: usize) -> FormatBox<'a> {
    FormatBox::new(FormatBoxKind::V, indent)
}

/// Constructs an empty horizontal/vertical box (hv box, `hvbox`).
pub fn hvbox<'a>(indent: usize) -> FormatBox<'a> {
    FormatBox::new(FormatBoxKind::Hv, indent)
}

/// Constructs an empty horizontal-or-vertical packing box (hov packing box, `hovbox`).
pub fn hovbox<'a>(indent: usize) -> FormatBox<'a> {
    FormatBox::new(FormatBoxKind::HovP, indent)
}

/// Constructs an empty horizontal-or-vertical structural box (hov structural box, `box`).
pub fn sbox<'a>(indent: usize) -> FormatBox<'a> {
    FormatBox::new(FormatBoxKind::HovS, indent)
}

/// Builder pattern methods.
impl<'a> FormatBoxSend<'a> {
    /// Appends indivisible content to the box, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        });
        Ok(self)
    }

    /// Appends indivisible content to the box, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(mut self, d: impl Display + Send + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        });
        Ok(self)
    }
}

/// Constructs an empty horizontal box (h box, `hbox`).
pub fn hbox_send<'a>() -> FormatBoxSend<'a> {
    FormatBox::new(FormatBoxKind::H, 0)
}

/// Constructs an empty vertical box (v box, `vbox`).
pub fn vbox_send<'a>(indent: usize) -> FormatBoxSend<'a> {
    FormatBox::new(FormatBoxKind::V, indent)
}

/// Constructs an empty horizontal/vertical box (hv box, `hvbox`).
pub fn hvbox_send<'a>(indent: usize) -> FormatBoxSend<'a> {
    FormatBox::new(FormatBoxKind::Hv, indent)
}

/// Constructs an empty horizontal-or-vertical packing box (hov packing box, `hovbox`).
pub fn hovbox_send<'a>(indent: usize) -> FormatBoxSend<'a> {
    FormatBox::new(FormatBoxKind::HovP, indent)
}

/// Constructs an empty horizontal-or-vertical structural box (hov structural box, `box`).
pub fn sbox_send<'a>(indent: usize) -> FormatBoxSend<'a> {
    FormatBox::new(FormatBoxKind::HovS, indent)
}

/// Builder pattern methods.
impl<'a> FormatBoxSync<'a> {
    /// Appends indivisible content to the box, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Sync + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        });
        Ok(self)
    }

    /// Appends indivisible content to the box, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(mut self, d: impl Display + Sync + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        });
        Ok(self)
    }
}

/// Constructs an empty horizontal box (h box, `hbox`).
pub fn hbox_sync<'a>() -> FormatBoxSync<'a> {
    FormatBox::new(FormatBoxKind::H, 0)
}

/// Constructs an empty vertical box (v box, `vbox`).
pub fn vbox_sync<'a>(indent: usize) -> FormatBoxSync<'a> {
    FormatBox::new(FormatBoxKind::V, indent)
}

/// Constructs an empty horizontal/vertical box (hv box, `hvbox`).
pub fn hvbox_sync<'a>(indent: usize) -> FormatBoxSync<'a> {
    FormatBox::new(FormatBoxKind::Hv, indent)
}

/// Constructs an empty horizontal-or-vertical packing box (hov packing box, `hovbox`).
pub fn hovbox_sync<'a>(indent: usize) -> FormatBoxSync<'a> {
    FormatBox::new(FormatBoxKind::HovP, indent)
}

/// Constructs an empty horizontal-or-vertical structural box (hov structural box, `box`).
pub fn sbox_sync<'a>(indent: usize) -> FormatBoxSync<'a> {
    FormatBox::new(FormatBoxKind::HovS, indent)
}

/// Builder pattern methods.
impl<'a> FormatBoxSendSync<'a> {
    /// Appends indivisible content to the box, through a formatting closure.
    ///
    /// The content should not contain newlines.
    ///
    /// The formatting closure is called multiple times, to get the width of the content.
    pub fn atom_fn(
        mut self,
        fmt_fn: impl Fn(&mut Formatter) -> fmt::Result + Send + Sync + 'a,
    ) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&FmtFnWrapper::new(&fmt_fn))?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(fmt_fn),
            width,
        });
        Ok(self)
    }

    /// Appends indivisible content to the box, from a value implementing [`Display`].
    ///
    /// The content should not contain newlines.
    ///
    /// The value is formatted multiple times, to get the width of the content.
    pub fn atom(mut self, d: impl Display + Send + Sync + 'a) -> Result<Self, fmt::Error> {
        let width = fmt_width::width_of(&d)?;
        self.flat_width += width;
        self.doc = self.doc.atom_inner(Atom {
            fmt_fn: Box::new(move |f| write!(f, "{}", d)),
            width,
        });
        Ok(self)
    }
}

/// Constructs an empty horizontal box (h box, `hbox`).
pub fn hbox_send_sync<'a>() -> FormatBoxSendSync<'a> {
    FormatBox::new(FormatBoxKind::H, 0)
}

/// Constructs an empty vertical box (v box, `vbox`).
pub fn vbox_send_sync<'a>(indent: usize) -> FormatBoxSendSync<'a> {
    FormatBox::new(FormatBoxKind::V, indent)
}

/// Constructs an empty horizontal/vertical box (hv box, `hvbox`).
pub fn hvbox_send_sync<'a>(indent: usize) -> FormatBoxSendSync<'a> {
    FormatBox::new(FormatBoxKind::Hv, indent)
}

/// Constructs an empty horizontal-or-vertical packing box (hov packing box, `hovbox`).
pub fn hovbox_send_sync<'a>(indent: usize) -> FormatBoxSendSync<'a> {
    FormatBox::new(FormatBoxKind::HovP, indent)
}

/// Constructs an empty horizontal-or-vertical structural box (hov structural box, `box`).
pub fn sbox_send_sync<'a>(indent: usize) -> FormatBoxSendSync<'a> {
    FormatBox::new(FormatBoxKind::HovS, indent)
}

/// A helper type created by [`Doc::display`] that implements [`Display`].
#[derive(Clone, Copy)]
pub struct DocDisplay<'a, F: ?Sized + 'a> {
    doc: &'a Doc<'a, F>,
    options: &'a FormattingOptions,
}

impl<'a, F: ?Sized + Fn(&mut Formatter) -> fmt::Result + 'a> Display for DocDisplay<'a, F> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Engine {
            options: self.options,
            f,
            caret_pos: 0,
            just_newline: true,
            prev_indent: 0,
        }
        .fmt(self.doc)
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
    // `hovbox(0)`, as per `pp_rinit` and `pp_open_sys_box`.
    fn fmt(&mut self, doc: &Doc<impl ?Sized + Fn(&mut Formatter) -> fmt::Result>) -> fmt::Result {
        doc.items.iter().try_for_each(|doc_item| match doc_item {
            DocItem::Atom(atom) => self.fmt_atom(atom),
            DocItem::FormatBox(format_box) => {
                if self.caret_pos > self.options.max_indent {
                    self.fmt_newline(0)?;
                }
                self.fmt_format_box(format_box)
            }
            &DocItem::FormatBreak(FormatBreak {
                spaces,
                indent,
                segment_flat_width,
            }) => {
                if self.caret_pos + segment_flat_width <= self.options.width {
                    self.fmt_spaces(spaces)
                } else {
                    self.fmt_newline(indent)
                }
            }
        })
    }

    fn fmt_format_box(
        &mut self,
        format_box: &FormatBox<impl ?Sized + Fn(&mut Formatter) -> fmt::Result>,
    ) -> fmt::Result {
        let curr_indent = self.caret_pos + format_box.indent;
        let fmt_newline = |engine: &mut Self, format_break_indent| {
            engine.fmt_newline(curr_indent + format_break_indent)
        };

        let fmt_format_break_spaces =
            |engine: &mut Self, format_break: &FormatBreak| engine.fmt_spaces(format_break.spaces);
        let fmt_format_break_newline = |engine: &mut Self, format_break: &FormatBreak| {
            fmt_newline(engine, format_break.indent)
        };
        let fmt_format_break: Box<dyn Fn(&mut Self, &FormatBreak) -> _> = match format_box.kind {
            FormatBoxKind::H => Box::new(fmt_format_break_spaces),
            FormatBoxKind::V => Box::new(fmt_format_break_newline),
            FormatBoxKind::Hv => {
                if self.caret_pos + format_box.flat_width <= self.options.width {
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
                if self.caret_pos + format_box.flat_width <= self.options.width {
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
        format_box
            .doc
            .items
            .iter()
            .try_for_each(|doc_item| match doc_item {
                DocItem::Atom(atom) => self.fmt_atom(atom),
                DocItem::FormatBox(format_box) => {
                    if self.caret_pos > self.options.max_indent && self.caret_pos > curr_indent {
                        fmt_newline(self, 0)?;
                    }
                    self.fmt_format_box(format_box)
                }
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

    use fmt::Write as _;

    #[test]
    fn test_hbox() -> fmt::Result {
        let doc = doc().format_box(
            hbox()
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 5,
                max_indent: 5,
            }),
        )?;
        assert_eq!(buf, "-- -- --");
        Ok(())
    }

    #[test]
    fn test_vbox() -> fmt::Result {
        let doc = doc().format_box(
            vbox(1)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 5,
                max_indent: 5,
            }),
        )?;
        assert_eq!(
            buf,
            "\
--
 --
 --",
        );
        Ok(())
    }

    #[test]
    fn test_hvbox_h() -> fmt::Result {
        let doc = doc().format_box(
            hvbox(1)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(buf, "-- -- --");
        Ok(())
    }

    #[test]
    fn test_hvbox_v() -> fmt::Result {
        let doc = doc().format_box(
            hvbox(1)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(
            buf,
            "\
---
 ---
 ---",
        );
        Ok(())
    }

    #[test]
    fn test_hovbox_0() -> fmt::Result {
        let doc = doc().format_box(
            hovbox(2)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(
            buf,
            "\
--- ---
  ---",
        );
        Ok(())
    }

    #[test]
    fn test_hovbox_1() -> fmt::Result {
        let doc = doc().format_box(
            hovbox(2)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "---"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 6,
                max_indent: 6,
            }),
        )?;
        assert_eq!(
            buf,
            "\
---
  ---
  ---",
        );
        Ok(())
    }

    #[test]
    fn test_open_indent() -> fmt::Result {
        let doc = doc().atom_fn(|f| write!(f, "---["))?.format_box(
            hovbox(2)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 0)
                .atom_fn(|f| write!(f, "--"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 11,
                max_indent: 11,
            }),
        )?;
        assert_eq!(
            buf,
            "\
---[-- --
      -- --",
        );
        Ok(())
    }

    #[test]
    fn test_break_hint() -> fmt::Result {
        let doc = doc().atom_fn(|f| write!(f, "---"))?.format_box(
            hovbox(1)
                .atom_fn(|f| write!(f, "[--"))?
                .format_break(1, 2)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 2)
                .atom_fn(|f| write!(f, "--"))?
                .format_break(1, 2)
                .atom_fn(|f| write!(f, "--"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(
            buf,
            "\
---[-- --
      --
      --",
        );
        Ok(())
    }

    #[test]
    fn test_cbox() -> fmt::Result {
        let doc = doc().format_box(
            sbox(0)
                .atom_fn(|f| write!(f, "(---"))?
                .format_break(0, 1)
                .format_box(
                    sbox(0)
                        .atom_fn(|f| write!(f, "(----"))?
                        .format_break(0, 1)
                        .format_box(
                            sbox(0)
                                .atom_fn(|f| write!(f, "(---"))?
                                .format_break(0, 0)
                                .atom_fn(|f| write!(f, ")"))?,
                        )
                        .format_break(0, 0)
                        .atom_fn(|f| write!(f, ")"))?,
                )
                .format_break(0, 0)
                .atom_fn(|f| write!(f, ")"))?,
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(
            buf,
            "\
(---
 (----
  (---)
 )
)",
        );
        Ok(())
    }

    #[test]
    fn test_atom() -> fmt::Result {
        let doc = doc().atom(42)?;
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 10,
            }),
        )?;
        assert_eq!(buf, "42");
        Ok(())
    }

    #[test]
    fn test_max_indent() -> fmt::Result {
        let doc = doc().format_box(
            vbox(2).atom("v")?.cut().format_box(
                vbox(2).atom("v")?.cut().format_box(
                    sbox(1)
                        .atom("c1")?
                        .format_box(sbox(0).atom("c0")?.format_break(10, 2).atom("bla")?),
                ),
            ),
        );
        let mut buf = String::new();
        write!(
            buf,
            "{}",
            doc.display(&FormattingOptions {
                width: 10,
                max_indent: 5,
            }),
        )?;
        assert_eq!(
            buf,
            "\
v
  v
    c1
     c0
     bla",
        );
        Ok(())
    }
}
