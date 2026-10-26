use crate::error::Result;
use std::error::Error as StdError;

type Error = Box<dyn StdError + Send + Sync>;

/// Retain the original cause and report cleanup failures from an independent phase.
pub(crate) fn finish<T>(primary: Result<T>, cleanup: Result<()>, phase: &'static str) -> Result<T> {
    match (primary, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(Box::new(CleanupFailure {
            primary,
            cleanup,
            phase,
        })),
    }
}

#[derive(Debug)]
struct CleanupFailure {
    primary: Error,
    cleanup: Error,
    phase: &'static str,
}

impl std::fmt::Display for CleanupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}; {}: {}",
            self.primary, self.phase, self.cleanup
        )
    }
}

impl StdError for CleanupFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.primary.as_ref())
    }
}
