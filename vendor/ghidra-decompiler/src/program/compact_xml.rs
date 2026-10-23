pub(crate) const NONE: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub(crate) struct Span {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

pub(crate) enum ContentKind {
    Text,
    CData,
    Reference,
}

pub(crate) struct Content {
    pub(crate) span: Span,
    pub(crate) kind: ContentKind,
    pub(crate) next: u32,
}

pub(crate) struct Attribute {
    pub(crate) name: Span,
    pub(crate) value: Span,
}

pub(crate) struct Node {
    pub(crate) name: Span,
    pub(crate) attributes: std::ops::Range<u32>,
    pub(crate) first_content: u32,
    pub(crate) first_child: u32,
    pub(crate) next_sibling: u32,
}

pub(crate) struct Document {
    source: Vec<u8>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) contents: Vec<Content>,
    pub(crate) root: u32,
}

impl Document {
    pub(crate) fn text(&self, span: Span) -> crate::error::Result<&str> {
        let bytes = self
            .source
            .get(span.start as usize..span.end as usize)
            .ok_or_else(|| crate::error::Error::Decoder("saved XML span is outside source".into()))?;
        std::str::from_utf8(bytes).map_err(parse::invalid_utf8)
    }
}

#[path = "compact_xml_index.rs"]
mod index;
#[path = "compact_xml_text.rs"]
mod text;

#[path = "compact_xml_parse.rs"]
mod parse;
