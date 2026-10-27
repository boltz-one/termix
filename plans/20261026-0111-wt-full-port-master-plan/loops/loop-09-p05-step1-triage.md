# P05 Ledger Triage - ADR-0004 Decisions

Based on P05 spec and approved decisions (loop-09-p05-decisions.md).

## Classification Rules

**Port:** WT has custom behavior not replaceable by Rust std
**Replaced:** Rust std provides equivalent semantics + better (ADR-0004)
**Out-of-scope:** Belongs to other phases (P08, P10, P15)

## Triage Results (63 files)

### Port (37 files)

**til/ tests (24 files) - Port test vectors:**
- f01126: til/ut_til/BaseTests.cpp → wt_til_base.rs
- f01127: CoalesceTests.cpp → wt_til_coalesce.rs (RLE tests)
- f01128: ColorTests.cpp → wt_til_color.rs
- f01134: MathTests.cpp → wt_til_math.rs
- f01136: OperatorTests.cpp → wt_til_operators.rs
- f01137: PointTests.cpp → wt_til_point.rs
- f01138: RectangleTests.cpp → wt_til_rectangle.rs
- f01140: RunLengthEncodingTests.cpp → wt_til_rle.rs
- f01141: SizeTests.cpp → wt_til_size.rs
- f01147: u8u16convertTests.cpp → wt_til_convert.rs
- f01148: UnicodeTests.cpp → wt_til_unicode.rs

**til/ headers marked replaced below - test content ported, impl via std**

**types/ implementations (13 files):**
- f01190: types/CodepointWidthDetector.cpp → types/src/width.rs
- f01191: types/ColorFix.cpp → types/src/color.rs (Oklab)
- f01193: types/convert.cpp → types/src/convert.rs (UTF-8/16)
- f01194: types/GlyphWidth.cpp → types/src/width.rs (tables)
- f01196-f01200: inc/*.hpp (headers for above)
- f01205: inc/utils.hpp → types/src/utils.rs (GUID parse, console)
- f01206: inc/viewport.hpp → types/src/viewport.rs
- f01223: types/utils.cpp → types/src/utils.rs
- f01224: ut_types/CodepointWidthDetectorTests.cpp → wt_types_width.rs
- f01225: UtilsTests.cpp → wt_types_utils.rs
- f01226: UuidTests.cpp → wt_types_uuid.rs
- f01227: types/viewport.cpp → types/src/viewport.rs

### Replaced (18 files) - Rust std equivalents

**til/ - std replacements:**
- f01124-f01125: til/precomp.* → N/A (PCH, Rust has modules)
- f01129: EnumSetTests.cpp → bitflags crate (decision approved)
- f01130: EnvTests.cpp → std::env
- f01131: FlatSetTests.cpp → std::collections::BTreeSet
- f01132: GenerationalTests.cpp → generational_arena crate or custom (defer)
- f01133: HashTests.cpp → std::collections::HashMap
- f01135: mutex.cpp → std::sync::Mutex
- f01139: ReplaceTests.cpp → str::replace
- f01142: SmallVectorTests.cpp → smallvec crate
- f01143: SPSCTests.cpp → crossbeam::channel::bounded
- f01144: StaticMapTests.cpp → phf crate or const fn
- f01145: string.cpp → String/str
- f01146: throttled_func.cpp → defer (not foundational)

**types/ - std replacements:**
- f01192: types/colorTable.cpp → keep in settings (decision)
- f01198: inc/colorTable.hpp → keep in settings
- f01209-f01210: types/precomp.* → N/A

### Out-of-scope (8 files) - Other phases

**UIA (P15 a11y):**
- f01195: types/IControlAccessibilityInfo.h → P15
- f01201: inc/IInputEvent.hpp → P04 (terminal_input)
- f01207-f01208: IUiaEventDispatcher.h, IUiaTraceable.h → P15
- f01211-f01222: ScreenInfoUiaProviderBase.*, TermControlUiaProvider.*, UiaTextRange*.*, UiaTracing.* → P15 (11 files)

**Buffer/Renderer (P08/P10):**
- f01202: inc/sgrStack.hpp → P08 (buffer)
- f01203: inc/ThemeUtils.h → P10 (renderer)
- f01204: inc/User32Utils.hpp → P16 (host) or replaced
- f01214: types/sgrStack.cpp → P08
- f01218: types/ThemeUtils.cpp → P10

## Summary

| Category | Count | Action |
|---|---|---|
| Port | 37 | Implement in types crate |
| Replaced | 18 | Mark with ADR-0004, use Rust std/crates |
| Out-of-scope | 8 | Mark as P08/P10/P15 scope |
| **Total** | **63** | |

## Next Steps

1. Update ledger CSV with status:
   - Port files: status=ported, rust_path=crates/types/src/*.rs
   - Replaced files: status=replaced, adr=ADR-0004
   - Out-of-scope: status=out-of-scope, notes=P08/P10/P15

2. Create ADR-0004 replacement map document

3. Proceed to P05 Step 2: Point/Size/Rect implementation

## Dependencies Added

```toml
[dependencies]
bitflags = "2.4"
unicode-segmentation = "1.10"

[dev-dependencies]
# For replaced items if tests need them:
# smallvec = "1.11"
# generational-arena = "0.2"
# phf = "0.11"
```
