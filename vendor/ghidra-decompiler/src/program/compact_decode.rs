use std::borrow::Cow;

use crate::error::{Error, Result};
use crate::marshal::{ATTRIB_CONTENT, AttributeId, ElementId};
use crate::translate::{AddrSpaceManager, Translate};

use super::compact_xml::{Attribute, ContentKind, Document, NONE, Node, Span};

#[path = "compact_symbol_ids.rs"]
mod symbol_ids;

pub(crate) struct CompactDecode<'a> {
    manager: Option<&'a AddrSpaceManager>,
    translate: Option<&'a dyn Translate>,
    document: Option<Document>,
    pending_root: Option<u32>,
    stack: Vec<u32>,
    iterators: Vec<u32>,
    attribute_index: i32,
    scope: i32,
}

impl<'a> CompactDecode<'a> {
    pub(crate) fn new(manager: Option<&'a AddrSpaceManager>, scope: i32) -> Self {
        Self {
            manager,
            translate: None,
            document: None,
            pending_root: None,
            stack: Vec::new(),
            iterators: Vec::new(),
            attribute_index: -1,
            scope,
        }
    }

    pub(crate) fn set_translate(&mut self, translate: Option<&'a dyn Translate>) {
        self.translate = translate;
    }

    pub(crate) fn next_global_function_id(&self) -> Result<u64> {
        let document = self
            .document
            .as_ref()
            .ok_or_else(|| Error::Decoder("compact XML document missing".into()))?;
        symbol_ids::next_global_function_id(document)
    }

    fn node(&self, index: u32) -> &Node {
        &self.document().nodes[index as usize]
    }

    fn current(&self) -> &Node {
        self.node(*self.stack.last().expect("no open element in decoder"))
    }

    fn document(&self) -> &Document {
        self.document.as_ref().expect("compact XML document missing")
    }

    fn value(&self, span: Span) -> Result<Cow<'_, str>> {
        quick_xml::escape::unescape(self.document().text(span)?)
            .map_err(|error| Error::Decoder(format!("invalid XML entity: {error}")))
    }

    fn content(&self, node: &Node) -> Result<String> {
        let mut output = String::new();
        let mut index = node.first_content;
        while index != NONE {
            let piece = &self.document().contents[index as usize];
            let text = self.document().text(piece.span)?;
            match piece.kind {
                ContentKind::Text => output.push_str(&self.value(piece.span)?),
                ContentKind::CData => output.push_str(text),
                ContentKind::Reference => {
                    let escaped = format!("&{text};");
                    let value = quick_xml::escape::unescape(&escaped)
                        .map_err(|error| Error::Decoder(format!("invalid XML entity: {error}")))?;
                    if !value.bytes().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')) {
                        output.push_str(&value);
                    }
                }
            }
            index = piece.next;
        }
        Ok(output)
    }

    fn attributes(&self) -> &[Attribute] {
        let range = &self.current().attributes;
        &self.document().attributes[range.start as usize..range.end as usize]
    }

    fn text_attribute(&self, id: AttributeId) -> Result<Cow<'_, str>> {
        if id == ATTRIB_CONTENT {
            return self.content(self.current()).map(Cow::Owned);
        }
        for attribute in self.attributes() {
            if self.document().text(attribute.name)? == id.get_name() {
                return self.value(attribute.value);
            }
        }
        Err(Error::Decoder(format!("Attribute missing: {}", id.get_name())))
    }

    fn current_value(&self) -> Result<Cow<'_, str>> {
        let index =
            usize::try_from(self.attribute_index).map_err(|_| Error::Decoder("no current XML attribute".into()))?;
        let attribute = self
            .attribute_at(index)
            .ok_or_else(|| Error::Decoder("no current XML attribute".into()))?;
        self.value(attribute.value)
    }

    fn attribute_at(&self, target: usize) -> Option<&Attribute> {
        self.attributes().get(target)
    }

    fn next_child(&mut self) -> Option<u32> {
        let child = *self.iterators.last()?;
        if child == NONE {
            return None;
        }
        *self.iterators.last_mut()? = self.node(child).next_sibling;
        Some(child)
    }

    fn open(&mut self, expected: Option<ElementId>) -> Result<u32> {
        let index = if self.stack.is_empty() {
            self.pending_root.take()
        } else {
            self.next_child()
        };
        let Some(index) = index else {
            return if expected.is_none() {
                Ok(0)
            } else {
                Err(Error::Decoder("reached end of XML element stream".into()))
            };
        };
        let node_name = self.document().text(self.node(index).name)?;
        if let Some(expected) = expected
            && node_name != expected.get_name()
        {
            return Err(Error::Decoder(format!(
                "Expecting <{}> but got <{node_name}>",
                expected.get_name()
            )));
        }
        let id = expected.map_or_else(|| ElementId::find(node_name, self.scope), |id| id.get_id());
        self.stack.push(index);
        self.iterators.push(self.node(index).first_child);
        self.attribute_index = -1;
        Ok(id)
    }
}

#[path = "compact_decode_decoder.rs"]
mod decoder;
