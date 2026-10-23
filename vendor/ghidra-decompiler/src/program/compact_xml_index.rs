use quick_xml::events::BytesStart;

use crate::error::{Error, Result};

use super::text::{is_whitespace, validate_name, validate_text};
use super::{Attribute, Content, ContentKind, NONE, Node, Span};

// This bounds allocated index capacity, independently of the encoded input.
const MAX_INDEX_BYTES: usize = 256 * 1024 * 1024;

pub(super) struct OpenFrame {
    node: u32,
    last_child: u32,
    last_content: u32,
}

impl OpenFrame {
    pub(super) fn new(node: u32) -> Self {
        Self {
            node,
            last_child: NONE,
            last_content: NONE,
        }
    }
}

#[derive(Default)]
pub(super) struct Index {
    pub(super) nodes: Vec<Node>,
    pub(super) attributes: Vec<Attribute>,
    pub(super) contents: Vec<Content>,
    allocated_bytes: usize,
}

impl Index {
    pub(super) fn add_node(&mut self, source: &[u8], start: BytesStart<'_>) -> Result<u32> {
        validate_name(start.name().as_ref())?;
        let name = span(source, start.name().as_ref())?;
        let first_attribute = self.attributes.len() as u32;
        for attribute in start.attributes().with_checks(true) {
            let attribute = attribute.map_err(|error| Error::Decoder(format!("XML attribute error: {error}")))?;
            validate_name(attribute.key.as_ref())?;
            if attribute.value.contains(&b'<') {
                return Err(Error::Decoder("unescaped '<' in XML attribute".into()));
            }
            validate_text(attribute.value.as_ref())?;
            push_bounded(
                &mut self.attributes,
                Attribute {
                    name: span(source, attribute.key.as_ref())?,
                    value: span(source, attribute.value.as_ref())?,
                },
                &mut self.allocated_bytes,
            )?;
        }
        let attributes = first_attribute..self.attributes.len() as u32;
        push_bounded(
            &mut self.nodes,
            Node {
                name,
                attributes,
                first_content: NONE,
                first_child: NONE,
                next_sibling: NONE,
            },
            &mut self.allocated_bytes,
        )
    }

    pub(super) fn attach_child(&mut self, parent: Option<&mut OpenFrame>, child: u32) {
        let Some(parent) = parent else { return };
        if parent.last_child == NONE {
            self.nodes[parent.node as usize].first_child = child;
        } else {
            self.nodes[parent.last_child as usize].next_sibling = child;
        }
        parent.last_child = child;
    }

    pub(super) fn add_content(
        &mut self,
        source: &[u8],
        frame: Option<&mut OpenFrame>,
        bytes: &[u8],
        kind: ContentKind,
    ) -> Result<()> {
        let Some(frame) = frame else {
            if is_whitespace(bytes) && !matches!(kind, ContentKind::Reference) {
                return Ok(());
            }
            return Err(Error::Decoder("text outside XML root".into()));
        };
        // Match the original DOM: ignore entirely white text/CDATA chunks.
        if !matches!(kind, ContentKind::Reference) && is_whitespace(bytes) {
            return Ok(());
        }
        let content = Content {
            span: span(source, bytes)?,
            kind,
            next: NONE,
        };
        let id = push_bounded(&mut self.contents, content, &mut self.allocated_bytes)?;
        if frame.last_content == NONE {
            self.nodes[frame.node as usize].first_content = id;
        } else {
            self.contents[frame.last_content as usize].next = id;
        }
        frame.last_content = id;
        Ok(())
    }
}

fn push_bounded<T>(items: &mut Vec<T>, value: T, allocated: &mut usize) -> Result<u32> {
    if items.len() == items.capacity() {
        let old_bytes = items.capacity() * size_of::<T>();
        let capacity = items
            .capacity()
            .max(4)
            .checked_mul(2)
            .ok_or_else(|| Error::Decoder("XML index capacity overflow".into()))?;
        let next_bytes = capacity
            .checked_mul(size_of::<T>())
            .and_then(|bytes| allocated.checked_sub(old_bytes)?.checked_add(bytes))
            .ok_or_else(|| Error::Decoder("XML index capacity overflow".into()))?;
        if next_bytes > MAX_INDEX_BYTES {
            return Err(Error::Decoder(
                "saved XML index exceeds the 256 MiB metadata limit".into(),
            ));
        }
        items
            .try_reserve_exact(capacity - items.len())
            .map_err(|error| Error::Decoder(format!("cannot allocate saved XML index: {error}")))?;
        *allocated = *allocated - old_bytes + items.capacity() * size_of::<T>();
    }
    let index = u32::try_from(items.len()).map_err(|_| Error::Decoder("too many XML index entries".into()))?;
    items.push(value);
    Ok(index)
}

fn span(source: &[u8], bytes: &[u8]) -> Result<Span> {
    let start = (bytes.as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .ok_or_else(|| Error::Decoder("XML event was not borrowed from source".into()))?;
    let end = start
        .checked_add(bytes.len())
        .filter(|end| *end <= source.len())
        .ok_or_else(|| Error::Decoder("XML span is outside source".into()))?;
    Ok(Span {
        start: start as u32,
        end: end as u32,
    })
}
