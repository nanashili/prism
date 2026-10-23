use crate::error::{Error, Result};
use crate::istream::{Basefield, read_u64};

use super::super::compact_xml::{Document, Node};

const SYMBOL_ID_BASE: u64 = 0x4000_0000_0000_0000;
const COUNTER_LIMIT: u64 = 1 << 40;

pub(super) fn next_global_function_id(document: &Document) -> Result<u64> {
    let mut maximum = 0;
    for node in &document.nodes {
        let name = document.text(node.name)?;
        if matches!(name, "function" | "functionshell") {
            maximum = maximum.max(function_counter(document, node)?);
        }
    }

    maximum
        .checked_add(1)
        .ok_or_else(|| Error::Decoder("saved function symbol IDs exhausted".into()))
}

fn function_counter(document: &Document, node: &Node) -> Result<u64> {
    let attributes = &document.attributes[node.attributes.start as usize..node.attributes.end as usize];
    for attribute in attributes {
        if document.text(attribute.name)? != "id" {
            continue;
        }
        let text = document.text(attribute.value)?;
        let id = read_u64(text, Basefield::Auto, 0);
        let Some(offset) = id.checked_sub(SYMBOL_ID_BASE) else {
            return Ok(0);
        };
        if offset >= COUNTER_LIMIT {
            return Ok(0);
        }
        return Ok(offset);
    }
    Ok(0)
}
