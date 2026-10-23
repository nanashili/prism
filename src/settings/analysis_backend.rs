use crate::error::{Result, failure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalysisBackend {
    JavaGhidraRea,
    RustGhidraRea,
    Rizin,
    External,
}

impl AnalysisBackend {
    pub(super) fn default_provider() -> &'static str {
        if cfg!(feature = "prism-ghidra-rea") {
            "ghidra-rea"
        } else if cfg!(all(
            feature = "rea-rust-ghidra",
            any(target_os = "macos", target_os = "linux")
        )) {
            "ghidra-rust-rea"
        } else {
            "rizin"
        }
    }

    pub(super) fn from_provider(provider: &str) -> Self {
        match provider {
            "ghidra" | "ghidra-rea" => Self::JavaGhidraRea,
            "ghidra-rust-rea" => Self::RustGhidraRea,
            "rizin" => Self::Rizin,
            _ => Self::External,
        }
    }

    pub(crate) fn retains_prepared_session(self) -> bool {
        matches!(self, Self::JavaGhidraRea | Self::RustGhidraRea)
    }

    pub(crate) fn rea_provider(self, configured: &str) -> &str {
        match self {
            Self::JavaGhidraRea => "ghidra",
            Self::RustGhidraRea => "ghidra-rust",
            _ => configured,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::JavaGhidraRea => "Ghidra (Java through REA)",
            Self::RustGhidraRea => "Ghidra (Rust through REA)",
            Self::Rizin => "Rizin",
            Self::External => "REA",
        }
    }

    pub(crate) fn require_available(self) -> Result<()> {
        if self == Self::RustGhidraRea && !cfg!(any(target_os = "macos", target_os = "linux")) {
            return Err(failure(
                "Ghidra (Rust through REA) is supported only on macOS and Linux. Choose an available engine in Settings > Engines.",
            ));
        }

        let missing = match self {
            Self::JavaGhidraRea if !cfg!(feature = "prism-ghidra-rea") => "prism-ghidra-rea",
            Self::RustGhidraRea if !cfg!(feature = "rea-rust-ghidra") => "rea-rust-ghidra",
            _ => return Ok(()),
        };
        Err(failure(format!(
            "{} requires a Prism build with the {missing} feature. Choose an available engine in Settings > Engines or enable that feature.",
            self.label(),
        )))
    }
}
