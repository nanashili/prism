pub(crate) mod android;
mod android_output;
mod engine;
pub(crate) mod file_inventory;
pub(crate) mod graph;
mod inventory;
pub(crate) mod references;
pub(crate) mod resources;
mod rizin;
mod rizin_inventory;
mod rizin_tools;
pub(crate) mod semantic;
pub(crate) mod type_recovery;
pub(crate) use engine::Engine;
pub(crate) mod local;
mod rea;

pub(crate) use rea::Rea;

#[cfg(any(feature = "prism-ghidra-rea", feature = "rea-rust-ghidra"))]
pub(crate) use rea::{
    DecompilationBatch, DecompilationBatchPoll, DecompilationItem,
    backend_identity as rea_backend_identity,
};
