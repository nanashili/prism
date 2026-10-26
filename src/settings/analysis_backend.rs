use crate::error::{Result, failure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalysisBackend {
    RustGhidraRea,
    Rizin,
    External,
}

impl AnalysisBackend {
    pub(super) fn default_provider() -> &'static str {
        if cfg!(feature = "rea-rust-ghidra") {
            "ghidra-rust-rea"
        } else {
            "rizin"
        }
    }

    pub(super) fn from_provider(provider: &str) -> Self {
        match provider {
            "ghidra-rust-rea" => Self::RustGhidraRea,
            "rizin" => Self::Rizin,
            _ => Self::External,
        }
    }

    pub(crate) fn retains_prepared_session(self) -> bool {
        self == Self::RustGhidraRea
    }

    pub(crate) fn rea_provider(self, configured: &str) -> &str {
        match self {
            Self::RustGhidraRea => "ghidra-rust",
            _ => configured,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::RustGhidraRea => "Ghidra (Rust through REA)",
            Self::Rizin => "Rizin",
            Self::External => "REA",
        }
    }

    pub(crate) fn require_available(self) -> Result<()> {
        if self == Self::RustGhidraRea
            && !cfg!(any(
                target_os = "macos",
                target_os = "linux",
                target_os = "windows"
            ))
        {
            return Err(failure(
                "Ghidra (Rust through REA) requires macOS, Linux or Windows. Choose an available engine in Settings > Engines.",
            ));
        }

        let missing = match self {
            Self::RustGhidraRea if !cfg!(feature = "rea-rust-ghidra") => "rea-rust-ghidra",
            _ => return Ok(()),
        };
        Err(failure(format!(
            "{} requires a Prism build with the {missing} feature. Choose an available engine in Settings > Engines or enable that feature.",
            self.label(),
        )))
    }
}
