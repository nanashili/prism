use std::error::Error;

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) fn failure(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    std::io::Error::other(message.into()).into()
}
