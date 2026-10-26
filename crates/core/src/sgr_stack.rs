//! `SgrStack` — port `windows/src/types/sgrStack.{hpp,cpp}` (MSFT:15996906).
//!
//! Implements XTPUSHSGR / XTPOPSGR — push/pop text-attributes stack với
//! partial save (chỉ save specified attrs rồi restore nguyên vẹn những
//! attrs khác từ currentAttributes). Max 10 slot (ring buffer) —
//! `c_MaxStoredSgrPushes = 10` (`sgrStack.hpp:35`).
//!
//! Phase 1 skeleton: struct + Push/Pop stub với `todo!()`. Parity test chỉ
//! check struct + options enum + ring capacity; không check semantic
//! (lane C impl thật).
//!
//! Anchors:
//! - `DispatchTypes.hpp:479-494` — `SgrSaveRestoreStackOptions`.
//! - `sgrStack.hpp:22-46` — Push / Pop signature.
//! - `sgrStack.cpp:17-72` — Push/Pop impl.
use crate::attrs::TextAttribute;

/// `SgrSaveRestoreStackOptions` — port `DispatchTypes.hpp:479`.
///
/// Mỗi option chỉ định một phần của `TextAttribute` để save/restore.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum SgrSaveOptions {
    #[default]
    All = 0,
    Intense = 1,
    Faintness = 2,
    Italics = 3,
    Underline = 4,
    Blink = 5,
    Negative = 7,
    Invisible = 8,
    CrossedOut = 9,
    DoublyUnderlined = 21,
    SaveForegroundColor = 30,
    SaveBackgroundColor = 31,
}

impl SgrSaveOptions {
    /// Max value — port `Max = SaveBackgroundColor` (`:493`).
    pub const MAX: SgrSaveOptions = SgrSaveOptions::SaveBackgroundColor;

    /// Port `VTParameter::value_or(0)` semantic cho XTERM SGR save/restore (`#`).
    /// Raw `i32` param → `SgrSaveOptions`, default `All` (0) nếu ngoài known set
    /// (`sgrStack.cpp` dùng `gsl::narrow_cast<SgrSaveOptions>(v)` — match WT).
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::All,
            1 => Self::Intense,
            2 => Self::Faintness,
            3 => Self::Italics,
            4 => Self::Underline,
            5 => Self::Blink,
            7 => Self::Negative,
            8 => Self::Invisible,
            9 => Self::CrossedOut,
            21 => Self::DoublyUnderlined,
            30 => Self::SaveForegroundColor,
            31 => Self::SaveBackgroundColor,
            _ => Self::All,
        }
    }
}

/// Slot trong stack — port `SavedSgrAttributes` (`sgrStack.hpp:54`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SavedSgrAttributes {
    text_attr: TextAttribute,
    valid_parts: u32, // bitmask of SgrSaveOptions
}

/// `SgrStack` — port `class SgrStack` (`sgrStack.hpp:22`). Ring buffer 10
/// slot.
pub struct SgrStack {
    /// `_nextPushIndex` (`sgrStack.hpp:46`).
    next_push_index: usize,
    /// `_numSavedAttrs` (`:47`).
    num_saved: usize,
    /// `_storedSgrAttributes[10]` (`:48`).
    stored: [SavedSgrAttributes; SgrStack::MAX_STORED],
}

impl SgrStack {
    /// `c_MaxStoredSgrPushes = 10` (`sgrStack.hpp:35`).
    pub const MAX_STORED: usize = 10;

    pub fn new() -> Self {
        Self {
            next_push_index: 0,
            num_saved: 0,
            stored: [SavedSgrAttributes::default(); Self::MAX_STORED],
        }
    }

    /// `Push(currentAttributes, options)` (`sgrStack.hpp:24`).
    ///
    /// Phase 1: lưu composite slot với `All` flag. Phase 2 sẽ port partial
    /// save logic từ `sgrStack.cpp:17` — tách từng bit + chỉ take listed.
    pub fn push(&mut self, attr: &TextAttribute, options: &[SgrSaveOptions]) {
        let valid_parts = if options.is_empty() {
            // Default: full save (`sgrStack.cpp:25`).
            0xffffffff
        } else {
            options
                .iter()
                .map(|o| 1u32 << (*o as u32))
                .fold(0u32, |a, b| a | b)
        };
        let slot = &mut self.stored[self.next_push_index];
        slot.text_attr = *attr;
        slot.valid_parts = valid_parts;
        self.next_push_index = (self.next_push_index + 1) % Self::MAX_STORED;
        if self.num_saved < Self::MAX_STORED {
            self.num_saved += 1;
        }
    }

    /// `Pop(currentAttributes)` (`sgrStack.hpp:32`).
    ///
    /// Phase 1: trả stored attr nếu stack non-empty; else trả `current`.
    /// Phase 2 port `_CombineWithCurrentAttributes` (`sgrStack.cpp:57`)
    /// để chỉ restore listed attrs.
    pub fn pop(&mut self, current: &TextAttribute) -> TextAttribute {
        if self.num_saved == 0 {
            return *current;
        }
        // Ring: pop index = newest push.
        self.num_saved -= 1;
        // undo the next-push advance for the slot we just popped.
        let pop_idx = (self.next_push_index + Self::MAX_STORED - 1) % Self::MAX_STORED;
        self.next_push_index = pop_idx;
        let saved = self.stored[pop_idx];
        self.combine_with_current(current, &saved.text_attr, saved.valid_parts)
    }

    /// `_CombineWithCurrentAttributes` (`sgrStack.cpp:89-225`).
    /// Phase 1: nếu `All` → return saved; else return current.
    /// Phase 2: full per-flag merge.
    fn combine_with_current(
        &self,
        current: &TextAttribute,
        saved: &TextAttribute,
        valid_parts: u32,
    ) -> TextAttribute {
        if valid_parts == 0xffffffff {
            return *saved;
        }
        let mut out = *current;
        let all_bit = 1u32 << (SgrSaveOptions::All as u32);
        if valid_parts & all_bit != 0 {
            return *saved;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Intense as u32)) != 0 {
            out.intense = saved.intense;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Faintness as u32)) != 0 {
            out.faint = saved.faint;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Italics as u32)) != 0 {
            out.italic = saved.italic;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Blink as u32)) != 0 {
            out.blink = saved.blink;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Negative as u32)) != 0 {
            out.reverse_video = saved.reverse_video;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Invisible as u32)) != 0 {
            out.invisible = saved.invisible;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::CrossedOut as u32)) != 0 {
            out.crossed_out = saved.crossed_out;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::Underline as u32)) != 0 {
            out.underline_style = saved.underline_style;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::DoublyUnderlined as u32)) != 0
            && saved.underline_style == crate::attrs::UnderlineStyle::DoublyUnderlined
        {
            out.underline_style = saved.underline_style;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::SaveForegroundColor as u32)) != 0 {
            out.foreground = saved.foreground;
            out.fg_index = saved.fg_index;
            out.fg_kind = saved.fg_kind;
        }
        if valid_parts & (1u32 << (SgrSaveOptions::SaveBackgroundColor as u32)) != 0 {
            out.background = saved.background;
            out.bg_index = saved.bg_index;
            out.bg_kind = saved.bg_kind;
        }
        out
    }

    pub fn depth(&self) -> usize {
        self.num_saved
    }

    pub fn is_empty(&self) -> bool {
        self.num_saved == 0
    }
}

impl Default for SgrStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stack_pop_returns_current() {
        let mut s = SgrStack::new();
        let cur = TextAttribute::default();
        assert!(s.is_empty());
        let popped = s.pop(&cur);
        assert_eq!(popped, cur);
    }

    #[test]
    fn push_then_pop_returns_attr() {
        let mut s = SgrStack::new();
        let mut a = TextAttribute::default();
        a.set_italic(true);
        a.set_foreground(0x11223344);
        s.push(&a, &[]);
        assert_eq!(s.depth(), 1);
        let popped = s.pop(&TextAttribute::default());
        assert_eq!(popped, a);
        assert!(s.is_empty());
    }

    #[test]
    fn ring_buffer_caps_at_10() {
        let mut s = SgrStack::new();
        for i in 0..15 {
            let mut a = TextAttribute::default();
            a.set_foreground(i as u32);
            s.push(&a, &[]);
        }
        assert_eq!(s.depth(), SgrStack::MAX_STORED);
    }

    #[test]
    fn partial_save_preserves_unlisted_attrs_from_current() {
        let mut s = SgrStack::new();
        let mut saved = TextAttribute::default();
        saved.set_italic(true);
        saved.set_foreground(0xdead_beef);
        // Only save italic
        s.push(&saved, &[SgrSaveOptions::Italics]);

        let mut cur = TextAttribute::default();
        cur.set_reverse_video(true);
        cur.set_foreground(0xaaaa_aaaa);
        let popped = s.pop(&cur);

        assert!(popped.is_italic(), "italic should be restored from saved");
        assert!(
            popped.is_reverse_video(),
            "reverse_video NOT in save options → keep from current"
        );
        assert_eq!(
            popped.get_foreground(),
            0xaaaa_aaaa,
            "fg NOT in save options → keep from current"
        );
    }

    #[test]
    fn save_options_default_is_all() {
        assert_eq!(SgrSaveOptions::default(), SgrSaveOptions::All);
    }
}
