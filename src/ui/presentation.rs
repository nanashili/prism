//! Named choices shared by presentation components.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    Selected,
    Unselected,
}

impl From<bool> for Selection {
    fn from(value: bool) -> Self {
        if value {
            Self::Selected
        } else {
            Self::Unselected
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActivityState {
    Active,
    Idle,
}

impl From<bool> for ActivityState {
    fn from(value: bool) -> Self {
        if value { Self::Active } else { Self::Idle }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThoughtDisplay {
    Visible,
    Hidden,
}

impl From<bool> for ThoughtDisplay {
    fn from(value: bool) -> Self {
        if value { Self::Visible } else { Self::Hidden }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceAccess {
    Allowed,
    Blocked,
}

impl From<bool> for ChoiceAccess {
    fn from(value: bool) -> Self {
        if value { Self::Allowed } else { Self::Blocked }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnEnd {
    Stopped,
    Finished,
}

impl From<bool> for TurnEnd {
    fn from(value: bool) -> Self {
        if value { Self::Stopped } else { Self::Finished }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChatOrigin {
    Document,
    Workspace,
}

impl From<bool> for ChatOrigin {
    fn from(value: bool) -> Self {
        if value {
            Self::Document
        } else {
            Self::Workspace
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpenTarget {
    Folder,
    File,
}

impl From<bool> for OpenTarget {
    fn from(value: bool) -> Self {
        if value { Self::Folder } else { Self::File }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveDestination {
    Choose,
    Current,
}

impl From<bool> for SaveDestination {
    fn from(value: bool) -> Self {
        if value { Self::Choose } else { Self::Current }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteTheme {
    Dark,
    Light,
}

impl From<bool> for PaletteTheme {
    fn from(value: bool) -> Self {
        if value { Self::Dark } else { Self::Light }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabCloseScope {
    ToTheRight,
    AllOthers,
}

impl From<bool> for TabCloseScope {
    fn from(value: bool) -> Self {
        if value {
            Self::ToTheRight
        } else {
            Self::AllOthers
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disclosure {
    Open,
    Closed,
}

impl From<bool> for Disclosure {
    fn from(value: bool) -> Self {
        if value { Self::Open } else { Self::Closed }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypingState {
    Typing,
    Idle,
}

impl From<bool> for TypingState {
    fn from(value: bool) -> Self {
        if value { Self::Typing } else { Self::Idle }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowPosition {
    First,
    Following,
}

impl From<bool> for RowPosition {
    fn from(value: bool) -> Self {
        if value { Self::First } else { Self::Following }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interaction {
    Active,
    Idle,
}

impl From<bool> for Interaction {
    fn from(value: bool) -> Self {
        if value { Self::Active } else { Self::Idle }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Border {
    Visible,
    Hidden,
}

impl From<bool> for Border {
    fn from(value: bool) -> Self {
        if value { Self::Visible } else { Self::Hidden }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Caret {
    Visible,
    Hidden,
}

impl From<bool> for Caret {
    fn from(value: bool) -> Self {
        if value { Self::Visible } else { Self::Hidden }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presence {
    Present,
    Absent,
}

impl From<bool> for Presence {
    fn from(value: bool) -> Self {
        if value { Self::Present } else { Self::Absent }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryDirection {
    Forward,
    Backward,
}

impl From<bool> for HistoryDirection {
    fn from(value: bool) -> Self {
        if value { Self::Forward } else { Self::Backward }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentMenuContext {
    Opening,
    Existing,
}

impl From<bool> for AgentMenuContext {
    fn from(value: bool) -> Self {
        if value { Self::Opening } else { Self::Existing }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeaderBehavior {
    Collapsible,
    Fixed,
}

impl From<bool> for HeaderBehavior {
    fn from(value: bool) -> Self {
        if value {
            Self::Collapsible
        } else {
            Self::Fixed
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CatalogScope {
    All,
    Preview,
}

impl From<bool> for CatalogScope {
    fn from(value: bool) -> Self {
        if value { Self::All } else { Self::Preview }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusTone {
    Good,
    Neutral,
}

impl From<bool> for StatusTone {
    fn from(value: bool) -> Self {
        if value { Self::Good } else { Self::Neutral }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputFocus {
    Focused,
    Unfocused,
}

impl From<bool> for InputFocus {
    fn from(value: bool) -> Self {
        if value {
            Self::Focused
        } else {
            Self::Unfocused
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReconstructionReadiness {
    Ready,
    Waiting,
}

impl From<bool> for ReconstructionReadiness {
    fn from(value: bool) -> Self {
        if value { Self::Ready } else { Self::Waiting }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentLaunch {
    Start,
    Observe,
}

impl From<bool> for AgentLaunch {
    fn from(value: bool) -> Self {
        if value { Self::Start } else { Self::Observe }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionAge {
    New,
    Existing,
}

impl From<bool> for SessionAge {
    fn from(value: bool) -> Self {
        if value { Self::New } else { Self::Existing }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChatNavigation {
    Available,
    Unavailable,
}

impl From<bool> for ChatNavigation {
    fn from(value: bool) -> Self {
        if value {
            Self::Available
        } else {
            Self::Unavailable
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoticeContext {
    History,
    Toast,
}

impl From<bool> for NoticeContext {
    fn from(value: bool) -> Self {
        if value { Self::History } else { Self::Toast }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstallationState {
    Installed,
    Available,
}

impl From<bool> for InstallationState {
    fn from(value: bool) -> Self {
        if value {
            Self::Installed
        } else {
            Self::Available
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolScope {
    WithMobile,
    Native,
}

impl From<bool> for ToolScope {
    fn from(value: bool) -> Self {
        if value {
            Self::WithMobile
        } else {
            Self::Native
        }
    }
}
