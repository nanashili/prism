mod catalog;
mod cli;
mod configuration;
mod decompilation_arguments;
mod endpoint;
mod instructions;
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
pub use instructions::{BATCH_INSTRUCTIONS, RESPONSE_INSTRUCTIONS};
pub use scheduling::{CallClass, classify_call, durable_control};
pub use validation::{validate_arguments, validate_call};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Combined,
    Window,
    Analysis,
    Headless,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Combined | Self::Window => "prism",
            Self::Analysis => "prism-analysis",
            Self::Headless => "prism-headless",
        }
    }

    pub fn argument(self) -> Option<&'static str> {
        match self {
            Self::Combined => None,
            Self::Window => Some("--window"),
            Self::Analysis => Some("--analysis"),
            Self::Headless => Some("--headless"),
        }
    }
}
