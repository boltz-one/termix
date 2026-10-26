//! Sub-type stubs cho trait method signatures. Phase 2 sẽ switch sang
//! full `bitflags`-backed enums ở `DispatchTypes.hpp`; phase 1 giữ `i32`-like
//! newtypes cho signature stability.
//!
//! Port: `windows/src/terminal/adapter/DispatchTypes.hpp` — nhóm enums.

/// `EraseType` — port `DispatchTypes::EraseType` (`DispatchTypes.hpp:367-373`).
/// Verbatim variant order: `ToEnd=0, FromBeginning=1, All=2, Scrollback=3`.
/// ED/EL/DECSED/DECSEL dùng.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum EraseType {
    #[default]
    ToEnd = 0,
    FromBeginning = 1,
    All = 2,
    Scrollback = 3,
}

impl EraseType {
    /// Port `VTParameter::operator T()` (`DispatchTypes.hpp:148`): omit
    /// (< 0) → `value_or(0)` → `ToEnd`. Out-of-range (>3) → clamp `ToEnd`
    /// (match C++ `static_cast<EraseType>(v)` với enum đỉnh `Scrollback=3`).
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::FromBeginning,
            2 => Self::All,
            3 => Self::Scrollback,
            _ => Self::ToEnd, // 0 hoặc omitted (v < 0) → ToEnd.
        }
    }
}

/// Port `DispatchTypes::TabClearType` (`DispatchTypes.hpp:446-451`).
/// `TBC` action (`CSI g`). Verbatim: `ClearCurrentColumn=0, ClearAllColumns=1`.
/// `VTParameter::operator TabClearType()` = `value_or(0)`.
impl TabClearType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::ClearAll,
            _ => Self::ClearCurrent, // 0 / omit / out-of-range → ClearCurrent.
        }
    }
}

/// Port `DispatchTypes::CursorStyle` (`DispatchTypes.hpp:541-549`).
/// `DECSCUSR` action (`CSI Ps SP q`). `VTParameter::operator CursorStyle()`.
/// Verbatim mapping: 0/omit → `UserDefault`, 1 → `BlinkingBlock`,
/// 2 → `SteadyBlock`, 3 → `BlinkingUnderline`, 4 → `SteadyUnderline`,
/// 5 → `BlinkingBar`, 6 → `SteadyBar`, out-of-range → `UserDefault`.
impl CursorStyle {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::BlinkingBlock,
            2 => Self::SteadyBlock,
            3 => Self::BlinkingUnderline,
            4 => Self::SteadyUnderline,
            5 => Self::BlinkingBar,
            6 => Self::SteadyBar,
            _ => Self::UserDefault, // 0 / omit / >6 → UserDefault (WT parity).
        }
    }
}

/// Port `DispatchTypes::ReportingPermission` (`DispatchTypes.hpp`).
/// `DECREQTPARM` — `parameters.at(0)` cast qua `VTParameter::operator ReportingPermission()`.
/// WT: 0 → `Granted` (repo sent on request), 1 → `Denied`. Out-of-range → `Granted`.
impl ReportingPermission {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Denied,
            _ => Self::Granted, // 0 / omit → Granted.
        }
    }
}

/// Port `DispatchTypes::ANSIStandardMode` constructor `ANSIStandardMode(int)`.
/// WT `DispatchTypes.hpp:515-549`. SM/RM/DECRQM dùng.
/// Only LNM (20) và IRM (4) là ANSI standard mode — phần còn lại là `DECPrivateMode`.
pub fn ansi_standard_mode_from_i32(v: i32) -> ModeParams {
    match v {
        4 => ModeParams::InsertReplaceMode,       // IRM
        20 => ModeParams::LnmLineFeedNewLineMode, // LNM
        _ => ModeParams::Unknown,                 // off-spec ANSI mode — WT tương tự Unknown.
    }
}

/// Port `DispatchTypes::DECPrivateMode` constructor `DECPrivateMode(int)`.
/// WT `DispatchTypes.hpp:515-549`. Một loạt `DECPrivateMode(N)`.
pub fn dec_private_mode_from_i32(v: i32) -> ModeParams {
    match v {
        1 => ModeParams::ApplicationCursorMode, // DECCKM
        3 => ModeParams::ColumnMode132,         // DECCOLM
        5 => ModeParams::ReverseColors,         // DECSCNM
        6 => ModeParams::OriginModeRelative,    // DECOM
        7 => ModeParams::AutoWrapMode,          // DECAWM
        8 => ModeParams::AutoRepeatMode,        // DECARM
        66 => ModeParams::NumericKeypadMode,    // DECNKM
        67 => ModeParams::BackarrowKeyMode,     // DECBKM
        12 => ModeParams::CursorVisibilityMode, // ATT610 blink (alias DECTCEM)
        25 => ModeParams::CursorVisibilityMode, // DECTCEM
        40 => ModeParams::ColumnMode132,        // XTERM_EnableDECCOLMSupport
        47 => ModeParams::AlternateScreenBuffer,
        1047 => ModeParams::AlternateScreenBuffer,
        1049 => ModeParams::AlternateScreenBuffer,
        1000 => ModeParams::MouseTracking, // VT200_MOUSE
        1002 => ModeParams::ButtonEventMouseTracking, // BUTTON_EVENT_MOUSE
        1003 => ModeParams::AnyEventMouseTracking,
        1004 => ModeParams::FocusEvent,
        1005 => ModeParams::Utf8MouseEncoding, // UTF8_EXTENDED
        1006 => ModeParams::SgrMouseTracking,  // SGR_EXTENDED
        1007 => ModeParams::AlternateScroll,
        2004 => ModeParams::BracketedPaste, // XTERM_BracketedPaste
        _ => ModeParams::Unknown,           // off-spec — WT cũng drop.
    }
}

/// Port `DispatchTypes::ANSIStandardStatus` constructor `ANSIStandardStatus(int)`.
/// WT `DispatchTypes.hpp:496-498`. DSR ANSI path — value truyền thẳng.
pub fn ansi_standard_status_from_i32(v: i32) -> StatusType {
    match v {
        5 => StatusType::OperatingStatus,
        6 => StatusType::CursorPositionReport,
        _ => StatusType::OperatingStatus, // omit + off-spec → default.
    }
}

/// Port `DispatchTypes::DECPrivateStatus` constructor `DECPrivateStatus(int)`.
/// WT `DispatchTypes.hpp:501-512`. DSR? path.
pub fn dec_private_status_from_i32(v: i32) -> StatusType {
    match v {
        6 => StatusType::ExtendedCursorPositionReport,
        15 => StatusType::PrinterStatus,
        25 => StatusType::UserDefinedKeys,
        26 => StatusType::KeyboardStatus,
        55 => StatusType::LocatorStatus,
        56 => StatusType::LocatorIdentity,
        62 => StatusType::MacroSpaceReport,
        63 => StatusType::MemoryChecksum,
        75 => StatusType::DataIntegrity,
        85 => StatusType::MultipleSessionStatus,
        _ => StatusType::OperatingStatus,
    }
}

/// `LineFeedType` — port `DispatchTypes::LineFeedType`
/// (`DispatchTypes.hpp:613-618`). 3 variants (rework từ 6-variant legacy):
/// `WithReturn` (CR+LF), `WithoutReturn` (LF only), `DependsOnMode` (read LNM).
/// IND/NEL/LF/FF/VT dùng.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum LineFeedType {
    #[default]
    DependsOnMode,
    WithReturn,
    WithoutReturn,
}

/// `TabClearType` — port `DispatchTypes::TabClearType`. TBC action.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum TabClearType {
    #[default]
    ClearCurrent,
    ClearAll,
}

/// `CursorStyle` — port `DispatchTypes::CursorStyle`. DECSCUSR.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum CursorStyle {
    #[default]
    UserDefault,
    BlinkingBlock,
    SteadyBlock,
    BlinkingUnderline,
    SteadyUnderline,
    BlinkingBar,
    SteadyBar,
}

/// `ColorItem` — port `DispatchTypes::ColorItem`. DECAC.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ColorItem {
    #[default]
    Text,
    Background,
    Cursor,
    Highlight,
}

/// `WindowManipulationType` — port `DispatchTypes::WindowManipulationType`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum WindowManipulationType {
    #[default]
    None,
    DeiconifyWindow,
    IconifyWindow,
    LowerWindow,
    RaiseWindow,
    RefreshWindow,
    RestoreWindowSize,
    SetWindowSizeChars,
    SetWindowSizePixels,
}

/// `ChangeExtent` (`DispatchTypes::ChangeAttributesChangeExtent`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ChangeExtent {
    #[default]
    All,
    Stream,
    Rectangle,
}

/// `ReportingPermission` — DECREQTPARM.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ReportingPermission {
    #[default]
    Granted,
    Denied,
}

/// `CharsetSize` — DECAUPSS.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum CharsetSize {
    #[default]
    Size94,
    Size96,
}

/// `ReportFormat` — DECRSTS/DECRQTSR.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ReportFormat {
    #[default]
    TerminalState,
    ColorTable,
}

/// `PresentationReportFormat` — DECRQPSR/DECRSPS.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum PresentationReportFormat {
    #[default]
    CursorInformation,
    TabulationInformation,
    Presentation,
}

/// `StatusType` — DSR.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum StatusType {
    #[default]
    OperatingStatus,
    CursorPositionReport,
    ExtendedCursorPositionReport,
    PrinterStatus,
    UserDefinedKeys,
    KeyboardStatus,
    LocatorStatus,
    LocatorIdentity,
    MacroSpaceReport,
    MemoryChecksum,
    DataIntegrity,
    MultipleSessionStatus,
}

/// `ModeParams` — SM/RM/DECRQM. Port `DispatchTypes::ModeParams`
/// (`DispatchTypes.hpp:519+`). LNM (20) ANSI added Wave 0.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ModeParams {
    #[default]
    Unknown,
    InsertReplaceMode,      // IRM (4)
    LnmLineFeedNewLineMode, // LNM (20) ANSI — Wave 0 add
    CursorVisibilityMode,   // DECTCEM (25)
    OriginModeRelative,     // DECOM (6)
    ColumnMode132,          // DECCOLM (3)
    ColumnMode80,
    AutoWrapMode,             // DECAWM (7)
    AutoRepeatMode,           // DECARM (8)
    NumericKeypadMode,        // DECNKM (66)
    BackarrowKeyMode,         // DECBKM (67)
    ApplicationCursorMode,    // DECCKM (1)
    AlternateScreenBuffer,    // ?47,1047,1049
    ReverseColors,            // ?5
    BracketedPaste,           // 2004
    FocusEvent,               // 1004
    MouseTracking,            // 1000..1006
    ButtonEventMouseTracking, // 1002
    Utf8MouseEncoding,        // 1005
    SgrMouseTracking,         // 1006
    AnyEventMouseTracking,    // 1003
    AlternateScroll,          // 1007
}

/// `GraphicsOptions` — port `DispatchTypes::GraphicsOptions`
/// (`DispatchTypes.hpp:391-453`). SGR param code. Verbatim numeric values
/// (xterm SGR spec — wt giữ explicit values cho `38`/`48`/`58` double-meaning
/// marker; variant name chọn theo primary meaning).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum GraphicsOptions {
    #[default]
    Off = 0,
    Intense = 1,
    /// `2` — dual meaning: Faint OR RGB-color subparam marker (`38;2;r;g;b`).
    RGBColorOrFaint = 2,
    Italics = 3,
    /// `4` — Underline. Extended form `SGR 4:x` использует sub params.
    Underline = 4,
    /// `5` — dual meaning: Blink OR 256-index subparam marker (`38;5;n`).
    BlinkOrXterm256Index = 5,
    RapidBlink = 6,
    Negative = 7,
    Invisible = 8,
    CrossedOut = 9,
    DoublyUnderlined = 21,
    NotIntenseOrFaint = 22,
    NotItalics = 23,
    NoUnderline = 24,
    Steady = 25,
    Positive = 27,
    Visible = 28,
    NotCrossedOut = 29,
    ForegroundBlack = 30,
    ForegroundRed = 31,
    ForegroundGreen = 32,
    ForegroundYellow = 33,
    ForegroundBlue = 34,
    ForegroundMagenta = 35,
    ForegroundCyan = 36,
    ForegroundWhite = 37,
    ForegroundExtended = 38,
    ForegroundDefault = 39,
    BackgroundBlack = 40,
    BackgroundRed = 41,
    BackgroundGreen = 42,
    BackgroundYellow = 43,
    BackgroundBlue = 44,
    BackgroundMagenta = 45,
    BackgroundCyan = 46,
    BackgroundWhite = 47,
    BackgroundExtended = 48,
    BackgroundDefault = 49,
    Overline = 53,
    NoOverline = 55,
    UnderlineColor = 58,
    UnderlineColorDefault = 59,
    BrightForegroundBlack = 90,
    BrightForegroundRed = 91,
    BrightForegroundGreen = 92,
    BrightForegroundYellow = 93,
    BrightForegroundBlue = 94,
    BrightForegroundMagenta = 95,
    BrightForegroundCyan = 96,
    BrightForegroundWhite = 97,
    BrightBackgroundBlack = 100,
    BrightBackgroundRed = 101,
    BrightBackgroundGreen = 102,
    BrightBackgroundYellow = 103,
    BrightBackgroundBlue = 104,
    BrightBackgroundMagenta = 105,
    BrightBackgroundCyan = 106,
    BrightBackgroundWhite = 107,
}

impl GraphicsOptions {
    /// Port `VTParameter::value_or(0)` semantic — convert raw `i32` param
    /// sang `GraphicsOptions`, default `Off` nếu ngoài range.
    pub fn from_i32(v: i32) -> Self {
        // Verbatim match — `gsl::narrow_cast<GraphicsOptions>(v)` trong C++.
        match v {
            0 => Self::Off,
            1 => Self::Intense,
            2 => Self::RGBColorOrFaint,
            3 => Self::Italics,
            4 => Self::Underline,
            5 => Self::BlinkOrXterm256Index,
            6 => Self::RapidBlink,
            7 => Self::Negative,
            8 => Self::Invisible,
            9 => Self::CrossedOut,
            21 => Self::DoublyUnderlined,
            22 => Self::NotIntenseOrFaint,
            23 => Self::NotItalics,
            24 => Self::NoUnderline,
            25 => Self::Steady,
            27 => Self::Positive,
            28 => Self::Visible,
            29 => Self::NotCrossedOut,
            30 => Self::ForegroundBlack,
            31 => Self::ForegroundRed,
            32 => Self::ForegroundGreen,
            33 => Self::ForegroundYellow,
            34 => Self::ForegroundBlue,
            35 => Self::ForegroundMagenta,
            36 => Self::ForegroundCyan,
            37 => Self::ForegroundWhite,
            38 => Self::ForegroundExtended,
            39 => Self::ForegroundDefault,
            40 => Self::BackgroundBlack,
            41 => Self::BackgroundRed,
            42 => Self::BackgroundGreen,
            43 => Self::BackgroundYellow,
            44 => Self::BackgroundBlue,
            45 => Self::BackgroundMagenta,
            46 => Self::BackgroundCyan,
            47 => Self::BackgroundWhite,
            48 => Self::BackgroundExtended,
            49 => Self::BackgroundDefault,
            53 => Self::Overline,
            55 => Self::NoOverline,
            58 => Self::UnderlineColor,
            59 => Self::UnderlineColorDefault,
            90 => Self::BrightForegroundBlack,
            91 => Self::BrightForegroundRed,
            92 => Self::BrightForegroundGreen,
            93 => Self::BrightForegroundYellow,
            94 => Self::BrightForegroundBlue,
            95 => Self::BrightForegroundMagenta,
            96 => Self::BrightForegroundCyan,
            97 => Self::BrightForegroundWhite,
            100 => Self::BrightBackgroundBlack,
            101 => Self::BrightBackgroundRed,
            102 => Self::BrightBackgroundGreen,
            103 => Self::BrightBackgroundYellow,
            104 => Self::BrightBackgroundBlue,
            105 => Self::BrightBackgroundMagenta,
            106 => Self::BrightBackgroundCyan,
            107 => Self::BrightBackgroundWhite,
            _ => Self::Off,
        }
    }
}

/// `LogicalAttributeOptions` — port `DispatchTypes::LogicalAttributeOptions`
/// (`DispatchTypes.hpp:455-460`). DECSCA — character protection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum LogicalAttributeOptions {
    #[default]
    Default = 0,
    Protected = 1,
    Unprotected = 2,
}

impl LogicalAttributeOptions {
    /// `VTParameter::value_or(0)` semantic.
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Protected,
            2 => Self::Unprotected,
            _ => Self::Default,
        }
    }
}

/// `WindowManipulationType::from_i32` — port `DispatchTypes::WindowManipulationType(int)`.
/// xterm dtterm: 0/omit → `None`, 1 → Deiconify, 2 → Iconify, 3 → Lower,
/// 4 → Raise, 5 → Refresh, 6 → RestoreWindowSize, 7 → SetWindowSize chars,
/// 8 → SetWindowSize pixels. Out-of-range → `None` (WT default).
impl WindowManipulationType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::DeiconifyWindow,
            2 => Self::IconifyWindow,
            3 => Self::LowerWindow,
            4 => Self::RaiseWindow,
            5 => Self::RefreshWindow,
            6 => Self::RestoreWindowSize,
            7 => Self::SetWindowSizeChars,
            8 => Self::SetWindowSizePixels,
            _ => Self::None,
        }
    }
}

/// `ChangeExtent::from_i32` — DECSACE (`DispatchTypes::ChangeAttributesChangeExtent`).
/// 0/omit → All, 1 → Stream, 2 → Rectangle. Out-of-range → All.
impl ChangeExtent {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Stream,
            2 => Self::Rectangle,
            _ => Self::All,
        }
    }
}

/// `ColorItem::from_i32` — DECAC (`DispatchTypes::ColorItem`).
/// 0/omit → Text (default), 1 → Text, 2 → Background, 3 → Cursor,
/// 4 → Highlight. Out-of-range → Text. WT `VTParameter::operator ColorItem()`.
impl ColorItem {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Text,
            2 => Self::Background,
            3 => Self::Cursor,
            4 => Self::Highlight,
            _ => Self::Text,
        }
    }
}

/// `ReportFormat::from_i32` — DECRSTS/DECRQTSR. 0/omit → TerminalState,
/// 1 → ColorTable. Out-of-range → TerminalState (default).
impl ReportFormat {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::ColorTable,
            _ => Self::TerminalState,
        }
    }
}

/// `PresentationReportFormat::from_i32` — DECRQPSR/DECRSPS.
/// 0/omit → CursorInformation, 1 → TabulationInformation, 2 → Presentation.
/// Out-of-range → CursorInformation (default).
impl PresentationReportFormat {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::TabulationInformation,
            2 => Self::Presentation,
            _ => Self::CursorInformation,
        }
    }
}
