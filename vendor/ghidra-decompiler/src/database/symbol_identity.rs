use super::SYMBOL_ID_BASE;
use super::{Database, ScopeId, SymbolId};
use crate::error::{Error, Result};
use crate::types::TypeFactory;

const SCOPE_BITS: u64 = 0xffff;
const COUNTER_BITS: u32 = 40;
const COUNTER_LIMIT: u64 = 1u64 << COUNTER_BITS;

pub(super) fn validate_counter(counter: u64) -> Result<()> {
    if counter > COUNTER_LIMIT {
        return Err(Error::Lowlevel("symbol ID reservation exceeds counter range".into()));
    }
    Ok(())
}

pub(super) fn generated_id(scope_id: u64, counter: u64) -> Result<u64> {
    if counter >= COUNTER_LIMIT {
        return Err(Error::Lowlevel("symbol ID counter exhausted".into()));
    }
    let scope_part = (scope_id & SCOPE_BITS)
        .checked_shl(COUNTER_BITS)
        .ok_or_else(|| Error::Lowlevel("symbol scope ID overflow".into()))?;
    SYMBOL_ID_BASE
        .checked_add(scope_part)
        .and_then(|value| value.checked_add(counter))
        .ok_or_else(|| Error::Lowlevel("symbol ID exhausted".into()))
}

pub(super) fn advance_counter(scope_id: u64, symbol_id: u64, current: u64) -> Result<u64> {
    let Some(counter) = generated_counter(scope_id, symbol_id) else {
        return Ok(current);
    };
    let required = counter
        .checked_add(1)
        .ok_or_else(|| Error::Lowlevel("symbol ID counter exhausted".into()))?;
    Ok(current.max(required))
}

fn generated_counter(scope_id: u64, symbol_id: u64) -> Option<u64> {
    let scope_part = (scope_id & SCOPE_BITS) << COUNTER_BITS;
    let generated = SYMBOL_ID_BASE.checked_add(scope_part)?;
    let counter = symbol_id.checked_sub(generated)?;
    (counter < COUNTER_LIMIT).then_some(counter)
}

impl Database {
    pub fn scope_add_symbol_internal(&mut self, scope: ScopeId, sym: SymbolId, types: &TypeFactory) -> Result<()> {
        let result = self
            .assign_symbol_identity(scope, sym)
            .and_then(|_| self.add_symbol_to_indexes(scope, sym, types));
        if let Err(error) = result {
            if error.is_lowlevel() {
                self.destroy_symbol(sym);
            }
            return Err(error);
        }
        Ok(())
    }

    fn assign_symbol_identity(&mut self, scope: ScopeId, sym: SymbolId) -> Result<()> {
        let symbol_id = self.symbol(sym).symbol_id;
        let unique_id = self.scope(scope).unique_id;
        let current = self.scope(scope).next_unique_id;
        let (assigned, next) = if symbol_id == 0 {
            let assigned = generated_id(unique_id, current)?;
            let next = current
                .checked_add(1)
                .ok_or_else(|| Error::Lowlevel("symbol ID counter exhausted".into()))?;
            (assigned, next)
        } else {
            let next = advance_counter(unique_id, symbol_id, current)?;
            (symbol_id, next)
        };

        self.symbol_mut(sym).symbol_id = assigned;
        self.scope_mut(scope).next_unique_id = next;
        Ok(())
    }

    fn add_symbol_to_indexes(&mut self, scope: ScopeId, sym: SymbolId, types: &TypeFactory) -> Result<()> {
        if self.symbol(sym).name.is_empty() {
            let name = self.scope_build_undefined_name(scope)?;
            let symbol = self.symbol_mut(sym);
            symbol.name = name.clone();
            symbol.display_name = name;
        }
        let Some(tp) = self.symbol(sym).tp else {
            return Err(Error::Lowlevel(format!(
                "{} symbol created with no type",
                self.symbol(sym).name
            )));
        };
        if types.get(tp).get_size() < 1 {
            return Err(Error::Lowlevel(format!(
                "{} symbol created with zero size type",
                self.symbol(sym).name
            )));
        }
        self.scope_insert_name_tree(scope, sym)?;
        let category = self.symbol(sym).category;
        if category >= 0 {
            let cat = category as usize;
            let catlist = &mut self.scopes.get_mut(scope).category;
            while catlist.len() <= cat {
                catlist.push(Vec::new());
            }
            let list = &mut catlist[cat];
            let symbol = self.symbols.get_mut(sym);
            if category > 0 {
                symbol.catindex = list.len() as u16;
            }
            while list.len() <= symbol.catindex as usize {
                list.push(None);
            }
            list[symbol.catindex as usize] = Some(sym);
        }
        Ok(())
    }
}
