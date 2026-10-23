use super::{FunctionEntry, FunctionSource, Program};
use crate::database::SymbolId;
use crate::error::{Error, Result};
use std::sync::Arc;

/// Stable, compact identity for a function in a loaded program.
///
/// The native analysis session keeps these handles instead of copying the
/// display strings for every function. Names remain owned by `Program` and
/// are borrowed only while a page or reference is being rendered. A handle
/// is valid only for the lifetime of the program that produced it.
#[derive(Clone, Debug)]
pub struct FunctionHandle {
    identity: Arc<()>,
    symbol: SymbolId,
    address: u64,
}

impl FunctionHandle {
    pub fn address(&self) -> u64 {
        self.address
    }
}

pub struct FunctionEntryRef<'a> {
    name: &'a str,
    symbol: Option<&'a str>,
    address: u64,
    source: FunctionSource,
}

impl FunctionEntryRef<'_> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn symbol(&self) -> Option<&str> {
        self.symbol
    }

    pub fn address(&self) -> u64 {
        self.address
    }

    pub fn source(&self) -> FunctionSource {
        self.source
    }
}

impl Program {
    pub fn function_inventory(&self) -> Result<Vec<FunctionHandle>> {
        self.architecture
            .function_symbols_address_order()?
            .into_iter()
            .map(|symbol| {
                Ok(FunctionHandle {
                    identity: Arc::clone(&self.identity),
                    symbol,
                    address: self.function_address(symbol)?,
                })
            })
            .collect()
    }

    pub fn function_entry(&self, handle: &FunctionHandle) -> Result<FunctionEntryRef<'_>> {
        let symboltab = self.symbol_table()?;
        if !Arc::ptr_eq(&self.identity, &handle.identity) {
            return Err(Error::Lowlevel(
                "function handle belongs to another program".to_string(),
            ));
        }
        if !symboltab.symbols.contains(handle.symbol) {
            return Err(Error::Lowlevel(format!(
                "function handle references an unknown symbol {}",
                handle.symbol.0
            )));
        }
        if !symboltab.symbol(handle.symbol).is_function() {
            return Err(Error::Lowlevel(format!(
                "function handle references non-function symbol {}",
                handle.symbol.0
            )));
        }
        let actual_address = self.function_address(handle.symbol)?;
        if actual_address != handle.address {
            return Err(Error::Lowlevel(format!(
                "function handle address mismatch: expected 0x{:x}, found 0x{:x}",
                handle.address, actual_address
            )));
        }
        let name = symboltab.symbol(handle.symbol).get_name();
        let (symbol, source) = match self.functions.get(&handle.symbol) {
            Some(record) => (record.symbol.as_deref(), record.source),
            None => (None, FunctionSource::SymbolTable),
        };
        Ok(FunctionEntryRef {
            name,
            symbol,
            address: handle.address,
            source,
        })
    }

    pub fn functions(&self) -> Result<Vec<FunctionEntry>> {
        self.function_inventory()?
            .into_iter()
            .map(|handle| {
                let entry = self.function_entry(&handle)?;
                Ok(FunctionEntry {
                    name: entry.name.to_owned(),
                    symbol: entry.symbol.map(str::to_owned),
                    address: entry.address,
                    source: entry.source,
                })
            })
            .collect()
    }
}
