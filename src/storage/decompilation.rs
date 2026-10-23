mod inventory;
mod jobs;
mod pages;
mod results;

pub(crate) use jobs::{DecompilationJob, DecompilationJobSpec};
pub(crate) use results::DecompilationResult;

#[derive(Clone, Copy)]
pub(crate) struct DecompilationError<'a> {
    pub message: &'a str,
    pub details: Option<&'a str>,
}
