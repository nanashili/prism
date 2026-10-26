use std::error::Error as StdError;
use std::fmt;

/// Reports invalid MCP input or a failed helper operation while retaining its cause.
#[derive(Debug)]
pub struct Error {
    source: Box<dyn StdError + Send + Sync>,
}

impl Error {
    pub(crate) fn from_source(source: Box<dyn StdError + Send + Sync>) -> Self {
        Self { source }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::from_source(std::io::Error::other(message).into())
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::from_source(source.into())
    }
}

const _: () = {
    const fn assert_error<T: StdError + Send + Sync + 'static>() {}
    assert_error::<Error>();
};
