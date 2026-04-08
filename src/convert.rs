use super::*;

impl<'a> From<DocSend<'a>> for Doc<'a> {
    fn from(value: DocSend<'a>) -> Self {
        Doc {
            items: value.items.into_iter().map(Into::into).collect(),
            head_segment_flat_width: value.head_segment_flat_width,
            last_format_break_index: value.last_format_break_index,
            _marker: value._marker,
        }
    }
}

impl<'a> From<DocSync<'a>> for Doc<'a> {
    fn from(value: DocSync<'a>) -> Self {
        Doc {
            items: value.items.into_iter().map(Into::into).collect(),
            head_segment_flat_width: value.head_segment_flat_width,
            last_format_break_index: value.last_format_break_index,
            _marker: value._marker,
        }
    }
}

impl<'a> From<DocSendSync<'a>> for DocSend<'a> {
    fn from(value: DocSendSync<'a>) -> Self {
        Doc {
            items: value.items.into_iter().map(Into::into).collect(),
            head_segment_flat_width: value.head_segment_flat_width,
            last_format_break_index: value.last_format_break_index,
            _marker: value._marker,
        }
    }
}

impl<'a> From<DocSendSync<'a>> for DocSync<'a> {
    fn from(value: DocSendSync<'a>) -> Self {
        Doc {
            items: value.items.into_iter().map(Into::into).collect(),
            head_segment_flat_width: value.head_segment_flat_width,
            last_format_break_index: value.last_format_break_index,
            _marker: value._marker,
        }
    }
}

impl<'a> From<DocItem<'a, FmtFnSend<'a>>> for DocItem<'a, FmtFn<'a>> {
    fn from(value: DocItem<'a, FmtFnSend>) -> Self {
        match value {
            DocItem::Atom(atom) => DocItem::Atom(atom.into()),
            DocItem::FormatBox(format_box) => DocItem::FormatBox(format_box.into()),
            DocItem::FormatBreak(format_break) => DocItem::FormatBreak(format_break),
        }
    }
}

impl<'a> From<DocItem<'a, FmtFnSync<'a>>> for DocItem<'a, FmtFn<'a>> {
    fn from(value: DocItem<'a, FmtFnSync>) -> Self {
        match value {
            DocItem::Atom(atom) => DocItem::Atom(atom.into()),
            DocItem::FormatBox(format_box) => DocItem::FormatBox(format_box.into()),
            DocItem::FormatBreak(format_break) => DocItem::FormatBreak(format_break),
        }
    }
}

impl<'a> From<DocItem<'a, FmtFnSendSync<'a>>> for DocItem<'a, FmtFnSend<'a>> {
    fn from(value: DocItem<'a, FmtFnSendSync>) -> Self {
        match value {
            DocItem::Atom(atom) => DocItem::Atom(atom.into()),
            DocItem::FormatBox(format_box) => DocItem::FormatBox(format_box.into()),
            DocItem::FormatBreak(format_break) => DocItem::FormatBreak(format_break),
        }
    }
}

impl<'a> From<DocItem<'a, FmtFnSendSync<'a>>> for DocItem<'a, FmtFnSync<'a>> {
    fn from(value: DocItem<'a, FmtFnSendSync>) -> Self {
        match value {
            DocItem::Atom(atom) => DocItem::Atom(atom.into()),
            DocItem::FormatBox(format_box) => DocItem::FormatBox(format_box.into()),
            DocItem::FormatBreak(format_break) => DocItem::FormatBreak(format_break),
        }
    }
}

impl<'a> From<FormatBoxSend<'a>> for FormatBox<'a> {
    fn from(value: FormatBoxSend<'a>) -> Self {
        FormatBox {
            kind: value.kind,
            indent: value.indent,
            doc: value.doc.into(),
            flat_width: value.flat_width,
        }
    }
}

impl<'a> From<FormatBoxSync<'a>> for FormatBox<'a> {
    fn from(value: FormatBoxSync<'a>) -> Self {
        FormatBox {
            kind: value.kind,
            indent: value.indent,
            doc: value.doc.into(),
            flat_width: value.flat_width,
        }
    }
}

impl<'a> From<FormatBoxSendSync<'a>> for FormatBoxSend<'a> {
    fn from(value: FormatBoxSendSync<'a>) -> Self {
        FormatBox {
            kind: value.kind,
            indent: value.indent,
            doc: value.doc.into(),
            flat_width: value.flat_width,
        }
    }
}

impl<'a> From<FormatBoxSendSync<'a>> for FormatBoxSync<'a> {
    fn from(value: FormatBoxSendSync<'a>) -> Self {
        FormatBox {
            kind: value.kind,
            indent: value.indent,
            doc: value.doc.into(),
            flat_width: value.flat_width,
        }
    }
}

impl<'a> From<Atom<FmtFnSend<'a>>> for Atom<FmtFn<'a>> {
    fn from(value: Atom<FmtFnSend<'a>>) -> Self {
        Atom {
            fmt_fn: value.fmt_fn,
            width: value.width,
        }
    }
}

impl<'a> From<Atom<FmtFnSync<'a>>> for Atom<FmtFn<'a>> {
    fn from(value: Atom<FmtFnSync<'a>>) -> Self {
        Atom {
            fmt_fn: value.fmt_fn,
            width: value.width,
        }
    }
}

impl<'a> From<Atom<FmtFnSendSync<'a>>> for Atom<FmtFnSend<'a>> {
    fn from(value: Atom<FmtFnSendSync<'a>>) -> Self {
        Atom {
            fmt_fn: value.fmt_fn,
            width: value.width,
        }
    }
}

impl<'a> From<Atom<FmtFnSendSync<'a>>> for Atom<FmtFnSync<'a>> {
    fn from(value: Atom<FmtFnSendSync<'a>>) -> Self {
        Atom {
            fmt_fn: value.fmt_fn,
            width: value.width,
        }
    }
}
