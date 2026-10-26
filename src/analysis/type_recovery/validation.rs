use super::{Access, Base, Convention, Evidence, Kind, Prototype};
use crate::error::{Result, failure};
use std::collections::BTreeSet;

const MAX_EVIDENCE_BYTES: usize = 384 * 1024;

impl Evidence {
    pub(crate) fn validate(&self) -> Result<()> {
        if !matches!(self.pointer_size, 4 | 8) {
            return Err(failure(
                "Type recovery requires a pointer_size of 4 or 8 bytes",
            ));
        }
        if self.accesses.len() > 1024
            || self.aliases.len() > 512
            || self.prototypes.len() > 256
            || self.names.len() > 256
            || serde_json::to_vec(self)?.len() > MAX_EVIDENCE_BYTES
        {
            return Err(failure("Type recovery evidence exceeds its bounded budget"));
        }

        for access in &self.accesses {
            validate_access(access, self.pointer_size)?;
        }

        for alias in &self.aliases {
            validate_base(&alias.left)?;
            validate_base(&alias.right)?;
            validate_site(&alias.site)?;
        }
        for name in &self.names {
            validate_base(&name.base)?;
            if !identifier(&name.name) {
                return Err(failure(
                    "A recovered type name must be a C identifier of at most 128 bytes",
                ));
            }
        }
        for prototype in &self.prototypes {
            validate_prototype(prototype, self.pointer_size)?;
        }
        Ok(())
    }

    pub(crate) fn functions(&self) -> BTreeSet<&str> {
        self.bases()
            .into_iter()
            .map(|base| base.function.as_str())
            .chain(self.prototypes.iter().map(|item| item.function.as_str()))
            .chain(
                self.accesses
                    .iter()
                    .filter_map(|item| item.target.as_deref()),
            )
            .collect()
    }

    pub(super) fn bases(&self) -> BTreeSet<&Base> {
        self.accesses
            .iter()
            .flat_map(|item| std::iter::once(&item.base).chain(item.pointee.iter()))
            .chain(
                self.aliases
                    .iter()
                    .flat_map(|item| [&item.left, &item.right]),
            )
            .chain(self.names.iter().map(|item| &item.base))
            .chain(self.prototypes.iter().flat_map(|item| {
                item.parameters
                    .iter()
                    .chain(item.returns.iter())
                    .filter_map(|value| value.pointee.as_ref())
            }))
            .collect()
    }

    pub(crate) fn merge(&mut self, incoming: Self) -> Result<()> {
        if self.pointer_size != incoming.pointer_size {
            return Err(failure("The accepted model uses a different pointer size"));
        }
        append_unique(&mut self.accesses, incoming.accesses);
        append_unique(&mut self.aliases, incoming.aliases);
        append_unique(&mut self.prototypes, incoming.prototypes);
        append_unique(&mut self.names, incoming.names);
        self.validate()
    }
}

fn validate_access(access: &Access, pointer_size: u64) -> Result<()> {
    validate_base(&access.base)?;
    validate_site(&access.site)?;
    validate_value(
        access.kind,
        access.width,
        access.pointee.as_ref(),
        pointer_size,
    )?;
    if access.count == 0
        || access.count > 1_000_000
        || access.stride() < access.width
        || access.end().is_none_or(|end| end > (1 << 32))
    {
        return Err(failure(
            "An observed access has an invalid or overflowing extent",
        ));
    }
    if let Some(target) = &access.target {
        validate_address(target)?;
        if access.kind != Kind::FunctionPointer {
            return Err(failure("Only function-pointer accesses can name a target"));
        }
    }
    Ok(())
}

fn validate_prototype(prototype: &Prototype, pointer_size: u64) -> Result<()> {
    validate_address(&prototype.function)?;
    validate_site(&prototype.site)?;
    if prototype.parameters.len() > 64 {
        return Err(failure(
            "A recovered prototype supports at most 64 parameters",
        ));
    }
    for value in prototype.parameters.iter().chain(prototype.returns.iter()) {
        validate_value(
            value.kind,
            value.width,
            value.pointee.as_ref(),
            pointer_size,
        )?;
    }
    Ok(())
}

fn append_unique<T: PartialEq>(current: &mut Vec<T>, incoming: Vec<T>) {
    for item in incoming {
        if !current.contains(&item) {
            current.push(item);
        }
    }
}

fn validate_base(base: &Base) -> Result<()> {
    validate_address(&base.function)?;
    if !identifier(&base.variable) {
        return Err(failure(
            "A pointer variable must be a C identifier of at most 128 bytes",
        ));
    }
    Ok(())
}

fn validate_site(site: &str) -> Result<()> {
    if site.trim().is_empty() || site.len() > 512 {
        return Err(failure(
            "Every observation needs a bounded instruction or call-site reference",
        ));
    }
    Ok(())
}

fn validate_address(address: &str) -> Result<()> {
    if address.len() > 32 || crate::analysis::graph::address_value(address).is_none() {
        return Err(failure(
            "Type evidence must reference a native function address",
        ));
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 128
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_value(kind: Kind, width: u64, pointee: Option<&Base>, pointer_size: u64) -> Result<()> {
    let valid_width = match kind {
        Kind::Bytes => (1..=256).contains(&width),
        Kind::Integer => matches!(width, 1 | 2 | 4 | 8 | 16),
        Kind::Float => matches!(width, 2 | 4 | 8 | 16),
        Kind::Pointer | Kind::FunctionPointer => width == pointer_size,
    };
    if !valid_width {
        return Err(failure(
            "An observed value width is incompatible with its kind or pointer size",
        ));
    }
    if let Some(base) = pointee {
        validate_base(base)?;
        if kind != Kind::Pointer {
            return Err(failure(
                "Only data pointers can reference a recovered layout",
            ));
        }
    }
    Ok(())
}

pub(super) fn convention_matches(convention: Convention, pointer_size: u64) -> bool {
    match convention {
        Convention::Unknown => true,
        Convention::Cdecl | Convention::Stdcall | Convention::Fastcall | Convention::Thiscall => {
            pointer_size == 4
        }
        Convention::SystemV64 | Convention::Win64 | Convention::Aapcs64 => pointer_size == 8,
    }
}
