//! VT mouse reports. Port of `TerminalInput::HandleMouse` and its encoders
//! (`windows/src/terminal/input/mouseInput.cpp`): default X10, UTF-8 (1005)
//! and SGR (1006) encodings, motion filtering, wheel accumulation and
//! alternate scroll.

use core::dispatch::td::MouseModes;

/// `WHEEL_DELTA`: one wheel notch.
pub const WHEEL_DELTA: i32 = 120;
/// `s_MaxDefaultCoordinate`: the largest 0-based coordinate the X10 encoding can carry.
const MAX_DEFAULT_COORD: usize = 94;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

/// The Windows message the event corresponds to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    Down(Button),
    Up(Button),
    Move,
    /// `WM_MOUSEWHEEL`
    Wheel,
    /// `WM_MOUSEHWHEEL`
    HWheel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

/// What `HandleMouse` decided.
#[derive(Debug, PartialEq, Eq)]
pub enum Out {
    /// Not handled; the caller may use the event itself.
    None,
    /// Handled: send this (possibly empty = swallowed).
    Send(String),
    /// Alternate scroll: send this key (`up`/`down`/`left`/`right`) as a keystroke.
    AltScroll(&'static str),
}

/// `_mouseInputState`.
#[derive(Debug, Default)]
pub struct Reporter {
    accumulated: i32,
    last_pos: Option<(usize, usize)>,
    last_msg: Option<Msg>,
    last_tracking: (bool, bool, bool),
}

fn x_button(msg: Msg, hover: bool, delta: i32, m: Mods, sgr: bool) -> u32 {
    let mut v = match msg {
        Msg::Down(Button::Left) => 0,
        Msg::Down(Button::Middle) => 1,
        Msg::Down(Button::Right) => 2,
        Msg::Up(b) if sgr => match b {
            Button::Left => 0,
            Button::Middle => 1,
            Button::Right => 2,
        },
        Msg::Up(_) => 3,
        Msg::Move => {
            if sgr {
                3
            } else {
                0
            }
        }
        Msg::Wheel => {
            if delta > 0 {
                0x40
            } else {
                0x41
            }
        }
        Msg::HWheel => {
            if delta > 0 {
                0x43
            } else {
                0x42
            }
        }
    };
    if hover {
        v += 0x20;
    }
    if m.shift {
        v |= 0x04;
    }
    if m.alt {
        v |= 0x08;
    }
    if m.ctrl {
        v |= 0x10;
    }
    v
}

impl Reporter {
    /// `HandleMouse`. `pos` is the 0-based viewport cell (already clamped),
    /// `held` the button down during a move, `alt_screen` whether the alternate
    /// buffer is active.
    #[allow(clippy::too_many_arguments)]
    pub fn handle(
        &mut self,
        modes: &MouseModes,
        alt_screen: bool,
        msg: Msg,
        pos: (usize, usize),
        mods: Mods,
        delta: i32,
        held: Option<Button>,
    ) -> Out {
        // Changing a tracking mode clears the saved position/button.
        let tracking_now = (modes.press_release, modes.button_motion, modes.any_motion);
        if tracking_now != self.last_tracking {
            self.last_tracking = tracking_now;
            self.last_pos = None;
            self.last_msg = None;
        }
        let tracking = modes.tracking();
        let wheel = matches!(msg, Msg::Wheel | Msg::HWheel);
        let alt_scroll = alt_screen && modes.alternate_scroll && wheel && delta != 0;

        if delta.signum() != self.accumulated.signum() {
            self.accumulated = 0;
        }
        if wheel {
            self.accumulated += delta;
            if self.accumulated.abs() < WHEEL_DELTA {
                // Swallow sub-notch wheel input we would otherwise have reported.
                return if tracking || alt_scroll {
                    Out::Send(String::new())
                } else {
                    Out::None
                };
            }
            self.accumulated = 0;
        }

        if tracking {
            let hover = msg == Msg::Move;
            let is_button = !hover;
            let same = self.last_pos == Some(pos) && self.last_msg == Some(msg);
            let pressed = held.is_some();
            if is_button
                || (hover && modes.button_motion && !same && pressed)
                || (hover && modes.any_motion && !same)
            {
                if modes.button_motion || modes.any_motion {
                    self.last_pos = Some(pos);
                    self.last_msg = Some(msg);
                }
                // A move with a button down reports that button.
                let real = if hover {
                    held.map(Msg::Down)
                } else {
                    Some(msg)
                };
                return Out::Send(encode(
                    modes,
                    real.unwrap_or(msg),
                    hover,
                    held.is_some(),
                    pos,
                    mods,
                    delta,
                ));
            }
        }
        if alt_scroll {
            return Out::AltScroll(match (msg, delta > 0) {
                (Msg::Wheel, true) => "up",
                (Msg::Wheel, false) => "down",
                (_, true) => "right",
                (_, false) => "left",
            });
        }
        Out::None
    }
}

/// `_GenerateDefaultSequence` / `_GenerateUtf8Sequence` / `_GenerateSGRSequence`.
fn encode(
    modes: &MouseModes,
    msg: Msg,
    hover: bool,
    pressed: bool,
    pos: (usize, usize),
    m: Mods,
    delta: i32,
) -> String {
    let (x, y) = pos;
    if modes.utf8 || (!modes.sgr) {
        // Default and UTF-8 share the layout; default is limited to coordinate 94.
        if !modes.utf8 && (x > MAX_DEFAULT_COORD || y > MAX_DEFAULT_COORD) {
            return String::new();
        }
        // A hover with no button pressed is encoded as "released" (3).
        let b = if hover && !pressed {
            Msg::Up(Button::Left)
        } else {
            msg
        };
        let code = x_button(b, hover, delta, m, false);
        let ch = |v: u32| char::from_u32(32 + v).unwrap_or(' ');
        return format!("\x1b[M{}{}{}", ch(code), ch(x as u32 + 1), ch(y as u32 + 1));
    }
    let release = matches!(msg, Msg::Up(_));
    let code = x_button(msg, hover, delta, m, true);
    format!(
        "\x1b[<{code};{};{}{}",
        x + 1,
        y + 1,
        if release { 'm' } else { 'M' }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO: Mods = Mods {
        shift: false,
        alt: false,
        ctrl: false,
    };

    fn modes(sgr: bool) -> MouseModes {
        MouseModes {
            press_release: true,
            sgr,
            ..MouseModes::default()
        }
    }

    fn send(
        r: &mut Reporter,
        m: &MouseModes,
        msg: Msg,
        pos: (usize, usize),
        delta: i32,
        held: Option<Button>,
    ) -> String {
        match r.handle(m, false, msg, pos, NO, delta, held) {
            Out::Send(s) => s,
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn sgr_buttons_and_release() {
        let mut r = Reporter::default();
        let m = modes(true);
        assert_eq!(
            send(&mut r, &m, Msg::Down(Button::Left), (4, 9), 0, None),
            "\x1b[<0;5;10M"
        );
        assert_eq!(
            send(&mut r, &m, Msg::Up(Button::Left), (4, 9), 0, None),
            "\x1b[<0;5;10m"
        );
        assert_eq!(
            send(&mut r, &m, Msg::Down(Button::Right), (0, 0), 0, None),
            "\x1b[<2;1;1M"
        );
    }

    #[test]
    fn x10_limit_is_94_and_release_is_3() {
        let mut r = Reporter::default();
        let m = modes(false);
        assert_eq!(
            send(&mut r, &m, Msg::Down(Button::Left), (0, 0), 0, None),
            "\x1b[M !!"
        );
        assert_eq!(
            send(&mut r, &m, Msg::Up(Button::Right), (0, 0), 0, None),
            "\x1b[M#!!"
        );
        assert_eq!(
            send(&mut r, &m, Msg::Down(Button::Left), (95, 0), 0, None),
            ""
        );
        assert_eq!(
            send(&mut r, &m, Msg::Down(Button::Left), (94, 0), 0, None),
            "\x1b[M \x7f!"
        );
    }

    #[test]
    fn utf8_lifts_the_limit() {
        let mut r = Reporter::default();
        let m = MouseModes {
            press_release: true,
            utf8: true,
            ..MouseModes::default()
        };
        let s = send(&mut r, &m, Msg::Down(Button::Left), (200, 0), 0, None);
        assert_eq!(s.chars().nth(4), Some(char::from_u32(32 + 201).unwrap()));
    }

    #[test]
    fn wheel_accumulates_whole_notches() {
        let mut r = Reporter::default();
        let m = modes(true);
        assert_eq!(send(&mut r, &m, Msg::Wheel, (1, 1), 60, None), "");
        assert_eq!(
            send(&mut r, &m, Msg::Wheel, (1, 1), 60, None),
            "\x1b[<64;2;2M"
        );
        assert_eq!(
            send(&mut r, &m, Msg::Wheel, (1, 1), -120, None),
            "\x1b[<65;2;2M"
        );
        assert_eq!(
            send(&mut r, &m, Msg::HWheel, (1, 1), 120, None),
            "\x1b[<67;2;2M"
        );
        assert_eq!(
            send(&mut r, &m, Msg::HWheel, (1, 1), -120, None),
            "\x1b[<66;2;2M"
        );
    }

    #[test]
    fn motion_filtering_and_dedup() {
        let mut r = Reporter::default();
        let m = MouseModes {
            button_motion: true,
            sgr: true,
            ..MouseModes::default()
        };
        // No button: nothing in 1002.
        assert_eq!(
            r.handle(&m, false, Msg::Move, (1, 1), NO, 0, None),
            Out::None
        );
        let a = r.handle(&m, false, Msg::Move, (2, 1), NO, 0, Some(Button::Left));
        assert_eq!(a, Out::Send("\x1b[<32;3;2M".into()));
        // Same cell again: dropped.
        assert_eq!(
            r.handle(&m, false, Msg::Move, (2, 1), NO, 0, Some(Button::Left)),
            Out::None
        );
        let any = MouseModes {
            any_motion: true,
            sgr: true,
            ..MouseModes::default()
        };
        assert_eq!(
            r.handle(&any, false, Msg::Move, (3, 3), NO, 0, None),
            Out::Send("\x1b[<35;4;4M".into())
        );
    }

    #[test]
    fn modifiers_encode() {
        let mut r = Reporter::default();
        let m = modes(true);
        let mods = Mods {
            shift: false,
            alt: true,
            ctrl: true,
        };
        let o = r.handle(&m, false, Msg::Down(Button::Left), (0, 0), mods, 0, None);
        assert_eq!(o, Out::Send("\x1b[<24;1;1M".into()));
    }

    #[test]
    fn alternate_scroll_only_on_alt_screen() {
        let mut r = Reporter::default();
        let m = MouseModes::default();
        assert_eq!(
            r.handle(&m, false, Msg::Wheel, (0, 0), NO, 120, None),
            Out::None
        );
        assert_eq!(
            r.handle(&m, true, Msg::Wheel, (0, 0), NO, 120, None),
            Out::AltScroll("up")
        );
        assert_eq!(
            r.handle(&m, true, Msg::Wheel, (0, 0), NO, -120, None),
            Out::AltScroll("down")
        );
        let off = MouseModes {
            alternate_scroll: false,
            ..MouseModes::default()
        };
        assert_eq!(
            r.handle(&off, true, Msg::Wheel, (0, 0), NO, 120, None),
            Out::None
        );
    }
}
