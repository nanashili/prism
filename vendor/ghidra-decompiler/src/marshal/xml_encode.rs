use std::fmt;
use std::io::{self, Write};

use super::{ATTRIB_CONTENT, AttributeId, ElementId, Encoder};
use crate::opcodes::{OpCode, get_opname};
use crate::space::AddrSpace;
use crate::xml::escaped_xml;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TagStatus {
    Start,
    Content,
    Stop,
}

const XML_SPACES: &str = "\n                        ";

/// XML encoder with either its traditional owned buffer or a caller-owned sink.
/// The Encoder trait cannot return I/O errors; streaming callers must call finish.
pub struct XmlEncode<W = Vec<u8>> {
    out: W,
    error: Option<io::Error>,
    tag_status: TagStatus,
    depth: i32,
    do_formatting: bool,
}

impl XmlEncode {
    pub fn new(do_format: bool) -> Self {
        Self::with_writer(Vec::new(), do_format)
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.out).expect("XML encoder emits UTF-8")
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.out
    }

    pub fn into_string(self) -> String {
        String::from_utf8(self.out).expect("XML encoder emits UTF-8")
    }

    pub fn take(&mut self) -> String {
        String::from_utf8(std::mem::take(&mut self.out)).expect("XML encoder emits UTF-8")
    }
}

impl<W: Write> XmlEncode<W> {
    pub fn with_writer(writer: W, do_format: bool) -> Self {
        Self {
            out: writer,
            error: None,
            tag_status: TagStatus::Stop,
            depth: 0,
            do_formatting: do_format,
        }
    }

    /// Return the sink or the first write failure. This does not flush the sink.
    pub fn finish(self) -> io::Result<W> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.out),
        }
    }

    fn append(&mut self, text: &str) {
        if self.error.is_none() {
            self.error = self.out.write_all(text.as_bytes()).err();
        }
    }

    fn append_format(&mut self, arguments: fmt::Arguments<'_>) {
        if self.error.is_none() {
            self.error = self.out.write_fmt(arguments).err();
        }
    }

    fn escaped(&mut self, text: &str) {
        for fragment in escaped_xml(text) {
            self.append(fragment);
        }
    }

    fn new_line(&mut self) {
        if self.do_formatting {
            let spaces = (self.depth * 2 + 1).min(XML_SPACES.len() as i32);
            self.append(&XML_SPACES[..spaces as usize]);
        }
    }

    fn start_content(&mut self) {
        if self.tag_status == TagStatus::Start {
            self.append(">");
        }
    }

    fn write_scalar(&mut self, attribute: AttributeId, value: fmt::Arguments<'_>) {
        if attribute == ATTRIB_CONTENT {
            self.start_content();
            self.append_format(value);
            self.tag_status = TagStatus::Content;
        } else {
            self.append_format(format_args!(" {}=\"", attribute.get_name()));
            self.append_format(value);
            self.append("\"");
        }
    }
}

impl<W: Write> Encoder for XmlEncode<W> {
    fn open_element(&mut self, element: ElementId) {
        if self.tag_status == TagStatus::Start {
            self.append(">");
        } else {
            self.tag_status = TagStatus::Start;
        }
        self.new_line();
        self.append("<");
        self.append(element.get_name());
        self.depth += 1;
    }

    fn close_element(&mut self, element: ElementId) {
        self.depth -= 1;
        if self.tag_status == TagStatus::Start {
            self.append("/>");
            self.tag_status = TagStatus::Stop;
            return;
        }
        if self.tag_status != TagStatus::Content {
            self.new_line();
        } else {
            self.tag_status = TagStatus::Stop;
        }
        self.append("</");
        self.append(element.get_name());
        self.append(">");
    }

    fn write_bool(&mut self, attribute: AttributeId, value: bool) {
        self.write_scalar(attribute, format_args!("{}", if value { "true" } else { "false" }));
    }

    fn write_signed_integer(&mut self, attribute: AttributeId, value: i64) {
        self.write_scalar(attribute, format_args!("{value}"));
    }

    fn write_unsigned_integer(&mut self, attribute: AttributeId, value: u64) {
        self.write_scalar(attribute, format_args!("0x{value:x}"));
    }

    fn write_string(&mut self, attribute: AttributeId, value: &str) {
        if attribute == ATTRIB_CONTENT {
            self.start_content();
            self.escaped(value);
            self.tag_status = TagStatus::Content;
        } else {
            self.append_format(format_args!(" {}=\"", attribute.get_name()));
            self.escaped(value);
            self.append("\"");
        }
    }

    fn write_string_indexed(&mut self, attribute: AttributeId, index: u32, value: &str) {
        self.append_format(format_args!(" {}{}=\"", attribute.get_name(), index.wrapping_add(1)));
        self.escaped(value);
        self.append("\"");
    }

    fn write_space(&mut self, attribute: AttributeId, space: &AddrSpace) {
        self.write_string(attribute, space.get_name());
    }

    fn write_opcode(&mut self, attribute: AttributeId, opcode: OpCode) {
        self.write_scalar(attribute, format_args!("{}", get_opname(opcode)));
    }
}
