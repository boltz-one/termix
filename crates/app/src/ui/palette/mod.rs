//! Command palette (action mode) — port of `CommandPalette` + `FilteredCommand`
//! + `fzf`. `state` is pure; `view` is a stateless GPUI panel.

pub mod filtered_command;
pub mod fzf;
pub mod state;
pub mod view;

pub use filtered_command::{FilteredCommand, PaletteItem};
pub use state::PaletteState;
pub use view::{PaletteCallbacks, PalettePanel};

/// Actions reachable from the palette. Dispatch goes through `TerminalPage`
/// (the `PageActions` trait of P12 will replace this).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteAction {
    NewTab,
    DuplicateTab,
    NewTabProfile(usize),
    SwitchTab(usize),
    CloseTab,
    NextTab,
    PreviousTab,
    SplitRight,
    SplitDown,
    ClosePane,
    ZoomPane,
    SwapLeft,
    SwapRight,
    SwapUp,
    SwapDown,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    ResizeLeft,
    ResizeRight,
    ResizeUp,
    ResizeDown,
}

/// Default command list (names/chords from WT defaults).
pub fn default_commands() -> Vec<(PaletteItem, PaletteAction)> {
    let item = |name: &str, chord: &str| PaletteItem {
        name: name.into(),
        subtitle: String::new(),
        key_chord_text: chord.into(),
    };
    vec![
        (item("New tab", "Ctrl+Shift+T"), PaletteAction::NewTab),
        (item("Close tab", ""), PaletteAction::CloseTab),
        (item("Split pane right", "Alt+Shift++"), PaletteAction::SplitRight),
        (item("Split pane down", "Alt+Shift+-"), PaletteAction::SplitDown),
        (item("Close pane", "Ctrl+Shift+W"), PaletteAction::ClosePane),
        (item("Duplicate tab", "Ctrl+Shift+D"), PaletteAction::DuplicateTab),
        // WT leaves zoom and swap unbound.
        (item("Toggle pane zoom", ""), PaletteAction::ZoomPane),
        (item("Swap pane left", ""), PaletteAction::SwapLeft),
        (item("Swap pane right", ""), PaletteAction::SwapRight),
        (item("Swap pane up", ""), PaletteAction::SwapUp),
        (item("Swap pane down", ""), PaletteAction::SwapDown),
        (item("Move focus left", "Alt+Left"), PaletteAction::FocusLeft),
        (item("Move focus right", "Alt+Right"), PaletteAction::FocusRight),
        (item("Move focus up", "Alt+Up"), PaletteAction::FocusUp),
        (item("Move focus down", "Alt+Down"), PaletteAction::FocusDown),
        (item("Resize pane left", "Alt+Shift+Left"), PaletteAction::ResizeLeft),
        (item("Resize pane right", "Alt+Shift+Right"), PaletteAction::ResizeRight),
        (item("Resize pane up", "Alt+Shift+Up"), PaletteAction::ResizeUp),
        (item("Resize pane down", "Alt+Shift+Down"), PaletteAction::ResizeDown),
        (item("Next tab", "Ctrl+Tab"), PaletteAction::NextTab),
        (item("Previous tab", "Ctrl+Shift+Tab"), PaletteAction::PreviousTab),
    ]
}
