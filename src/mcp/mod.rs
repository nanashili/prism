//! Tool schemas, validation, admission limits, and the standard-input MCP helper.

mod catalog;
mod cli;
mod configuration;
mod decompilation_arguments;
mod endpoint;
mod instructions;
mod mode;
pub mod question;
mod scheduling;
mod session;
mod transport;
mod validation;

pub use catalog::{definitions, window_tool};
pub use cli::run_cli;
pub use configuration::configuration;
pub use decompilation_arguments::{MAX_DURABLE_ADDRESSES, validate_durable_address_count};
pub use endpoint::support_dir;
pub use instructions::{BATCH_INSTRUCTIONS, INVESTIGATION_INSTRUCTIONS, RESPONSE_INSTRUCTIONS};
pub use mode::Mode;
pub use scheduling::{CallClass, classify_call, durable_control};
pub use validation::{validate_arguments, validate_call};
