mod inference;
mod validation;

use serde::{Deserialize, Serialize};

pub(crate) use inference::infer;

/// Agent observations remain hypotheses, even after internal consistency checks pass.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evidence {
    pub pointer_size: u64,
    #[serde(default)]
    pub accesses: Vec<Access>,
    #[serde(default)]
    pub aliases: Vec<Alias>,
    #[serde(default)]
    pub prototypes: Vec<Prototype>,
    #[serde(default)]
    pub names: Vec<TypeName>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct Base {
    pub function: String,
    pub variable: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Bytes,
    Integer,
    Float,
    Pointer,
    FunctionPointer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Access {
    pub base: Base,
    pub offset: u64,
    pub width: u64,
    pub kind: Kind,
    #[serde(default = "one")]
    pub count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stride: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointee: Option<Base>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub site: String,
}

fn one() -> u64 {
    1
}

impl Access {
    pub(super) fn stride(&self) -> u64 {
        self.stride.unwrap_or(self.width)
    }

    pub(super) fn end(&self) -> Option<u64> {
        self.count
            .checked_sub(1)?
            .checked_mul(self.stride())?
            .checked_add(self.width)?
            .checked_add(self.offset)
    }
}

/// A proven same-origin pointer flow; interior pointers need distinct bases.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Alias {
    pub left: Base,
    pub right: Base,
    pub site: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TypeName {
    pub base: Base,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValueType {
    pub kind: Kind,
    pub width: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointee: Option<Base>,
}

/// Independently observed definitions or call sites for a single function.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prototype {
    pub function: String,
    pub convention: Convention,
    pub parameters: Vec<ValueType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns: Option<ValueType>,
    pub site: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Convention {
    Unknown,
    Cdecl,
    Stdcall,
    Fastcall,
    Thiscall,
    SystemV64,
    Win64,
    Aapcs64,
}

#[derive(Debug, Serialize)]
pub(crate) struct Model {
    pub types: Vec<RecoveredType>,
    pub prototypes: Vec<RecoveredPrototype>,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecoveredType {
    pub id: String,
    pub name: String,
    pub shape: &'static str,
    pub minimum_size: u64,
    pub consumers: Vec<Base>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Field {
    pub offset: u64,
    pub width: u64,
    pub kind: Kind,
    pub count: u64,
    pub stride: u64,
    pub pointee: Option<String>,
    pub targets: Vec<String>,
    pub evidence: Vec<Access>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecoveredPrototype {
    pub function: String,
    pub convention: Convention,
    pub parameters: Vec<RecoveredValue>,
    pub returns: Option<RecoveredValue>,
    pub evidence: Vec<Prototype>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct RecoveredValue {
    pub kind: Kind,
    pub width: u64,
    pub pointee: Option<String>,
}
