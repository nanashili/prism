mod classify;
mod index;
mod preferences;
mod query;
pub(crate) use preferences::Preferences;

use crate::model::Procedure;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Sort {
    Files,
    #[default]
    Modules,
    Address,
    Name,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Show {
    #[default]
    All,
    Yours,
    Renamed,
    Imports,
}

#[derive(Clone, Default)]
pub(crate) struct Query {
    pub filter: String,
    pub sort: Sort,
    pub show: Show,
    pub parent: String,
    pub offset: usize,
    pub renamed: BTreeMap<String, String>,
}

#[derive(Clone)]
pub(crate) enum Row {
    Group {
        key: String,
        name: String,
        count: usize,
    },
    Function {
        procedure: Procedure,
        location: String,
        inferred: bool,
    },
}

#[derive(Clone, Default)]
pub(crate) struct Counts {
    pub strings: Option<usize>,
    pub imports: Option<usize>,
    pub segments: Option<usize>,
    pub format: String,
    pub warning: Option<String>,
}

#[derive(Default)]
pub(crate) struct Page {
    pub rows: Vec<Row>,
    pub total: usize,
    pub matched: usize,
    pub counts: Counts,
    pub more: bool,
}
