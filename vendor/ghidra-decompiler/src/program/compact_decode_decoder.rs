use crate::error::{Error, Result};
use crate::istream::{self, Basefield};
use crate::marshal::{ATTRIB_UNKNOWN, AttributeId, Decoder, ElementId};
use crate::opcodes::{OpCode, get_opcode};
use crate::space::SpaceRef;
use crate::translate::{AddrSpaceManager, Translate};

use super::CompactDecode;
use crate::program::compact_xml::Document;

impl Decoder for CompactDecode<'_> {
    fn get_addr_space_manager(&self) -> Option<&AddrSpaceManager> {
        self.manager
    }
    fn get_translate(&self) -> Option<&dyn Translate> {
        self.translate
    }

    fn ingest_stream(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > 512 * 1024 * 1024 {
            return Err(Error::Decoder("saved XML exceeds the 512 MiB input limit".into()));
        }
        self.ingest_owned(data.to_vec())
    }

    fn peek_element(&mut self) -> Result<u32> {
        let index = if self.stack.is_empty() {
            self.pending_root
        } else {
            self.iterators.last().copied().filter(|index| *index != super::NONE)
        };
        let Some(index) = index else { return Ok(0) };
        let name = self.document().text(self.node(index).name)?;
        Ok(ElementId::find(name, self.scope))
    }

    fn open_element(&mut self) -> Result<u32> {
        self.open(None)
    }
    fn open_element_expect(&mut self, id: ElementId) -> Result<u32> {
        self.open(Some(id))
    }

    fn close_element(&mut self, _id: u32) -> Result<()> {
        self.stack
            .pop()
            .ok_or_else(|| Error::Decoder("XML element stack underflow".into()))?;
        self.iterators.pop();
        self.attribute_index = 1000;
        Ok(())
    }

    fn close_element_skipping(&mut self, id: u32) -> Result<()> {
        self.close_element(id)
    }

    fn get_next_attribute_id(&mut self) -> Result<u32> {
        let next = self.attribute_index + 1;
        if let Some(name) = self.attribute_at(next as usize).map(|attribute| attribute.name) {
            self.attribute_index = next;
            return Ok(AttributeId::find(self.document().text(name)?, self.scope));
        }
        Ok(0)
    }

    fn get_indexed_attribute_id(&mut self, id: AttributeId) -> Result<u32> {
        let Some(index) = usize::try_from(self.attribute_index).ok() else {
            return Ok(ATTRIB_UNKNOWN.get_id());
        };
        let Some(attribute) = self.attribute_at(index) else {
            return Ok(ATTRIB_UNKNOWN.get_id());
        };
        let name = self.document().text(attribute.name)?;
        let base = id.get_name();
        if !name.starts_with(base) {
            return Ok(ATTRIB_UNKNOWN.get_id());
        }
        let value = istream::read_u32(&name[base.len()..], Basefield::Dec, 0);
        if value == 0 {
            return Err(Error::Lowlevel(format!("Bad indexed attribute: {base}")));
        }
        Ok(id.get_id().wrapping_add(value - 1))
    }

    fn rewind_attributes(&mut self) {
        self.attribute_index = -1;
    }
    fn read_bool(&mut self) -> Result<bool> {
        Ok(crate::xml::xml_readbool(&self.current_value()?))
    }
    fn read_bool_attr(&mut self, id: AttributeId) -> Result<bool> {
        Ok(crate::xml::xml_readbool(&self.text_attribute(id)?))
    }
    fn read_signed_integer(&mut self) -> Result<i64> {
        Ok(istream::read_i64(&self.current_value()?, Basefield::Auto, 0))
    }
    fn read_signed_integer_attr(&mut self, id: AttributeId) -> Result<i64> {
        Ok(istream::read_i64(&self.text_attribute(id)?, Basefield::Auto, 0))
    }
    fn read_signed_integer_expect_string(&mut self, expect: &str, value: i64) -> Result<i64> {
        let text = self.current_value()?;
        Ok(if text == expect {
            value
        } else {
            istream::read_i64(&text, Basefield::Auto, 0)
        })
    }
    fn read_signed_integer_expect_string_attr(&mut self, id: AttributeId, expect: &str, value: i64) -> Result<i64> {
        let text = self.text_attribute(id)?;
        Ok(if text == expect {
            value
        } else {
            istream::read_i64(&text, Basefield::Auto, 0)
        })
    }
    fn read_unsigned_integer(&mut self) -> Result<u64> {
        Ok(istream::read_u64(&self.current_value()?, Basefield::Auto, 0))
    }
    fn read_unsigned_integer_attr(&mut self, id: AttributeId) -> Result<u64> {
        Ok(istream::read_u64(&self.text_attribute(id)?, Basefield::Auto, 0))
    }
    fn read_string(&mut self) -> Result<String> {
        self.current_value().map(|value| value.into_owned())
    }
    fn read_string_attr(&mut self, id: AttributeId) -> Result<String> {
        self.text_attribute(id).map(|value| value.into_owned())
    }

    fn read_space(&mut self) -> Result<SpaceRef> {
        self.lookup_space(&self.current_value()?)
    }
    fn read_space_attr(&mut self, id: AttributeId) -> Result<SpaceRef> {
        self.lookup_space(&self.text_attribute(id)?)
    }
    fn read_opcode(&mut self) -> Result<OpCode> {
        self.opcode(&self.current_value()?)
    }
    fn read_opcode_attr(&mut self, id: AttributeId) -> Result<OpCode> {
        self.opcode(&self.text_attribute(id)?)
    }
}

impl CompactDecode<'_> {
    pub(crate) fn ingest_owned(&mut self, data: Vec<u8>) -> Result<()> {
        let document = Document::parse_owned(data)?;
        self.pending_root = Some(document.root);
        self.document = Some(document);
        self.stack.clear();
        self.iterators.clear();
        self.attribute_index = -1;
        Ok(())
    }
}

impl CompactDecode<'_> {
    fn lookup_space(&self, name: &str) -> Result<SpaceRef> {
        self.manager().and_then(|manager| {
            manager
                .get_space_by_name(name)
                .ok_or_else(|| Error::Decoder(format!("Unknown address space name: {name}")))
        })
    }

    fn opcode(&self, text: &str) -> Result<OpCode> {
        let opcode = get_opcode(text);
        if opcode == OpCode::Blank {
            return Err(Error::Decoder("Bad encoded OpCode".into()));
        }
        Ok(opcode)
    }
}
