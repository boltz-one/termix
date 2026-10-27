# Phase 05 Implementation Plan — types crate (til + types utils)

**Phase:** P05 types-til  
**Loop:** 09  
**Effort:** 90h  
**Wave:** 1 (parallel with P02-P04, P06-P07)  
**Lane:** high-risk (new crate, 198 tests, ADR decisions, consumed by P08/P10/P07)

## Objective

Port WT `til/*` and `types/*` utilities into `crates/types`, establishing geometric primitives (point/size/rect/viewport), run-length encoding, color operations, Unicode width detection, and conversion helpers. Replace C++ utils with Rust std equivalents per ADR-0004 where semantics match; port custom behavior where needed.

## Context

- **Spec:** `plans/20261026-0111-wt-full-port-master-plan/phase-05-types-til.md`
- **Ledger:** `parity/ledger/P05-types-til/files.csv` (67 files: 24 til tests, 3 types tests, headers/impls)
- **Existing:** `crates/types/src/lib.rs` (229 bytes skeleton), `crates/core/src/screen.rs` (170 lines Viewport duplicate to migrate in P08)
- **Consumers:** P08 (buffer), P10 (renderer), P07 (settings)
- **Dependencies:** P01 complete (foundation, ledger, xtask)
- **ADR:** ADR-0004 (til types replacement) — finalize with port vs replace decisions

## Strategy

**Test-first:** Port WT tests → implement to green → verify coverage ≥90%/75%.

**Module priority:** Foundational types first (point/size/rect) → rle → viewport → color → width → unicode → utils.

**ADR decisions inline:** Mark each WT type `port` | `replaced(std equivalent)` | `out-of-scope(P15)` in ledger during implementation.

## Architecture

```
crates/types/src/
├── lib.rs              # Public API + re-exports
├── point.rs            # til::point (i32 x/y, checked ops, From conversions)
├── size.rs             # til::size (i32 width/height, area, checked ops)
├── rect.rs             # til::rect + inclusive_rect (inclusive bounds, intersect/union/contains)
├── viewport.rs         # Viewport (migrate from core::screen, add WT walk/clamp)
├── rle.rs              # til::rle run-length encoding
├── color.rs            # ColorFix (Oklab port per ADR-0015), color ops
├── width.rs            # CodepointWidthDetector (Unicode width tables from WT)
├── unicode.rs          # til::unicode helpers (combining marks, grapheme utils)
├── convert.rs          # UTF-8/16 conversion (types/convert.cpp port)
├── utils.rs            # GUID parse, console helpers behind cfg(windows)
└── enumset.rs          # til::enumset (or use bitflags crate if equivalent)

crates/types/tests/
├── wt_til_point.rs
├── wt_til_size.rs
├── wt_til_rect.rs
├── wt_til_rle.rs
├── wt_til_color.rs
├── wt_til_unicode.rs
├── wt_til_operators.rs
├── wt_til_math.rs
├── wt_types_viewport.rs
├── wt_types_width.rs
└── wt_types_utils.rs
```

## Implementation Steps

### Step 1: Ledger triage + ADR-0004 finalization (8h)

**Goal:** Classify every P05 ledger row as `port` | `replaced` | `out-of-scope`.

1. Read all 67 ledger rows from `parity/ledger/P05-types-til/files.csv`
2. Classify each WT file:
   - **Port:** custom behavior (rle, Viewport walk/clamp, CodepointWidthDetector tables, ColorFix Oklab, GUID parse)
   - **Replaced:** Rust std equivalents (til::small_vector → Vec, til::pmr → std allocator, til::string → String/str)
   - **Out-of-scope:** UIA files (ScreenInfoUiaProviderBase, TermControlUiaProvider, UiaTextRangeBase, UiaTracing) → P15
3. Update ADR-0004 with final replacement map table
4. Mark ledger rows: set `adr` column to `ADR-0004` for replaced items, `ADR-0009` for UIA out-of-scope
5. Output: `docs/decisions/0004-til-types-replacement.md` finalized, ledger rows classified

**Files:**
- Modify: `docs/decisions/0004-til-types-replacement.md`
- Modify: `parity/ledger/P05-types-til/files.csv` (adr column only)

**Success:** Every row has `port|replaced|out-of-scope` decision; ADR-0004 table complete.

---

### Step 2: Point/Size/Rect — foundations (12h)

**Goal:** Port til::point, til::size, til::rect with checked arithmetic and conversions.

**WT anchors:**
- `til/point.h` — `til::point` (i32 x/y, operators, `cast` method)
- `til/size.h` — `til::size` (i32 width/height, `area()`, narrow_height/width)
- `til/rectangle.h` — `til::rect` (left/top/right/bottom exclusive), `til::inclusive_rect`
- `til/ut_til/PointTests.cpp` — 17 test methods
- `til/ut_til/SizeTests.cpp` — 15 test methods
- `til/ut_til/RectangleTests.cpp` — 28 test methods
- `til/ut_til/OperatorTests.cpp` — operator overloads
- `til/ut_til/MathTests.cpp` — math helpers (clamp, abs, etc.)

**Implementation:**

1. **Port tests first (red):**
   - `crates/types/tests/wt_til_point.rs` — port PointTests methods 1:1, translate C++ VERIFY → Rust assert_eq!
   - `crates/types/tests/wt_til_size.rs` — SizeTests methods
   - `crates/types/tests/wt_til_rect.rs` — RectangleTests methods
   - `crates/types/tests/wt_til_operators.rs` — OperatorTests (Add/Sub/Mul/Div for Point/Size/Rect)
   - `crates/types/tests/wt_til_math.rs` — MathTests
   - Run `cargo test -p types` → all fail (red)

2. **Implement types (green):**
   - `crates/types/src/point.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
     pub struct Point { pub x: i32, pub y: i32 }
     // checked_add/sub, From<(i32,i32)>, operators via std::ops
     ```
   - `crates/types/src/size.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
     pub struct Size { pub width: i32, pub height: i32 }
     // area() -> Option<i32> (checked_mul), checked ops
     ```
   - `crates/types/src/rect.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
     pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }
     // Exclusive bounds (right/bottom are one-past-end)
     // intersect, union, contains, offset, inflate, width(), height()
     
     pub struct InclusiveRect { /* left..=right, top..=bottom */ }
     // WT til::inclusive_rect semantics
     ```
   - Add `#![forbid(unsafe_code)]` to `lib.rs`
   - Re-export from `lib.rs`: `pub use {point::Point, size::Size, rect::{Rect, InclusiveRect}};`

3. **Run tests (green):**
   - `cargo test -p types -- wt_til_point wt_til_size wt_til_rect wt_til_operators wt_til_math` → all pass

4. **Update ledger:**
   - Mark `til/point.h`, `til/size.h`, `til/rectangle.h`, `til/ut_til/PointTests.cpp`, `til/ut_til/SizeTests.cpp`, `til/ut_til/RectangleTests.cpp`, `til/ut_til/OperatorTests.cpp`, `til/ut_til/MathTests.cpp` as `status: ported`, `rust_path: types/src/{point,size,rect}.rs`, `test_ids: wt_til_*`

**Files:**
- Create: `types/src/point.rs`, `types/src/size.rs`, `types/src/rect.rs`
- Create: `types/tests/wt_til_point.rs`, `wt_til_size.rs`, `wt_til_rect.rs`, `wt_til_operators.rs`, `wt_til_math.rs`
- Modify: `types/src/lib.rs` (add modules, re-exports, `#![forbid(unsafe_code)]`)
- Modify: `parity/ledger/P05-types-til/files.csv` (8 rows: status→ported, rust_path, test_ids)

**Success:** `cargo test -p types -- wt_til_point wt_til_size wt_til_rect wt_til_operators wt_til_math` pass (60+ tests green).

---

### Step 3: RLE (Run-Length Encoding) (10h)

**Goal:** Port til::rle with custom run-length encoding for attribute spans.

**WT anchors:**
- `til/rle.h` — `til::rle<T>` (vector of runs with value + length)
- `til/ut_til/RunLengthEncodingTests.cpp` — 12 test methods

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_til_rle.rs` — port RunLengthEncodingTests 1:1
   - Run `cargo test -p types -- wt_til_rle` → fail

2. **Implement (green):**
   - `crates/types/src/rle.rs`:
     ```rust
     #[derive(Clone, Debug, PartialEq)]
     pub struct Rle<T> {
         runs: Vec<Run<T>>,
     }
     
     struct Run<T> { value: T, length: usize }
     
     impl<T> Rle<T> {
         pub fn new() -> Self;
         pub fn push(&mut self, value: T, length: usize) where T: PartialEq;
         pub fn get(&self, index: usize) -> Option<&T>;
         pub fn iter(&self) -> impl Iterator<Item = (&T, usize)>;
         // Coalesce adjacent runs with same value
     }
     ```
   - Re-export: `pub use rle::Rle;`

3. **Run tests:** `cargo test -p types -- wt_til_rle` → pass

4. **Update ledger:** Mark `til/rle.h`, `til/ut_til/RunLengthEncodingTests.cpp` as ported

**Files:**
- Create: `types/src/rle.rs`, `types/tests/wt_til_rle.rs`
- Modify: `types/src/lib.rs`, `parity/ledger/P05-types-til/files.csv` (2 rows)

**Success:** `cargo test -p types -- wt_til_rle` pass (12 tests green).

---

### Step 4: Viewport — migrate + extend (14h)

**Goal:** Migrate Viewport from `core::screen` to `types::viewport`, add WT walk/clamp/convert APIs.

**WT anchors:**
- `types/inc/viewport.hpp` — `Microsoft::Console::Types::Viewport` (52 methods)
- `types/viewport.cpp` — implementations
- **Existing:** `crates/core/src/screen.rs` (170 lines, basic Viewport already implemented in P01)

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_types_viewport.rs` — port WT ViewportTests (walk, clamp, ToOrigin, ToInclusive, ToExclusive, Dimensions, WalkInBounds, WalkInBoundsCircular, ConvertToOrigin, ConvertFromOrigin, TrimToViewport, GetWalkOrigin, etc.)
   - Run `cargo test -p types -- wt_types_viewport` → fail

2. **Implement (green):**
   - Copy `crates/core/src/screen.rs` → `crates/types/src/viewport.rs`
   - Extend Viewport:
     ```rust
     impl Viewport {
         // Existing: new, empty, from_size, width, height, is_empty, intersect, union, contains, offset
         
         // Add WT methods:
         pub fn to_origin(&self) -> Self;  // Translate to (0,0)
         pub fn to_inclusive_rect(&self) -> InclusiveRect;
         pub fn to_exclusive_rect(&self) -> Rect;
         pub fn dimensions(&self) -> Size;
         pub fn clamp_coord(&self, x: i32, y: i32) -> (i32, i32);
         pub fn walk_in_bounds(&self, x: i32, y: i32, delta_x: i32, delta_y: i32) -> Option<(i32, i32)>;
         pub fn walk_in_bounds_circular(&self, x: i32, y: i32, delta_x: i32, delta_y: i32) -> (i32, i32);
         pub fn trim_to_viewport(&self, rect: &Rect) -> Rect;
         // Port all 52 methods from viewport.hpp as needed by tests
     }
     ```
   - Re-export: `pub use viewport::Viewport;`

3. **Run tests:** `cargo test -p types -- wt_types_viewport` → pass

4. **Update core imports (P08 will fully migrate):**
   - `core/src/screen.rs` → add note: `// TODO(P08): migrate to types::Viewport`
   - Do NOT delete `core/src/screen.rs` yet — P08 owns that migration

5. **Update ledger:** Mark `types/inc/viewport.hpp`, `types/viewport.cpp` as ported

**Files:**
- Create: `types/src/viewport.rs` (copy + extend from core::screen), `types/tests/wt_types_viewport.rs`
- Modify: `types/src/lib.rs`, `core/src/screen.rs` (add TODO comment), `parity/ledger/P05-types-til/files.csv` (2 rows)

**Success:** `cargo test -p types -- wt_types_viewport` pass (30+ tests green); core still compiles.

---

### Step 5: Color — ColorFix Oklab port (12h)

**Goal:** Port ColorFix (Oklab color correction per ADR-0015) and color table helpers.

**WT anchors:**
- `types/inc/ColorFix.hpp`, `types/ColorFix.cpp` — Oklab color space conversions
- `types/inc/colorTable.hpp`, `types/colorTable.cpp` — Campbell palette, 256-color table
- `til/ut_til/ColorTests.cpp` — 8 test methods
- **ADR-0015:** ColorFix Oklab port decision (already exists)

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_til_color.rs` — port ColorTests
   - Run `cargo test -p types -- wt_til_color` → fail

2. **Implement (green):**
   - `crates/types/src/color.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, PartialEq)]
     pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
     
     impl Color {
         pub fn from_rgb(r: u8, g: u8, b: u8) -> Self;
         pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self;
         
         // ColorFix: Oklab color space conversions (port from ColorFix.cpp)
         pub fn fix_contrast(fg: Color, bg: Color, min_contrast: f32) -> Color;
         fn to_oklab(&self) -> Oklab;
         fn from_oklab(oklab: Oklab) -> Self;
     }
     
     struct Oklab { l: f32, a: f32, b: f32 }
     
     // Color table helpers (campbell_color_table already in settings::color)
     // Re-export from settings or duplicate minimally if types cannot dep on settings
     ```
   - Note: `campbell_color_table()` and `xterm256()` already exist in `settings::color` (see `core/src/color_table.rs:7`). types crate CANNOT depend on settings (circular). Options:
     - **Option A:** Duplicate minimal color table in types (DRY violation but no circular dep)
     - **Option B:** Move color tables from settings to types (settings depends on types)
     - **Recommended:** Option B — move color tables to types, settings re-exports

3. **Move color tables (if Option B):**
   - Copy `settings/src/color.rs` tables → `types/src/color.rs`
   - `settings/src/color.rs` → `pub use types::color::{Color, campbell_color_table, xterm256};`
   - Update `core/src/color_table.rs:7` → import from types

4. **Run tests:** `cargo test -p types -- wt_til_color` → pass

5. **Update ledger:** Mark ColorFix.*, colorTable.*, ColorTests as ported

**Files:**
- Create: `types/src/color.rs`, `types/tests/wt_til_color.rs`
- Modify: `types/src/lib.rs`, `settings/src/color.rs` (move tables to types), `core/src/color_table.rs` (update import), `parity/ledger/P05-types-til/files.csv` (5 rows)

**Success:** `cargo test -p types -- wt_til_color` pass; core/settings still compile.

---

### Step 6: Width — CodepointWidthDetector (16h)

**Goal:** Port Unicode width detection with EAW/emoji/combining tables from WT.

**WT anchors:**
- `types/inc/CodepointWidthDetector.hpp`, `types/CodepointWidthDetector.cpp` — width tables
- `types/inc/GlyphWidth.hpp`, `types/GlyphWidth.cpp` — glyph width helpers
- `types/ut_types/CodepointWidthDetectorTests.cpp` — width tests (CJK, emoji, combining)

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_types_width.rs` — port CodepointWidthDetectorTests
   - Run `cargo test -p types -- wt_types_width` → fail

2. **Implement (green):**
   - `crates/types/src/width.rs`:
     ```rust
     pub struct CodepointWidthDetector {
         // Unicode EAW tables, emoji ranges, combining marks
     }
     
     impl CodepointWidthDetector {
         pub fn new() -> Self;
         pub fn width(&self, codepoint: u32) -> u8;  // 0 (combining), 1, 2 (CJK/emoji)
         pub fn is_wide(&self, codepoint: u32) -> bool;
         pub fn is_combining(&self, codepoint: u32) -> bool;
     }
     
     // Port tables from CodepointWidthDetector.cpp:
     // - EAW (East Asian Width) ranges
     // - Emoji presentation sequences
     // - Combining marks (U+0300-U+036F, etc.)
     // Use static arrays or const fn initialization
     ```
   - **Data source:** Extract width tables from WT CodepointWidthDetector.cpp — these are Unicode 14.0 ranges. Copy verbatim as const arrays, add source comment.

3. **Run tests:** `cargo test -p types -- wt_types_width` → pass

4. **Update ledger:** Mark CodepointWidthDetector.*, GlyphWidth.*, CodepointWidthDetectorTests as ported

**Files:**
- Create: `types/src/width.rs`, `types/tests/wt_types_width.rs`
- Modify: `types/src/lib.rs`, `parity/ledger/P05-types-til/files.csv` (5 rows)

**Success:** `cargo test -p types -- wt_types_width` pass (20+ tests green); CJK/emoji widths match WT.

---

### Step 7: Unicode + Convert helpers (10h)

**Goal:** Port til::unicode helpers and UTF-8/16 conversion.

**WT anchors:**
- `til/ut_til/UnicodeTests.cpp` — 6 test methods
- `til/ut_til/u8u16convertTests.cpp` — UTF-8/16 conversion tests
- `types/inc/convert.hpp`, `types/convert.cpp` — UTF conversions

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_til_unicode.rs` — UnicodeTests
   - `crates/types/tests/wt_types_convert.rs` — u8u16convertTests
   - Run `cargo test -p types` → fail

2. **Implement (green):**
   - `crates/types/src/unicode.rs`:
     ```rust
     pub fn is_combining_mark(codepoint: u32) -> bool;
     pub fn grapheme_len(s: &str) -> usize;  // Use unicode-segmentation crate or port WT logic
     ```
   - `crates/types/src/convert.rs`:
     ```rust
     pub fn utf8_to_utf16(s: &str) -> Vec<u16>;
     pub fn utf16_to_utf8(s: &[u16]) -> String;
     // Note: Rust std has encode_utf16/from_utf16 — may use std, add thin wrapper if WT semantics differ
     ```
   - **Decision:** If Rust std::char::encode_utf16 matches WT semantics → use std, mark WT convert.cpp as `replaced(ADR-0004)`. If WT has custom error handling (replacement chars, etc.) → port.

3. **Run tests:** `cargo test -p types -- wt_til_unicode wt_types_convert` → pass

4. **Update ledger:** Mark unicode/convert files as ported or replaced

**Files:**
- Create: `types/src/unicode.rs`, `types/src/convert.rs`, `types/tests/wt_til_unicode.rs`, `types/tests/wt_types_convert.rs`
- Modify: `types/src/lib.rs`, `types/Cargo.toml` (add unicode-segmentation if needed), `parity/ledger/P05-types-til/files.csv` (5 rows)

**Success:** `cargo test -p types -- wt_til_unicode wt_types_convert` pass.

---

### Step 8: Utils + EnumSet (8h)

**Goal:** Port utils.cpp helpers (GUID parse, console) and til::enumset.

**WT anchors:**
- `types/inc/utils.hpp`, `types/utils.cpp` — GUID parse, color parsing, console helpers
- `types/ut_types/UtilsTests.cpp`, `types/ut_types/UuidTests.cpp` — utils tests
- `til/ut_til/EnumSetTests.cpp` — enumset tests
- `til/enumset.h` — til::enumset (bitflag set for enums)

**Implementation:**

1. **Port tests (red):**
   - `crates/types/tests/wt_types_utils.rs` — UtilsTests, UuidTests
   - `crates/types/tests/wt_til_enumset.rs` — EnumSetTests
   - Run `cargo test -p types` → fail

2. **Implement (green):**
   - `crates/types/src/utils.rs`:
     ```rust
     #[cfg(windows)]
     pub mod windows_utils {
         pub fn parse_guid(s: &str) -> Result<[u8; 16], ParseError>;
         pub fn parse_color_string(s: &str) -> Result<Color, ParseError>;
         // Console helpers behind cfg(windows)
     }
     ```
   - `crates/types/src/enumset.rs`:
     ```rust
     // Option 1: Use bitflags crate (mark til::enumset as replaced)
     // Option 2: Port til::enumset manually if semantics differ
     
     // Recommended: use bitflags, mark as replaced(ADR-0004)
     pub use bitflags;  // Re-export for consumers
     ```
   - **Decision:** If bitflags crate provides equivalent functionality → use it, mark til::enumset as `replaced(ADR-0004)`. Otherwise port.

3. **Run tests:** `cargo test -p types -- wt_types_utils wt_til_enumset` → pass

4. **Update ledger:** Mark utils/enumset files as ported or replaced

**Files:**
- Create: `types/src/utils.rs`, `types/src/enumset.rs`, `types/tests/wt_types_utils.rs`, `types/tests/wt_til_enumset.rs`
- Modify: `types/src/lib.rs`, `types/Cargo.toml` (add bitflags if used), `parity/ledger/P05-types-til/files.csv` (6 rows)

**Success:** `cargo test -p types -- wt_types_utils wt_til_enumset` pass.

---

### Step 9: Out-of-scope triage (3h)

**Goal:** Mark UIA/precomp files as out-of-scope for P15.

**Files to mark:**
- `types/ScreenInfoUiaProviderBase.*`
- `types/TermControlUiaProvider.*`
- `types/TermControlUiaTextRange.*`
- `types/UiaTextRangeBase.*`
- `types/UiaTracing.*`
- `types/IUiaEventDispatcher.h`
- `types/IUiaTraceable.h`
- `types/IControlAccessibilityInfo.h`
- `types/IInputEvent.hpp` (if unused by P08/P10)
- `types/sgrStack.*` (check if used by core — if yes, port; if no, defer)
- `types/ThemeUtils.*` (check if used by P10 — if yes, port; if no, defer)
- `types/User32Utils.hpp` (cfg(windows) low priority)
- `types/precomp.*` (C++ precompiled headers — not applicable to Rust)
- `til/ut_til/BaseTests.cpp` (til::base — check if just allocation helpers → replaced)
- `til/ut_til/CoalesceTests.cpp` (til::coalesce — check usage)
- `til/ut_til/EnvTests.cpp` (til::env — std::env equivalent → replaced)
- `til/ut_til/FlatSetTests.cpp` (til::flat_set — std::collections → replaced)
- `til/ut_til/GenerationalTests.cpp` (til::generational — check if arena pattern → port or defer)
- `til/ut_til/HashTests.cpp` (til::hash — std::hash → replaced)
- `til/ut_til/mutex.cpp` (til::mutex — std::sync → replaced)
- `til/ut_til/ReplaceTests.cpp` (til::replace — check usage)
- `til/ut_til/SmallVectorTests.cpp` (til::small_vector — Vec or smallvec crate → replaced)
- `til/ut_til/SPSCTests.cpp` (til::spsc — crossbeam channel → replaced)
- `til/ut_til/StaticMapTests.cpp` (til::static_map — HashMap → replaced)
- `til/ut_til/string.cpp` (til::string — String → replaced)
- `til/ut_til/throttled_func.cpp` (til::throttled_func — check usage)

**Implementation:**

1. Review each file in ledger against:
   - Used by P08 (buffer)? → port
   - Used by P10 (renderer)? → port
   - Used by P07 (settings)? → port
   - UIA-related? → out-of-scope (P15), mark `adr: ADR-0009`
   - Precomp? → out-of-scope (not applicable), mark `notes: C++ precomp, N/A Rust`
   - Rust std equivalent? → replaced, mark `adr: ADR-0004`, `notes: std::*`

2. Update ledger: set `status: out-of-scope` or `status: replaced`, fill `adr` and `notes` columns

3. Output: ~30 rows marked out-of-scope or replaced, leaving ~37 ported

**Files:**
- Modify: `parity/ledger/P05-types-til/files.csv` (~30 rows)
- Modify: `docs/decisions/0004-til-types-replacement.md` (add replaced items to table)

**Success:** Every P05 ledger row has status ∈ {ported, replaced, out-of-scope}.

---

### Step 10: Full test suite + coverage (7h)

**Goal:** Run full types test suite, verify 198 WT tests ported, measure coverage ≥90%/75%.

**Implementation:**

1. **Run all tests:**
   - `cargo test -p types` → expect ~150+ tests passing (ported from 198 WT tests minus replaced/out-of-scope)
   - Paste full test output

2. **Measure coverage:**
   - `cargo xtask cov --phase 05` → diff coverage on types crate
   - Verify: line ≥90%, branch ≥75%
   - Paste coverage report

3. **Fix gaps:**
   - If coverage < threshold: identify uncovered branches, add test cases
   - If tests fail: debug against WT test vectors

4. **Ledger verification:**
   - `cargo xtask ledger-check` → verify all P05 rows resolved
   - `cargo xtask tests-check` → verify test count matches ledger

**Files:**
- None (verification only)

**Success:** `cargo test -p types` pass (150+ tests green), coverage ≥90%/75%, ledger check pass.

---

## Deliverables

1. **Crate:** `crates/types` with 11 modules (point, size, rect, viewport, rle, color, width, unicode, convert, utils, enumset)
2. **Tests:** 11 test files in `crates/types/tests/` covering 150+ WT test cases
3. **Ledger:** All 67 P05 rows resolved (37 ported, 30 replaced/out-of-scope)
4. **ADR:** ADR-0004 finalized with replacement map
5. **Coverage:** ≥90% line, ≥75% branch on types crate diff

## Dependencies

- **Crates:** bitflags (if enumset replaced), unicode-segmentation (if unicode needs it)
- **Consumers:** P08 (buffer) will migrate from `core::screen::Viewport` to `types::Viewport`
- **Consumers:** P10 (renderer) will use `types::CodepointWidthDetector`, `types::Color`
- **Consumers:** P07 (settings) will re-export `types::Color` for color tables

## Risks

1. **Width table drift:** WT uses Unicode 14.0 tables; current Unicode is 15.1. Mitigation: port WT tables verbatim, note version in comment.
2. **Viewport API surface:** WT Viewport has 52 methods; only port methods used by P08/P10. Mitigation: start with test-required methods, expand on-demand.
3. **Circular deps:** types cannot depend on settings (color tables). Mitigation: move color tables to types (Step 5).
4. **Oklab complexity:** ColorFix port is math-heavy. Mitigation: port WT formulas 1:1, validate against WT test vectors.

## Success Criteria

- `cargo test -p types` pass (150+ tests)
- `cargo xtask ledger-check` pass (all P05 rows resolved)
- `cargo xtask cov --phase 05` ≥90%/75%
- ADR-0004 finalized
- `cargo check --workspace` pass (no regressions in core/settings)

## Unresolved Questions

1. **Color table ownership:** Confirm moving color tables from settings to types (breaks settings API if consumers import from settings::color). Alternative: duplicate in types, settings re-exports.
2. **EnumSet:** Use bitflags crate or port til::enumset? Need to check if til::enumset has custom iteration/masking logic beyond bitflags.
3. **Unicode segmentation:** Use unicode-segmentation crate or port WT grapheme logic? Check if WT has custom combining-mark handling.
4. **sgrStack/ThemeUtils:** Defer to P08/P10 or port in P05? Check if used by buffer/renderer.
