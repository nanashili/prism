use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::{Error, Result};

use super::index::{Index, OpenFrame};
use super::text::{validate_source, validate_text};
use super::{ContentKind, Document, NONE};

const MAX_PARSE_DEPTH: usize = 10_000;

impl Document {
    pub(crate) fn parse_owned(source: Vec<u8>) -> Result<Self> {
        validate_source(&source)?;
        let mut reader = Reader::from_reader(source.as_slice());
        reader.config_mut().check_end_names = true;
        reader.config_mut().check_comments = true;
        let mut index = Index::default();
        let mut open = Vec::<OpenFrame>::new();
        let mut root = NONE;
        let mut declaration_seen = false;

        loop {
            let event = reader
                .read_event()
                .map_err(|error| Error::Decoder(format!("XML parse error: {error}")))?;
            let empty = matches!(&event, Event::Empty(_));
            match event {
                Event::Start(start) | Event::Empty(start) => {
                    if open.is_empty() && root != NONE {
                        return Err(Error::Decoder("XML document has multiple roots".into()));
                    }
                    if open.len() >= MAX_PARSE_DEPTH {
                        return Err(Error::Decoder("XML parse nesting is too deep".into()));
                    }
                    let node = index.add_node(&source, start)?;
                    index.attach_child(open.last_mut(), node);
                    if open.is_empty() {
                        root = node;
                    }
                    if !empty {
                        open.push(OpenFrame::new(node));
                    }
                }
                Event::End(_) => {
                    open.pop()
                        .ok_or_else(|| Error::Decoder("unexpected XML closing element".into()))?;
                }
                Event::Text(text) => {
                    if text.as_ref().windows(3).any(|bytes| bytes == b"]]>") {
                        return Err(Error::Decoder("unescaped ']]>' in XML text".into()));
                    }
                    validate_text(text.as_ref())?;
                    index.add_content(&source, open.last_mut(), text.as_ref(), ContentKind::Text)?;
                }
                Event::CData(text) => {
                    if open.is_empty() {
                        return Err(Error::Decoder("CDATA outside XML root".into()));
                    }
                    index.add_content(&source, open.last_mut(), text.as_ref(), ContentKind::CData)?;
                }
                Event::GeneralRef(reference) => {
                    let escaped = format!("&{};", std::str::from_utf8(reference.as_ref()).map_err(invalid_utf8)?);
                    validate_text(escaped.as_bytes())?;
                    index.add_content(&source, open.last_mut(), reference.as_ref(), ContentKind::Reference)?;
                }
                Event::Comment(_) => {}
                Event::Decl(declaration) if root == NONE && !declaration_seen => {
                    declaration
                        .version()
                        .map_err(|error| Error::Decoder(format!("invalid XML declaration: {error}")))?;
                    declaration_seen = true;
                }
                Event::Decl(_) => return Err(Error::Decoder("misplaced XML declaration".into())),
                Event::PI(_) => return Err(Error::Decoder("processing instructions are not supported".into())),
                Event::DocType(_) => return Err(Error::Decoder("DTD's not supported".into())),
                Event::Eof => break,
            }
        }

        if !open.is_empty() {
            return Err(Error::Decoder("unexpected end of XML document".into()));
        }
        if root == NONE {
            return Err(Error::Decoder("XML document has no root".into()));
        }
        Ok(Self {
            source,
            nodes: index.nodes,
            attributes: index.attributes,
            contents: index.contents,
            root,
        })
    }
}

pub(super) fn invalid_utf8(error: std::str::Utf8Error) -> Error {
    Error::Decoder(format!("invalid UTF-8 in saved XML: {error}"))
}
