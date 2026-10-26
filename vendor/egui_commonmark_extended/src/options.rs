//! Named rendering choices for the Markdown viewer's builder.

/// Selects how table widths respond to changes in their available space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TableLayout {
    /// Keep the renderer's default table sizing.
    #[default]
    Fixed,
    /// Refit tables when their content or available width changes.
    Responsive,
}

/// Controls whether images reveal their alternative text on hover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageAltText {
    /// Leave alternative text out of image tooltips.
    Hidden,
    /// Include alternative text when the pointer hovers over an image.
    #[default]
    OnHover,
}

/// Controls how image paths without a URI scheme are interpreted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageUriScheme {
    /// Apply the configured default scheme to paths that omit one.
    #[default]
    Implicit,
    /// Require every image path to provide its own URI scheme.
    Explicit,
}

/// Chooses which font family paints emphasized strong text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum StrongFont {
    /// Use the viewer's regular font selection.
    #[default]
    Regular,
    /// Use the separately configured strong font family.
    Dedicated,
}

/// Controls whether a document's metadata appears in its rendered output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Frontmatter {
    /// Omit the document's frontmatter.
    #[default]
    Hidden,
    /// Render the document's frontmatter.
    Visible,
}

/// Chooses whether a scrollable frame measures the entire document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderScope {
    /// Paint the visible document slice using cached measurements.
    #[default]
    Viewport,
    /// Paint all content to obtain positions outside the viewport.
    FullDocument,
}

impl From<bool> for TableLayout {
    fn from(value: bool) -> Self {
        if value {
            Self::Responsive
        } else {
            Self::Fixed
        }
    }
}

impl From<bool> for Frontmatter {
    fn from(value: bool) -> Self {
        if value {
            Self::Visible
        } else {
            Self::Hidden
        }
    }
}

impl From<bool> for RenderScope {
    fn from(value: bool) -> Self {
        if value {
            Self::FullDocument
        } else {
            Self::Viewport
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathSupport {
    Enabled,
    Disabled,
}

impl From<bool> for MathSupport {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListSpacing {
    Newline,
    Inline,
}

impl From<bool> for ListSpacing {
    fn from(value: bool) -> Self {
        if value {
            Self::Newline
        } else {
            Self::Inline
        }
    }
}
