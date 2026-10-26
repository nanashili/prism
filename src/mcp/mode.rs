/// Selects the tool catalog and bridge endpoint used by an MCP connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mode {
    /// Exposes window, analysis, and reconstruction tools.
    Combined,
    /// Exposes window navigation and reconstruction tools.
    Window,
    /// Exposes analysis tools without window navigation.
    Analysis,
    /// Exposes analysis and reconstruction through a managed headless bridge.
    Headless,
}

impl Mode {
    /// Returns the server name advertised during MCP initialization.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Combined | Self::Window => "prism",
            Self::Analysis => "prism-analysis",
            Self::Headless => "prism-headless",
        }
    }

    /// Returns the helper argument selecting this mode, if one is needed.
    #[must_use]
    pub fn argument(self) -> Option<&'static str> {
        match self {
            Self::Combined => None,
            Self::Window => Some("--window"),
            Self::Analysis => Some("--analysis"),
            Self::Headless => Some("--headless"),
        }
    }
}
