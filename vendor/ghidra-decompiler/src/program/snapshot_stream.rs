//! Borrowed schema-1 metadata and architecture encoding for bounded snapshot I/O.
use super::snapshot::{SavedData, SavedFunction};
use super::{Program, panic_guard};
use crate::database::SymbolId;
use crate::error::Result;
use crate::marshal::Encoder;
use serde::ser::{Error as _, SerializeMap};
use serde::{Serialize, Serializer};
use std::ops::Range;

/// A consistent borrowed view; the Program cannot be mutated while it is in use.
pub struct ProgramSnapshotView<'a> {
    program: &'a Program,
    functions: Vec<(u64, SymbolId)>,
}

impl Program {
    /// Release temporary function analysis before creating a snapshot view.
    pub fn snapshot_view(&self) -> Result<ProgramSnapshotView<'_>> {
        panic_guard(|| {
            self.validate_snapshot_ready()?;
            let mut functions = self
                .functions
                .keys()
                .map(|id| self.function_address(*id).map(|address| (address, *id)))
                .collect::<Result<Vec<_>>>()?;
            // The owned snapshot's BTreeMap keeps the last SymbolId at an address.
            functions.sort_unstable_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)));
            functions.dedup_by_key(|(address, _)| *address);
            Ok(ProgramSnapshotView {
                program: self,
                functions,
            })
        })
    }
}

impl ProgramSnapshotView<'_> {
    /// Encode durable architecture state. The caller must also finish its encoder.
    pub fn encode_architecture(&self, encoder: &mut dyn Encoder) -> Result<()> {
        panic_guard(|| self.program.architecture.encode_base(encoder))
    }

    pub fn functions(&self) -> impl Serialize + '_ {
        SavedFunctions(self)
    }

    pub fn data(&self) -> impl Serialize + '_ {
        SavedDataRecords(self.program)
    }

    pub fn string_sections(&self) -> &[Range<u64>] {
        &self.program.string_sections
    }

    pub fn code_sections(&self) -> &[Range<u64>] {
        &self.program.code_sections
    }

    pub fn entry(&self) -> Option<u64> {
        self.program.entry
    }
}

struct SavedFunctions<'a>(&'a ProgramSnapshotView<'a>);

impl Serialize for SavedFunctions<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let program = self.0.program;
        let table = program.symbol_table().map_err(S::Error::custom)?;
        let mut map = serializer.serialize_map(Some(self.0.functions.len()))?;
        for (address, id) in &self.0.functions {
            let record = program
                .functions
                .get(id)
                .ok_or_else(|| S::Error::custom("missing snapshot function"))?;
            let saved = SavedFunction {
                name: table.symbol(*id).get_name(),
                loaded_name: record.loaded_name.as_str(),
                symbol: record.symbol.as_deref(),
                source: record.source,
            };
            map.serialize_entry(address, &saved)?;
        }
        map.end()
    }
}

struct SavedDataRecords<'a>(&'a Program);

impl Serialize for SavedDataRecords<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.data.len()))?;
        for (address, record) in &self.0.data {
            let saved = SavedData {
                name: record.name.as_str(),
                loaded_name: record.loaded_name.as_str(),
                symbol: record.symbol.as_deref(),
                size: record.size,
            };
            map.serialize_entry(address, &saved)?;
        }
        map.end()
    }
}
