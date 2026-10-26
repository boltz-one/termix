//! Chords the app owns (WT `defaults.json` bindings we implement). A bound
//! chord is never sent to the shell; the page handles it. Everything else
//! reaches the shell, as in WT where unbound keys are plain input.

use gpui::Keystroke;

/// True when `ks` is an app key binding that the terminal must let bubble up.
pub fn is_app_shortcut(ks: &Keystroke) -> bool {
    let m = &ks.modifiers;
    let key = ks.key.as_str();
    let digit = key.len() == 1 && matches!(key.as_bytes()[0], b'1'..=b'9');
    let arrow = matches!(key, "left" | "right" | "up" | "down");
    match (m.control, m.alt, m.shift) {
        // ctrl+shift: p palette, t new tab, w close pane, d duplicate, 1-9 new tab by profile, tab
        (true, false, true) => matches!(key, "p" | "t" | "w" | "d" | "tab") || digit,
        // ctrl+tab
        (true, false, false) => key == "tab",
        // ctrl+alt+1-9: switch to tab
        (true, true, false) => digit,
        // alt+arrows focus; alt+shift+arrows resize; alt+shift+-/+ split (Windows
        // folds Shift into the key: `_` / `+` arrive with the shift flag cleared)
        (false, true, false) => arrow || matches!(key, "_" | "+"),
        (false, true, true) => arrow || matches!(key, "-" | "minus" | "=" | "plus" | "_" | "+"),
        _ => false,
    }
}
