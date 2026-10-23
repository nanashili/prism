pub(crate) mod archive;
mod archive_json;
mod document;
mod document_save;
pub(crate) mod files;
pub(crate) mod patch;

pub(crate) use document::open;
pub(crate) use document_save::save;

#[cfg(test)]
mod tests;
