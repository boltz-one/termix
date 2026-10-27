# P05 (types-til) Implementation Decisions

Date: 2026-10-27  
Status: Approved for execution

## Unresolved Questions from Planning → Decisions

### Q1: Color table ownership

**Context:** `campbell_color_table()` and `xterm256()` currently in `settings/src/color.rs`, used by builtin_schemes. P05 plan suggests moving to types to avoid circular deps.

**Decision:** **Keep in settings for now.**

**Rationale:**
- P07 (settings) not yet complete — moving breaks existing imports
- No actual circular dep: types doesn't depend on settings
- If P08/P10 need these tables, they can import from settings
- Alternative: duplicate in types (YAGNI - wait for actual need)

**Action:** None. P05 creates `types/src/color.rs` for ColorFix/Oklab/RGB conversion only, not color tables.

---

### Q2: EnumSet implementation

**Context:** WT uses `til::enumset` (bitset with custom iteration). Port verbatim or use Rust crate?

**Decision:** **Use `bitflags` crate.**

**Rationale:**
- Battle-tested, zero-cost abstraction
- No unsafe code needed
- Standard Rust idiom (better than custom port)
- WT `enumset` logic mostly std bitset ops

**Action:** Add `bitflags = "2.4"` to types/Cargo.toml. Port only if behavior mismatch found in tests.

---

### Q3: Unicode segmentation

**Context:** WT has grapheme cluster logic. Port or use `unicode-segmentation` crate?

**Decision:** **Use `unicode-segmentation` crate initially.**

**Rationale:**
- Correct Unicode 15.1 (WT uses 14.0)
- Defer WT logic port only if tests reveal behavior mismatch
- Standard crate (unicode-rs project)

**Action:** Add `unicode-segmentation = "1.10"` to types/Cargo.toml. Monitor P05 tests for mismatches.

---

### Q4: sgrStack/ThemeUtils scope

**Context:** Are sgrStack, ThemeUtils in P05 scope or defer to later phases?

**Decision:** **Defer to P08 (buffer).**

**Rationale:**
- `sgrStack` is buffer-specific (SGR attribute state machine)
- `ThemeUtils` is renderer-specific color theme ops
- Not base types — they consume types (rect, color) but aren't foundational

**Action:** Mark ledger rows for sgrStack/ThemeUtils as P08/P10 scope. P05 focuses on geometric/color/width primitives only.

---

## Approved Dependencies

```toml
[dependencies]
bitflags = "2.4"
unicode-segmentation = "1.10"
```

## Updated P05 Scope

**In scope:**
- Geometric: point, size, rect, viewport (52 methods)
- RLE: run-length encoding
- Color: RGB, Oklab, ColorFix (no color tables)
- Width: CodepointWidthDetector Unicode width tables
- Unicode: grapheme clusters (via crate), UTF-8/16 conversion
- Utils: GUID parse, console helpers
- EnumSet: via bitflags

**Out of scope (deferred):**
- sgrStack → P08 (buffer)
- ThemeUtils → P10 (renderer)
- Color tables (campbell/xterm256) → keep in settings

## Next Steps

1. Update loop-09-p05-plan.md with these decisions
2. Delegate to dev subagent: execute P05 per updated plan
3. Parallel: start P06 (connection) and P07 (settings) planning

---

**Approver:** Main agent  
**Implementation:** Ready for dev subagent
