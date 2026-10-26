//! Bounded analysis experiments retain observations separately from specialist interpretations.

mod comparison;
mod diagnostics;
mod model;
mod native_signals;

pub(crate) use diagnostics::{diagnose, pseudocode_signals};
pub(crate) use model::{Adjustment, Observation, Plan};

#[cfg(test)]
mod tests;
pub(crate) use comparison::compare;
