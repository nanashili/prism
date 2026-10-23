//! Prism's snapshot adapter for the pinned decompiler revision.
//!
//! Snapshots contain durable architecture state, not executable bytes or compiled
//! translator caches. Callers must bind them to an exact image and engine version.
use super::compact_decode::CompactDecode;
use super::{DataRecord, FunctionRecord, Program, SymbolInput, panic_guard};
use crate::address::Address;
use crate::database::Database;
use crate::error::{Error, Result};
use crate::image_bytes::ImageBytes;
use crate::marshal::XmlEncode;
use crate::object_arch::ObjectArchitecture;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct ProgramSnapshot {
    architecture: String,
    functions: BTreeMap<u64, SavedFunction>,
    data: BTreeMap<u64, SavedData>,
    string_sections: Vec<std::ops::Range<u64>>,
    code_sections: Vec<std::ops::Range<u64>>,
    entry: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct SavedFunction<S = String> {
    /// Retained for compatibility with existing snapshot JSON.
    pub(super) name: S,
    pub(super) loaded_name: S,
    pub(super) symbol: Option<S>,
    pub(super) source: super::FunctionSource,
}

#[derive(Serialize, Deserialize)]
pub(super) struct SavedData<S = String> {
    pub(super) name: S,
    pub(super) loaded_name: S,
    pub(super) symbol: Option<S>,
    pub(super) size: u64,
}

impl Program {
    /// Capture durable symbols, types, context, comments and string knowledge.
    /// Release temporary function analysis before calling this method.
    pub fn snapshot(&self) -> Result<ProgramSnapshot> {
        panic_guard(|| {
            self.validate_snapshot_ready()?;
            let mut encoder = XmlEncode::new(false);
            self.architecture.encode_base(&mut encoder)?;
            let functions = self
                .functions
                .iter()
                .map(|(id, record)| {
                    let symbol = self.symbol_table()?.symbol(*id);
                    Ok((
                        self.function_address(*id)?,
                        SavedFunction {
                            name: symbol.get_name().to_string(),
                            loaded_name: record.loaded_name.clone(),
                            symbol: record.symbol.clone(),
                            source: record.source,
                        },
                    ))
                })
                .collect::<Result<_>>()?;
            let data = self
                .data
                .iter()
                .map(|(address, record)| {
                    (
                        *address,
                        SavedData {
                            name: record.name.clone(),
                            loaded_name: record.loaded_name.clone(),
                            symbol: record.symbol.clone(),
                            size: record.size,
                        },
                    )
                })
                .collect();
            Ok(ProgramSnapshot {
                architecture: encoder.into_string(),
                functions,
                data,
                string_sections: self.string_sections.clone(),
                code_sections: self.code_sections.clone(),
                entry: self.entry,
            })
        })
    }

    pub(super) fn validate_snapshot_ready(&self) -> Result<()> {
        let table = self.symbol_table()?;
        for id in self.architecture.function_symbols_address_order()? {
            if table.symbol(id).function_checked_out {
                return Err(Error::Lowlevel("cannot save a function while it is in use".into()));
            }
        }
        Ok(())
    }

    /// Restore an exact-image snapshot without repeating symbol or code discovery.
    /// Native symbol IDs are resolved again, never reused across architectures.
    pub fn from_snapshot(
        name: &str,
        bytes: Vec<u8>,
        language: Option<&str>,
        snapshot: ProgramSnapshot,
    ) -> Result<Self> {
        Self::from_snapshot_bytes(name, ImageBytes::from_vec(bytes), language, snapshot)
    }

    pub fn from_snapshot_bytes(
        name: &str,
        bytes: ImageBytes,
        language: Option<&str>,
        mut snapshot: ProgramSnapshot,
    ) -> Result<Self> {
        panic_guard(|| {
            let builder = ObjectArchitecture::with_image_bytes(name, language.unwrap_or("default"), bytes);
            let mut program = Self::initialize_unguarded(Arc::new(builder), SymbolInput::None)?;
            program.architecture.builder = None;
            let mut encoded = std::mem::take(&mut snapshot.architecture);
            restore_legacy_string_elements(&mut encoded);
            let manager = program.architecture.manager.clone();
            let translate = program.architecture.translate.clone();
            let mut decoder = CompactDecode::new(Some(&manager), 0);
            decoder.set_translate(translate.as_deref());
            decoder
                .ingest_owned(encoded.into_bytes())
                .map_err(|error| Error::Decoder(format!("cannot decode saved analysis XML: {error}")))?;
            let next_function_id = decoder.next_global_function_id()?;
            Database::reserve_global_function_ids(&mut program.architecture, next_function_id)?;
            program.architecture.restore_xml_base_decoder(&mut decoder)?;
            drop(decoder);
            program.restore_records(snapshot)?;
            Ok(program)
        })
    }

    fn restore_records(&mut self, snapshot: ProgramSnapshot) -> Result<()> {
        let scope = self.global_scope()?;
        for (address, record) in snapshot.functions {
            let location = self.code_address(address)?;
            let id = self
                .symbol_table()?
                .scope_query_function(scope, &location)
                .ok_or_else(|| Error::Lowlevel(format!("missing saved function at {address:#x}")))?;
            self.functions.insert(
                id,
                FunctionRecord {
                    loaded_name: record.loaded_name,
                    symbol: record.symbol,
                    source: record.source,
                },
            );
        }
        for (address, record) in snapshot.data {
            let location = self.data_address(address)?;
            let table = self.symbol_table()?;
            let entry = table
                .scope_find_addr(scope, &location, &Address::invalid())
                .ok_or_else(|| Error::Lowlevel(format!("missing saved data at {address:#x}")))?;
            self.data.insert(
                address,
                DataRecord {
                    sym: table.entry(entry).get_symbol(),
                    name: record.name,
                    loaded_name: record.loaded_name,
                    symbol: record.symbol,
                    size: record.size,
                },
            );
        }
        self.string_sections = snapshot.string_sections;
        self.code_sections = snapshot.code_sections;
        self.entry = snapshot.entry;
        Ok(())
    }
}

fn restore_legacy_string_elements(architecture: &mut String) {
    const START: &str = "<stringmanage>";
    const END: &str = "</stringmanage>";

    let Some(start) = architecture.find(START) else {
        return;
    };
    let content_start = start + START.len();
    let Some(relative_end) = architecture[content_start..].find(END) else {
        return;
    };
    let content_end = content_start + relative_end;
    let content = &architecture[content_start..content_end];

    // The original writer omitted every closing string tag. Repair only that
    // complete legacy shape; leave valid or otherwise damaged state untouched.
    let entries = content.matches("<string>").count();
    if entries == 0
        || content.contains("</string>")
        || entries != content.matches("</bytes>").count()
        || !content.trim_end().ends_with("</bytes>")
    {
        return;
    }

    let repaired = content.replace("</bytes>", "</bytes></string>");
    architecture.replace_range(content_start..content_end, &repaired);
}
