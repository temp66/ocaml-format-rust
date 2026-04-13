use super::*;

impl<'a> From<DocSync<'a>> for Doc<'a> {
    fn from(value: DocSync<'a>) -> Self {
        Doc {
            items: value.items.into_iter().map(Into::into).collect(),
            flat_width: value.flat_width,
            head_segment_flat_width: value.head_segment_flat_width,
            last_format_break_index: value.last_format_break_index,
            _marker: value._marker,
        }
    }
}

impl<'a> From<DocItem<'a, FmtFnSync<'a>>> for DocItem<'a, FmtFn<'a>> {
    fn from(value: DocItem<'a, FmtFnSync>) -> Self {
        match value {
            DocItem::FormatBox(format_box) => DocItem::FormatBox(format_box.into()),
            DocItem::Atom(atom) => DocItem::Atom(atom.into()),
            DocItem::FormatBreak(format_break) => DocItem::FormatBreak(format_break),
            DocItem::Newline => DocItem::Newline,
        }
    }
}

impl<'a> From<FormatBox<'a, FmtFnSync<'a>>> for FormatBox<'a, FmtFn<'a>> {
    fn from(value: FormatBox<'a, FmtFnSync<'a>>) -> Self {
        FormatBox {
            kind: value.kind,
            indent: value.indent,
            doc: value.doc.into(),
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
