# P05 Step 9: Out-of-Scope Files - Phase Assignment

Based on ledger triage (loop-09-p05-step1-triage.md), marking files for other phases.

## UIA Files → P15 (Accessibility)

**Scope:** UI Automation (UIA) provider implementations for screen readers/accessibility

Files to mark as P15:
- f01195: types/IControlAccessibilityInfo.h
- f01207: types/IUiaEventDispatcher.h
- f01208: types/IUiaTraceable.h
- f01211: types/ScreenInfoUiaProviderBase.cpp
- f01212: types/ScreenInfoUiaProviderBase.h
- f01214: types/TermControlUiaProvider.cpp
- f01215: types/TermControlUiaProvider.hpp
- f01216: types/TermControlUiaTextRange.cpp
- f01217: types/TermControlUiaTextRange.hpp
- f01219: types/UiaTextRangeBase.cpp
- f01220: types/UiaTextRangeBase.hpp
- f01221: types/UiaTracing.cpp
- f01222: types/UiaTracing.h

**Total:** 13 files

## Buffer Files → P08 (Buffer)

**Scope:** SGR attribute stack, buffer-specific logic

Files to mark as P08:
- f01202: types/inc/sgrStack.hpp
- f01214: types/sgrStack.cpp

**Total:** 2 files

## Renderer Files → P10 (Renderer)

**Scope:** Theme utilities, color theme operations

Files to mark as P10:
- f01203: types/inc/ThemeUtils.h
- f01218: types/ThemeUtils.cpp

**Total:** 2 files

## Input Files → P04 (Parser-Input)

**Scope:** Input event interfaces

Files to mark as P04:
- f01201: types/inc/IInputEvent.hpp

**Total:** 1 file

## Host/Platform Files → P16 (Host) or Replaced

**Scope:** User32 Windows APIs

Files to mark:
- f01204: types/inc/User32Utils.hpp → P16 or replaced (check if needed)

**Total:** 1 file

## Summary

| Phase | Files | Scope |
|---|---|---|
| P15 (a11y) | 13 | UIA providers, screen reader support |
| P08 (buffer) | 2 | sgrStack attribute management |
| P10 (renderer) | 2 | Theme utilities |
| P04 (input) | 1 | Input event interface |
| P16 (host) | 1 | User32 utilities |
| **Total** | **19** | Out-of-scope for P05 |

## Ledger Update Actions

Update `parity/ledger/P05-types-til/files.csv`:

1. **P15 rows (13 files):**
   - Status: `out-of-scope`
   - Notes: `P15-a11y`

2. **P08 rows (2 files):**
   - Status: `out-of-scope`
   - Notes: `P08-buffer`

3. **P10 rows (2 files):**
   - Status: `out-of-scope`
   - Notes: `P10-renderer`

4. **P04 rows (1 file):**
   - Status: `out-of-scope`
   - Notes: `P04-parser-input`

5. **P16 rows (1 file):**
   - Status: `out-of-scope` or `replaced`
   - Notes: `P16-host` or `replaced-ADR-0004`

## Ported Files Summary

From step 1 triage:
- **Port:** 37 files → P05 implementation ✅
- **Replaced:** 18 files → ADR-0004 (Rust std equivalents)
- **Out-of-scope:** 19 files → P04/P08/P10/P15/P16

**Ledger reconciliation:**
- Total WT files: 63 (step 1) vs 64 (ledger CSV header says 63 data rows + 1 header)
- Breakdown: 37 + 18 + 8 = 63 ✅

## Next Step

Update ledger CSV with phase assignments, then proceed to Step 10 (Coverage verification).
