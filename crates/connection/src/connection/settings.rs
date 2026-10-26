//! Cấu hình ConPTY — `ConptySettings` (thay WinRT `ValueSet`) + `PseudoConsoleFlags`
//! bitflags. Port 9 keys từ `ConptyConnection.cpp:245-324` (`Initialize(ValueSet)`).
//!
//! `ValueSet keys (OriginalKey -> field)`:
//! - `commandline`        → `commandline`
//! - `startingDirectory`  → `starting_directory`
//! - `startingTitle`      → `starting_title` (Optional)
//! - `initialRows`        → `initial_rows`
//! - `initialCols`        → `initial_cols`
//! - `environment`        → `environment`
//! - `flags`/`PseudoConsoleFlags` → `flags`
//! - `sessionId`          → `session_id` (`uuid::Uuid`)
//! - `profileGuid`         → `profile_guid` (Optional)
//! - `inheritCursor`       → `inherit_cursor`
//! - `textMeasurement`    → `text_measurement` (`u8`: 0=default/none, 1=graphemes,
//!   2=wcswidth, 3=console — port WT string keys sang numeric mode pass-through app layer)
//! - `ambiguousIsWide`    → `ambiguous_is_wide`
//! - `reloadEnvironmentVariables` → `reload_env` (default `true` per WT `ConptyConnection.cpp:298`).

use std::collections::HashMap;

use bitflags::bitflags;
use uuid::Uuid;

/// 9+3 keys — port `ConptyConnection.cpp:206-243` (`CreateSettings`).
#[derive(Debug, Clone)]
pub struct ConptySettings {
    /// Lệnh + args (e.g. `cmd.exe /k`).
    pub commandline: String,
    /// `cwd` — port `ExpandEnvironmentStringsW`私下 ở app layer (Win32_System_Environment thiếu).
    pub starting_directory: String,
    /// Tiêu đề console window (Optional).
    pub starting_title: Option<String>,
    /// Hàng khởi tạo (PTY size).
    pub initial_rows: u32,
    /// Cột khởi tạo.
    pub initial_cols: u32,
    /// Env vars truyền vào child process.
    pub environment: HashMap<String, String>,
    /// Bitmask `PseudoConsoleFlags` (inherit cursor, glyph width, ambiguous wide).
    pub flags: PseudoConsoleFlags,
    /// WT session id (duy nhất cho pane).
    pub session_id: Uuid,
    /// WT profile guid (Optional — mở mode khác từ cùng profile).
    pub profile_guid: Option<Uuid>,
    /// `PSEUDOCONSOLE_INHERIT_CURSOR` semantic → đặt bit `INHERIT_CURSOR`.
    pub inherit_cursor: bool,
    /// `text_measurement` mode — 0 default, copy từ WT profile.
    pub text_measurement: u8,
    /// `PSEUDOCONSOLE_AMBIGUOUS_IS_WIDE` — east-asian wide glyph fallback.
    pub ambiguous_is_wide: bool,
    /// `reloadEnvironmentVariables` — port `ConptyConnection.cpp:298-303`.
    /// `true` (default): rebuild env block từ `std::env::vars()` + `environment` overrides
    /// trước khi spawn child (gap #7 — used ở `launch_attached_client::build_env_block`).
    /// `false`: chỉ dùng `environment` HashMap (caller cung cấp env đầy đủ).
    pub reload_env: bool,
}

impl Default for ConptySettings {
    /// Default — port `ConptyConnection.cpp:245-247` (rows/cols default 80x25).
    fn default() -> Self {
        Self {
            commandline: String::new(),
            starting_directory: String::new(),
            starting_title: None,
            initial_rows: 25,
            initial_cols: 80,
            environment: HashMap::new(),
            flags: PseudoConsoleFlags::empty(),
            // `uuid::Uuid::new_v4()` thay `CoCreateGuid` (Win32_System_Com thiếu trong windows-rs 0.61).
            session_id: Uuid::new_v4(),
            profile_guid: None,
            inherit_cursor: false,
            text_measurement: 0,
            ambiguous_is_wide: false,
            reload_env: true,
        }
    }
}

// Bitflags — port `PSEUDOCONSOLE_*` constants. Windows SDK:
// - `PSEUDOCONSOLE_INHERIT_CURSOR = 0x1` (windows-rs 0.61 có sẵn).
// - `PSEUDOCONSOLE_GLYPH_WIDTH_WIDE = 0x4` (windows-rs 0.61 KHÔNG có — define thủ công).
// - `PSEUDOCONSOLE_AMBIGUOUS_IS_WIDE = 0x8` (windows-rs 0.61 KHÔNG có — define thủ công).
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct PseudoConsoleFlags: u32 {
        /// `0x1` — con trỏ kế thừa vị trí từ parent console.
        const INHERIT_CURSOR      = 0x1;
        /// `0x4` — glyph width được kích hoạt wide (winconpty.dll flag).
        const GLYPH_WIDTH_WIDE    = 0x4;
        /// `0x8` — ambiguous-width character handle as wide (East Asian locale).
        const AMBIGUOUS_IS_WIDE   = 0x8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_rows_cols() {
        let s = ConptySettings::default();
        assert_eq!((s.initial_rows, s.initial_cols), (25, 80));
        assert!(s.flags.is_empty());
        assert_eq!(s.text_measurement, 0);
    }

    #[test]
    fn bitflags_combinations() {
        let mut f = PseudoConsoleFlags::INHERIT_CURSOR | PseudoConsoleFlags::GLYPH_WIDTH_WIDE;
        assert_eq!(f.bits(), 0x5);
        f |= PseudoConsoleFlags::AMBIGUOUS_IS_WIDE;
        assert_eq!(f.bits(), 0xD);
        f.remove(PseudoConsoleFlags::GLYPH_WIDTH_WIDE);
        assert_eq!(f.bits(), 0x9);
        assert!(f.contains(PseudoConsoleFlags::INHERIT_CURSOR));
    }
}
