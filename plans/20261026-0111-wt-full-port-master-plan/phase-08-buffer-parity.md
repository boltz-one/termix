# Phase 08 — TextBuffer / Row / attributes parity (detailed plan)

## Context

Wave 2 của master port plan. Đưa `core/src/buffer.rs` (1,334 LOC skeleton — hiện 1,111 theo brief, thực tế 1,334) lên parity với WT `buffer/out/` textBuffer family (`textBuffer.cpp` 3,560 + `Row.cpp` 1,288 + `Marks.hpp` 91 + `ImageSlice.cpp` 271 + `search.cpp` 153 + iterator files ~401 = ~5,764 LOC). ADR (per master plan): mirror **WT ROW layout SEMANTICS** (char offsets, DBCS lead/trailer, grapheme width) — KHÔNG cần mirror memory layout (`charsBuffer`/`charOffsetsBuffer` SIMD union). Lý do: 14+ WT tests (`TextBufferTests.cpp`) phụ thuộc contract char-offset/width, không phải raw buffer bytes.

P08 alone trong Wave 2; P09 (selection/control) và P10 (renderer paint) wait trên P08 (per master plan).

## WT source map (file:line anchors)

Quét `windows/src/buffer/out/` + `windows/src/host/ut_host/`:

**textBuffer.cpp** (3,560 LOC):
- `:539` `TextBuffer::Replace`
- `:548` `TextBuffer::Insert`
- `:583` `TextBuffer::FillRect`
- `:728` `TextBuffer::IncrementCircularBuffer`
- `:760` `TextBuffer::GetLastNonSpaceCharacter`
- `:806` `TextBuffer::ScrollRows`
- `:876` `TextBuffer::CopyRow`
- `:904` `TextBuffer::SetCurrentAttributes`
- `:914` `TextBuffer::SetCurrentLineRendition`
- `:938` `TextBuffer::ResetLineRenditionRange`
- `:960` `TextBuffer::GetLineWidth`
- `:967` `TextBuffer::ClampPositionWithinLine`
- `:999` `TextBuffer::ClearScrollback`
- `:1038` `TextBuffer::ResizeTraditional`
- `:1097` `TextBuffer::TriggerRedraw`
- `:1121` `TextBuffer::TriggerScroll(delta)`
- `:1172` `TextBuffer::GetWordStart(pos, wordDelimiters, includeWhitespace, limitOptional)`
- `:1235` `TextBuffer::GetWordEnd(...)`  (mirror GetWordStart)
- `:1675` `TextBuffer::GetTextRects(start, end, blockSelection, bufferCoordinates)` + internal `_ExpandTextRow` helper (`:1789`)
- `:1734` `TextBuffer::GetTextSpans`
- `:1821` `TextBuffer::GetPlainText(start, end)`
- `:1887` `TextBuffer::GetPlainText(CopyRequest)` (rtf/escape path)
- `:2718` `TextBuffer::Reflow(oldBuffer, newBuffer, lastCharacterViewport, positionInfo)` (~280 LOC core reflow logic)

**Row.cpp** (1,288 LOC):
- `:177` `ROW::SetWrapForced`
- `:211` `ROW::GetReadableColumnCount`
- `:226` `ROW::Reset(attr)` — đã port
- `:356` `ROW::CopyFrom(source)` — đã port `Row::copy_from`
- `:373` `ROW::NavigateToPrevious`, `:380` `::NavigateToNext`, `:390` `::AdjustToGlyphStart`, `:402` `::AdjustToGlyphEnd` (glyph-aware cursor navigation — MISSING)
- `:413` `ROW::ClearCell(column)`
- `:542` `ROW::SetAttrToEnd`
- `:547` `ROW::ReplaceAttributes` — đã port
- `:572` `ROW::ReplaceCharacters(columnBegin, width, chars)`
- `:613` `ROW::ReplaceText(RowWriteState&)` — đã port width-aware
- `:765` `ROW::CopyTextFrom(RowCopyTextFromState&)`
- `:950` `ROW::Attributes()` / `:955` `ROW::GetAttrByColumn` — đã port qua `attr_at`
- `:979` `ROW::GetImageSlice` / `SetImageSlice` — MISSING
- `:1002` `ROW::GetLastNonSpaceColumn`
- `:1027` `ROW::MeasureLeft` — MISSING (chỉ có `measure_right`)
- `:1047` `ROW::MeasureRight` — đã port
- `:1116` `ROW::GetText() / GetText(begin, end)` — MISSING
- `:1136/1141` `ROW::GetLeadingColumnAtCharOffset` / `GetTrailingColumnAtCharOffset` — MISSING (char-offset API)
- `:1248` `ROW::GetScrollbarData` — MISSING

**Marks.hpp** (91 LOC): `MarkCategory` enum (`:21`), `ScrollbarData` (`:35`), `MarkExtents` (`:56`), `ScrollMark` (`:87`) — MISSING (anchor-row / shell-integration support).

**LineRendition.hpp** (`:16`, 4 variants): `SingleWidth`, `DoubleWidth`, `DoubleHeightTop`, `DoubleHeightBottom`. ✅ Đã có ở `core/src/attrs.rs:421` 1:1 variant order; helpers `screen_to_buffer_line`/`buffer_to_screen_line` cũng đã port (`attrs.rs:442+`).

**search.cpp** (153 LOC) + **search.h**: `SearchFlag{None, CaseInsensitive, RegularExpression}` (`search.h:31`), `Search::Reset` (`search.cpp:19`), `::FindNext` (`:96`), `::MoveToPoint`/`::MovePastPoint` — port riêng ở `control/src/search.rs` (163 LOC, cần verify against brief).

**textBufferCellIterator.cpp** (358 LOC) + **textBufferTextIterator.cpp** (43 LOC) — iterator family, port minimal at Phase 4.0-pre.

**ImageSlice.cpp** (271 LOC) — sixel; stub OK (P10 land real impl).

## Current Rust port status

**`crates/core/src/buffer.rs`** (1,334 LOC, line count verified, NOT brief's 1,111):
- `Row` struct (`:36`): `attr: TextAttribute`, `attr_runs: Vec<(u16, TextAttribute)>` (RLE-inline, NOT `types::rle`), `line_rendition`, `glyphs: SmallVec<[(char, u8);128]>`, `column_count: u16`, `wrap_forced`, `double_byte_padded`, `text_cleared` — ĐỦ vạt cơ bản.
- `Row` methods: `new`, `reset` (port `:226` Reset), `measure_right`, `contains_text`, `glyph_at`, `replace_text` (width-aware `:613` port), `replace_attributes`, `attr_at`, `set_attr_range`, `fill_attr_range`, `copy_attr_range_from`, `copy_from`, `line_rendition/get/set`, `is_double_width`.
- TextBuffer struct (`:343`): `rows: Vec<Row>`, `size: (u16,u16)`, `cursor`, `current_attr`, `scrollback: VecDeque<Row>`, `scrollback_limit`, `alt_main`, `scrollback_dropped`.
- TextBuffer methods (`:362+`): `new`, `row_abs`, `total_rows`, `push_history`, `enter_alt_screen`/`leave_alt_screen`, `clear_history`, `rows`/`rows_mut`, `get_row`/`get_row_mut` (modulo ring), `size`/`get_size`/`get_total_buffer_size`, `get_cursor`/`get_cursor_mut`, `get_cell_data`/`set_text`, `clear_row`, `replace` (verbatim port `:539`), `insert` (verbatim `:548`), `fill_rect` (verbatim `:583`), `increment_circular_buffer` (verbatim `:728`), `resize_traditional` (verbatim `:1038`), `set_current_attributes`/`get_current_attributes`, `set_wrap_forced`, `is_double_width_line`, **Phase 4.0-pre verbatim group** (`:677-956`): `get_line_rendition` `:930`, `get_line_width` `:944`, `scroll_rows` `:806`, `copy_row_in_range` helper, `get_last_non_space_character` `:760`, `measure_right_of` helper, `clamp_position_within_line` `:967`, `reset_line_rendition_range` `:938`, `clear_scrollback` `:999`, `trigger_scroll` (no-op skeleton), `set_current_line_rendition` `:914`, `copy_row`, `reset`, `trigger_redraw`/`trigger_redraw_all`/`notify_paint_frame` (no-op skeleton).
- Tests (`:988-1334`): 29 `#[test]` đã có (new_buffer_inits_blank_rows, get_row_modulo_handles_circular, scroll_rows_*4 variants, get_last_non_space_*3, clamp_position_within_line_caps_x, reset_line_rendition_range_resets, clear_scrollback_*3, trigger_scroll_is_safe_noop, set_current_line_rendition_*2, copy_row_*2, wide_glyph_has_trailer_and_overwrites_cleanly, row_attr_runs_set_and_query, row_fill_and_copy_attrs, scrolled_off_rows_go_to_scrollback_and_trim).

**`crates/core/src/dispatch/buffer_ext.rs`**: KHÔNG TỒN TẠI. Grep `buffer_ext` cho 0 hit trên toàn crates/. Task brief's claim "P03 added `dispatch/buffer_ext.rs`... P08 must FOLD ext into `buffer/**`" không match state của repo. Tương tự "`ResizeTraditional` currently `todo!` upstream at `control/src/core.rs:141`" — line 141 là `pub fn start()` (connection lifecycle), và `resize_dispatch` ở `core.rs:39-48` đã gọi `d.terminal.resize_traditional((cols, rows))` đầy đủ. **Fold-in mục tiêu của brief thu hẹp về "no-op verification"** (xem §8).

**`crates/types/src/lib.rs`** (6 LOC doc comment): empty skeleton. `rle`/`point`/`rect`/`viewport` primitives KHÔNG ở `types`. Project thay thế bằng:
- `Viewport` ở `core/src/screen.rs:9`
- `LineRendition` ở `core/src/attrs.rs:421`
- RLE thuần inline trong `Row::attr_runs` (Vec of runs) — không có `types::rle`.

→ P08 GIỮ trạng thái hiện tại (không migrate sang types); brief's "verify `types::rle` exists; if not, P05 must land first" → **PASS: không cần P05, vì RLE đã inline**.

**`crates/control/src/search.rs`** (163 LOC) — search layer đã có nhưng chưa verify content parity với `search.cpp` 153 LOC.待 P08 confirm.

**`crates/core/src/dispatch/`** subfolder: `headless_host.rs` 296, `interact.rs` 348, `interact_impl.rs` 384, `mod.rs` 1106, `seams.rs`, `types.rs` 517 — plus `dispatch/td/` 13 files (~4.5k LOC verbatim TD). KHÔNG có `buffer_ext.rs`, không có `mod buffer_ext;` line để remove.

## Gaps (concrete WT anchor + Rust gap + target file)

| # | Brief gap | WT anchor | Rust gap | Target Rust file |
|---|-----------|-----------|---------|------------------|
| 1 | Row/ROW char-offset layout (DBCS lead/trailer, grapheme clusters, `GetLeadingColumnAtCharOffset`/`GetTrailingColumnAtCharOffset`) | `Row.cpp:1136-1141`, `Row.hpp:77+`(CharToColumnMapper) | `Row` chỉ có `glyphs: SmallVec<[(char,u8)]>` + `glyph_at` — thiếu `char_offsets` index, `CharToColumnMapper`-equivalent, `*_column_at_char_offset` | `core/src/buffer.rs` Row::新增 `char_offset_at_char_offset`, `get_leading_column_at_char_offset`, `get_trailing_column_at_char_offset` |
| 1a | Row navigation (`NavigateToPrevious`, `NavigateToNext`, `AdjustToGlyphStart`, `AdjustToGlyphEnd`) | `Row.cpp:373,380,390,402` | MISSING tất cả 4 | `core/src/buffer.rs` Row impl block |
| 1b | `MeasureLeft`, `GetLastNonSpaceColumn` | `Row.cpp:1027,1002` | Chỉ có `measure_right` (`buffer.rs:100`); thiếu `measure_left` & `get_last_non_space_column` | `core/src/buffer.rs` Row impl block |
| 1c | `ReplaceCharacters`, `ClearCell`, `SetAttrToEnd`, `CopyTextFrom` | `Row.cpp:572,413,542,765` | MISSING tất cả | `core/src/buffer.rs` Row impl |
| 1d | `ROW::GetText()` / `GetText(begin,end)` | `Row.cpp:1116,1125` | MISSING — Row chỉ expose `glyphs()` slice, không có text-slice API | `core/src/buffer.rs` Row impl — thêm `row_text(&self, begin, end) -> String` |
| 2 | Attribute RLE on `types::rle` | (n/a — brief want) | `types::rle` empty; Row has inline `attr_runs: Vec<(u16, TextAttribute)>` đã làm RLE-via-runs | **No-op**: giữ inline. OPT: extract thành `pub mod rle;` trong core (ko bắt buộc) |
| 3 | Reflow (`TextBuffer::Reflow`) — `ResizeTraditional` currently `todo!` | `textBuffer.cpp:2718-3000` (~280 LOC) | `resize_traditional` ĐÃ port (`buffer.rs:623-651`) — KHÔNG còn todo; **`Reflow` chưa port** | `core/src/buffer.rs` TextBuffer::新增 `pub fn reflow(&mut old, &mut new, last_char_vp: Option<&Viewport>, pos_info: Option<&mut PositionInformation>)` — split thành module `core/src/buffer/reflow.rs` nếu >200 LOC |
| 4 | Scrollback ring + `ScrollRows` parity | `textBuffer.cpp:806-873` ScrollRows + `:728` IncrementCircularBuffer | `scroll_rows` (`:719`) và `increment_circular_buffer` (`:608`) đã port verbatim — thiếu `PositionInformation` struct (Reflow dùng) + `_firstRow` circular trong memory (bỏ qua vì giữ semantics-only) | `core/src/buffer.rs` — thêm `pub struct PositionInformation { pub mutable_viewport_top: i32, pub visible_viewport_top: i32 }` (port `textBuffer.hpp` struct) |
| 5 | `ClearScrollback` | `textBuffer.cpp:999-1035` | `clear_scrollback(new_first_row, rows_to_keep)` ĐÃ port verbatim (`:846-878`) | **No-op** — verify against `textBuffer.cpp:999` |
| 6 | Hyperlinks (link URI → Row field) | `ROW::GetHyperlinks` (`Row.hpp`), `TextBuffer::GetUrl` (search in `textBuffer.cpp:3340`+), `TextAttribute::_hyperlinkId` | `TextAttribute` đã có `hyperlink_id: u16` (`attrs.rs:92`); thiếu `Row::get_hyperlinks() -> Vec<u16>` + `TextBuffer` hyperlink registry (hash `u16 -> String`) | `core/src/buffer.rs` Row::新增 `get_hyperlinks` + `TextBuffer::新增 `hyperlinks: HashMap<u16, String>` + `add_hyperlink(uri) -> u16` + `get_hyperlink(id) -> Option<&str>` |
| 7 | Marks / shell-integration (anchor rows) | `Marks.hpp:21-91` (`MarkCategory`, `MarkExtents`, `ScrollMark`), `TextBuffer::MoveMark`/`AddMark`/`GetMarkRanges` | `TextAttribute::mark_kind: MarkKind` ĐÃ có (`attrs.rs:93`); thiếu `Marks.hpp` struct + `TextBuffer::mark_ranges` storage | `core/src/buffer.rs` 新增 `pub mod marks` (or struct trong same file): `MarkCategory`, `MarkExtents`, `ScrollMark`, `ScrollbarData` + `TextBuffer::scroll_marks: Vec<ScrollMark>` + `add_mark`/`move_mark`/`get_mark_ranges` |
| 8 | Image slices (sixel) | `ImageSlice.cpp` 271 LOC, `Row.cpp:979 GetImageSlice/SetImageSlice` | MISSING hoàn toàn | `core/src/buffer.rs` 新增 `pub mod image_slice` minimal stub: `pub struct ImageSlice { pub data: Vec<u8> /* placeholder */, pub width: u16, pub height: u16 }` + `Row::image_slice: Option<Box<ImageSlice>>` + `Row::get_image_slice`/`set_image_slice` accessors. Real resize/clone waits P10. |
| 9 | LineRendition enum | `LineRendition.hpp:16` 4 variants | **ĐÃ PORT 1:1** (`attrs.rs:421` — `SingleWidth=0,DoubleWidth=1,DoubleHeightTop=2,DoubleHeightBottom=3`) | **No-op** |
| 10 | Search (`search.cpp`) + text iterators + word-boundary + URL pattern | `search.cpp:19 Reset`, `:96 FindNext`; `textBuffer.cpp:1172 GetWordStart`, `:1235 GetWordEnd`; `textBufferCellIterator.cpp` 358 LOC; `textBufferTextIterator.cpp` 43 LOC | `control/src/search.rs` 163 LOC đã có skeleton; `TextBuffer::get_word_start`/`get_word_end` MISSING; `Row::delimiter_class_at` MISSING | `core/src/buffer.rs` TextBuffer::新增 `get_word_start` / `get_word_end` (port verbatim `:1172`/`:1235`); Row::新增 `delimiter_class_at(col, delimiters)` (`Row.cpp` DelimiterClassAt); module `core/src/buffer/iter.rs` cho `TextBufferCellIterator`/`TextBufferTextIterator` |
| 11 | `GetText` / `GetWordStart` / `GetWordEnd` / `ExpandWideGlyphSelection` helpers | `Row.cpp:1116/1125 GetText`; `textBuffer.cpp:1172/1235` word; `textBuffer.cpp:1675 GetTextRects`; `:1734 GetTextSpans`; `:1821 GetPlainText`; `_ExpandTextRow` (`:1789` internal) | MISSING: `Row::get_text`, `TextBuffer::get_text_rects`, `get_text_spans`, `get_plain_text`. NOTE: `ExpandWideGlyphSelection` does NOT exist as public symbol in WT — equivalent is internal `_ExpandTextRow` (`:1789`). | `core/src/buffer.rs` Row + TextBuffer impl; optional `core/src/buffer/text_ops.rs` module nếu >200 LOC |
| 12 | Fold P03 `dispatch/buffer_ext.rs` into `buffer/**` | (n/a) | `dispatch/buffer_ext.rs` KHÔNG TỒN TẠI trong repo (verified via `grep -rn buffer_ext crates/` = 0 hit). Tương tự `mod buffer_ext;` không có trong `dispatch/mod.rs`. | **No-op** — không có gì fold. Mark gap closed. |

## Worker partition

**Single-worker** (per brief "Wave 2 alone; P09/P10 wait on it"). Phân chia nội bộ thành 4 sub-wave USER-C sequential, ko parallel:

- **W0 — Verify/no-op** (gap 2, 5, 9, 12): wc confirm, write `// P08 クローズマーク` aần kết luận trong PR description. Estimate 0.5 giờ.
- **W1 — Row parity** (gap 1, 1a, 1b, 1c, 1d, hyperlink #6 partial-row, image slice #8): sửa `core/src/buffer.rs` Row block + (optional) tách `core/src/buffer/mod.rs` + `row.rs` + `marks.rs` + `image_slice.rs` nếu buffer.rs vượt 200 LOC/file (already 1,334 — vượt KISS 200 → phải split). Ước tính ~800 LOC port.
- **W2 — TextBuffer parity** (gap 3 reflow, 4 PositionInformation, 10 search + iter, 11 text op): port verbatim `Reflow` (280 LOC), `GetWordStart/End` (~120 LOC), `GetTextRects/GetPlainText` (~250 LOC), `PositionInformation` struct. Ước tính ~700 LOC.
- **W3 — Hyperlink + Marks wire-up** (gap 6 TextBuffer-side, 7): `hyperlinks: HashMap<u16,String>` registry + marks storage + `add_hyperlink`/`add_mark` API. Ước ~200 LOC.
- **W4 — Test gate** (§9): port 14 red tests từ `TextBufferTests.cpp` vào `buffer.rs` `mod tests` block (hoặc split `tests/buffer_parity.rs`). Ước ~600 LOC test.

**File ownership**: single worker owns `crates/core/src/buffer.rs` (+sponsor tách sub-module nếu >200 LOC/file). Phải split vì 1,334 LOC đã việt 200-LOC KISS rule:
- `core/src/buffer/mod.rs` (Re-export + TextBuffer struct + impl core methods, <200 LOC)
- `core/src/buffer/row.rs` (Row struct + impl, <200 LOC)
- `core/src/buffer/text_ops.rs` (GetText/GetWordStart/GetWordEnd/GetTextRects, <200 LOC)
- `core/src/buffer/reflow.rs` (Reflow, <200 LOC)
- `core/src/buffer/marks.rs` (Marks struct + impl, <200 LOC)
- `core/src/buffer/image_slice.rs` (ImageSlice stub, <50 LOC)
- `core/src/buffer/iter.rs` (CellIterator + TextIterator, <200 LOC)
- `core/src/buffer/tests.rs` (mock+tests, <200 LOC/file)

## Implementation steps per gap

Gap-by-gap, file:line anchor WT ⇄ Rust target:

**W1 Row**:
1. Tách `core/src/buffer.rs` → `core/src/buffer/mod.rs` + sub-files (DRY:255a KISS 200). `lib.rs` đổi `pub mod buffer;` để giữ `core::buffer::TextBuffer` path stable.
2. `buffer/row.rs` Row impl block 新增 (port từ `Row.cpp`):
   - `measure_left` ← `Row.cpp:1027` (mirror `measure_right`)
   - `get_last_non_space_column` ← `Row.cpp:1002`
   - `navigate_to_previous(col)` ← `Row.cpp:373`; `navigate_to_next(col)` ← `Row.cpp:380`
   - `adjust_to_glyph_start(col)` ← `Row.cpp:390`; `adjust_to_glyph_end(col)` ← `Row.cpp:402`
   - `clear_cell(col)` ← `Row.cpp:413`
   - `set_attr_to_end(col_begin, attr)` ← `Row.cpp:542`
   - `replace_characters(col_begin, width, chars)` ← `Row.cpp:572`
   - `copy_text_from(state)` ← `Row.cpp:765`
   - `row_text()` / `row_text(begin, end)` → String ← `Row.cpp:1116,1125`
   - `get_leading_column_at_char_offset(offset)` / `get_trailing_column_at_char_offset(offset)` ← `Row.cpp:1136-1141` (uses `char_offsets: Vec<u16>` — add field to Row).
   - `get_char_offset(col)` ← `Row.cpp` counterpart
   - `delimiter_class_at(col, delimiters)` ← `Row.cpp` (port + add `DelimiterClass` enum upstream)
   - `get_hyperlinks() -> Vec<u16>` (crawls `attr_runs` collecting `hyperlink_id != 0`)
   - `image_slice: Option<Box<ImageSlice>>` + `get_image_slice`/`set_image_slice`/`get_mutable_image_slice` ← `Row.cpp:979`
3. `buffer/image_slice.rs` (stub): `pub struct ImageSlice { pub data: Vec<u8>, pub width: u16, pub height: u16 }` + `Clone`. Sixel decode waits P10.

**W2 TextBuffer**:
1. `buffer/reflow.rs`: `pub fn reflow(old: &mut TextBuffer, new: &mut TextBuffer, last_char_vp: Option<&Viewport>, pos_info: Option<&mut PositionInformation>)` ← `textBuffer.cpp:2718` verbatim. Inner loop considers `LineRendition != SingleWidth` → newRow.CopyFrom + reset wrap. Brand new row when `newY >= newHeight`. Uses `cursor.AdjustToGlyphStart` (=> `Row::adjust_to_glyph_start`).
2. `buffer/text_ops.rs`:
   - `get_word_start(pos, word_delimiters, include_whitespace, limit)` ← `textBuffer.cpp:1172` verbatim. Uses `Row::delimiter_class_at`.
   - `get_word_end(pos, ...)` ← `textBuffer.cpp:1235` mirror.
   - `get_text_rects(start, end, block_selection, buffer_coords) -> Vec<Viewport>` ← `textBuffer.cpp:1675`. Calls `_expand_text_row` helper.
   - `_expand_text_row(rect)` ← `textBuffer.cpp:1789` internal helper (expands wide glyph rect).
   - `get_text_spans(start, end, block, buffer_coords) -> Vec<(til::point, til::point)>` ← `:1734`
   - `get_plain_text(start, end) -> String` ← `:1821`, calls `row_text` from Row impl.
3. `buffer/iter.rs`:
   - `pub struct TextBufferCellIterator<'a>` (port `textBufferCellIterator.cpp:24`) — yields `(char, u8, &TextAttribute)` per column. Walks `Row::glyphs` + `attr_at`.
   - `pub struct TextBufferTextIterator<'a>` (port `textBufferTextIterator.cpp`) — yields grapheme-aware text spans.
4. `buffer/mod.rs` TextBuffer struct 新增强:
   - `pub struct PositionInformation { pub mutable_viewport_top: i32, pub visible_viewport_top: i32 }`
   - `pub fn reflow(...)` wrapper to `buffer::reflow::reflow`.
   - `pub fn get_word_start/.../get_text_rects/.../get_plain_text` — delegate to `buffer::text_ops`.

**W3 Hyperlinks + Marks**:
- `buffer/marks.rs`: `#[derive(Clone, Copy, Debug, Default) pub enum MarkCategory { None=0, Prompt, Command, Output }` (mirror `MarkKind` — note `MarkCategory` trong `Marks.hpp:21` may differ from `TextAttribute::MarkKind` ở `TextAttribute.hpp:42`; verify naming). `pub struct MarkExtents { start: til::point, end: til::point, category: MarkCategory }` (`Marks.hpp:56`). `pub struct ScrollMark { row: i32, category: MarkCategory, extents: Option<MarkExtents> }` (`Marks.hpp:87`). `pub struct ScrollbarData { ... }` (`Marks.hpp:35`).
- TextBuffer 新增zői `scroll_marks: Vec<ScrollMark>` + methods `add_mark(row, category)`, `move_mark(delta)`, `get_mark_ranges() -> &[ScrollMark]`.
- `buffer/mod.rs` TextBuffer 新增 `hyperlinks: HashMap<u16, String>` + `next_hyperlink_id: u16`, methods `add_hyperlink(uri: &str) -> u16`, `get_hyperlink_uri(id) -> Option<&str>`. Row._VEC_`get_hyperlinks()` collects non-zero `hyperlink_id`s from `attr_runs`.

## Fold-in plan (buffer_ext.rs → buffer/** migration table)

**Verification result**: `dispatch/buffer_ext.rs` và `mod buffer_ext;` line KHÔNG TỒN TẠI trong repo. Brief instruction "fold P03 `dispatch/buffer_ext.rs` methods INTO `buffer/**`" hits empty set.

| Source method (brief assumption) | Target file | Action |
|---|---|---|
| (none — file doesn't exist) | n/a | No-op. Đánh dấu closed trong final report. |

**Adjacent verification** (đề phòng các method mà brief NAÏVE предполаг cần fold — actually live elsewhere):
- P03 verbatim TD dispatch files (`dispatch/td/` 13 files) gọi `TextBuffer` methods qua `&mut self.terminal` (e.g. `erase.rs` calls `replace`/`insert`/`fill_rect`; `scroll.rs` calls `scroll_rows`/`increment_circular_buffer`; `cursor.rs` calls `cursor.set_position`; `print.rs` calls `set_text`). Các method TEXT đã có trong `buffer.rs`. **No migration needed** — TD uses pub API only.
- `dispatch/interact_impl.rs` 384 LOC (host interaction dispatch) có các buffer-touching như `get_current_attributes`, maybe cần verify against P08 changes — done in W3 wire-up (attr field name changes ripple).

**Compat re-exports**: vì không có `buffer_ext` symbol, không cần re-export. PATH `core::buffer::TextBuffer` và `core::buffer::Row` phải giữ sau split. `lib.rs` sửa:
- Trước: `pub mod buffer;` (file `src/buffer.rs`)
- Sau: rename `src/buffer.rs` → `src/buffer/mod.rs` + add `src/buffer/row.rs` etc. Public path `core::buffer::TextBuffer` giữ nguyên.

## Test gate

Port subset của `windows/src/host/ut_host/TextBufferTests.cpp` (30+ TEST_METHOD) vào `core/src/buffer/tests.rs` dưới module `cfg(test) mod tests`. Run namespace: `cargo test -p core -- buffer::tests::` (snake_case; WT names sẽ chuyển `TestBufferCreate` → `test_buffer_create`).

**14 red tests selected** (cover 6 missing-gap areas; không port tất cả 30, chọn 14 đại diện phù hợp brief 14-count offer):

| # | WT source line | WT test name | Rust snake_case | Validates gaps |
|---|---|---|---|---|
| 1 | `TextBufferTests.cpp:68` | `TestBufferCreate` | `test_buffer_create` | baseline |
| 2 | `:76` | `TestWrapFlag` | `test_wrap_flag` | Row wrap_forced parity (gap 1) |
| 3 | `:78` | `TestWrapThroughWriteLine` | `test_wrap_through_write_line` | replace_text + wrap interaction |
| 4 | `:80` | `TestDoubleBytePadFlag` | `test_double_byte_pad_flag` | Row DBCS trailer (gap 1) |
| 5 | `:91` | `TestBoundaryMeasuresRegularString` | `test_boundary_measures_regular_string` | MeasureRight/MeasureLeft (gap 1b) |
| 6 | `:99` | `TestIncrementCircularBuffer` | `test_increment_circular_buffer` | scrollback ring (gap 4) |
| 7 | `:130` | `ResizeTraditional` | `resize_traditional` | size change (gap 3 precondition) |
| 8 | `:133` | `ScrollBufferRotationPreservesHighUnicode` | `scroll_buffer_rotation_preserves_high_unicode` | wide glyph scroll_rows (gap 1, 4) |
| 9 | `:135` | `ResizeTraditionalHighUnicodeRowRemoval` | `resize_traditional_high_unicode_row_removal` | wide glyph resize (gap 3) |
| 10 | `:136` | `ResizeTraditionalHighUnicodeColumnRemoval` | `resize_traditional_high_unicode_column_removal` | wide glyph column shrink (gap 3) |
| 11 | `:145` | `GetWordBoundaries` | `get_word_boundaries` | GetWordStart/End (gap 10, 11) |
| 12 | `:149` | `GetTextRects` | `get_text_rects` | GetTextRects + _ExpandTextRow (gap 11) |
| 13 | `:150` | `GetPlainText` | `get_plain_text` | Row::GetText + GetPlainText (gap 11) |
| 14 | `:155` | `ReflowPromptRegions` | `reflow_prompt_regions` | Reflow + Marks (gap 3, 7) |

Selection criteria: covers all 12 gaps với ít nhất 1 red test each. SKIP non-essential tests (TestMixedRgb* for SGR color — đã cover P05; CopyAttrs/CopyLastAttr — đã cover existing attr_runs tests; appendRTFText — out of scope).

**Red-test discipline**: viết test FIRST (red), rồi port code để chuyển green. Mỗi test fail đầu tiên (`cargo test -p core -- buffer::tests::test_X` fails) → port code → test passes. **No mocks**: real `TextBuffer` instances, real wide-char strings (e.g. `"\u{65E5}x"`, `"\u{672C}"`), real cursors.

## Out of scope

1. `vte::Parser`/dispatch wiring — covered P04.
2. Sixel real decode — waits P10 (only `ImageSlice` stub here).
3. `control/src/search.rs` rewrite — only verify against P08 new `get_word_start`/`get_word_end` APIs.
4. Search.cpp full port (regex flag) — wait P09 if not blocking.
5. `TextBufferCellIterator` full `OutputCell`-typed API — minimal iteration only.
6. Renderer wire-up (TriggerRedraw/TriggerScroll → GPUI paint cycle) — wait P10.
7. RLE migration to `types::rle` — out (verified unnecessary, inline `attr_runs` suffices).
8. `dispatch/buffer_ext.rs` fold-in — out (file doesn't exist; no-op verification).

## Risk assessment

**HIGH**: Reflow (`textBuffer.cpp:2718`) là ~280 LOC logic phức tạp với double-height row truncation, viewport tracking, cursor `AdjustToGlyphStart` calls. Phải port verbatim với REFLOW_RESET comment marker + REFLOW_JANK_CURSOR_WRAP special case (`:2726-2733`). Bug ở reflow có thể corrupt buffer state khi user resize. Mitigation: TDD red test `reflow_prompt_regions` viết ĐẦU TIÊN trong W2, sau đó port.

**MEDIUM**: Char-offset API (gap 1, `GetLeadingColumnAtCharOffset` / `GetTrailingColumnAtCharOffset`) phụ thuộc `char_offsets: Vec<u16>` field phải sync với `glyphs` trên mỗi `replace_text`. If out-of-sync, wide-glyph cursor navigation breaks. Mitigation: refactor `replace_text` để cập nhật `char_offsets` cùng lúc, add `assert_char_offsets_invariant()` in test.

**MEDIUM**: Module split (`buffer.rs` 1,334 → 8 sub-file <200-LOC) có thể break public API nếu `lib.rs` re-export chưa đầy đủ. Mitigation: W1 đầu tiên chỉ tạo `buffer/mod.rs` re-export all cũ từ `row.rs`/`mod.rs` split — commit dry-run + `cargo check -p core --all-features` giữa split & code port.

**LOW**: `ExpandWideGlyphSelection` không tồn tại trong WT — brief false-memory. Internal `_ExpandTextRow` (`textBuffer.cpp:1789`) là helper private, không cần public API. Drop gap 11 part.

**LOW**: `buffer_ext.rs` không tồn tại — brief stale state. No-op fold-in, documented trong §8.

**LOW**: `types::rle` empty — brief assumes P05 dependency. Inline `attr_runs: Vec<(u16, TextAttribute)>` đã equivalent. No P05 blocker.

**LOW**: `LineRendition` enum ĐÃ PORT — pass through.

**LOW**: `buffer_ext` removal không cần compat re-export (no symbol leakage).
