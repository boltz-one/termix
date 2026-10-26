#!/usr/bin/env python3
"""Loop-08 T1-T13: fold 123 ITermDispatch trait default todo!() -> no-op lane B.

Strategy: pre-existing bodies already suppress unused args with `let _ = arg;` lines
placed BEFORE the `todo!("...")` panic. We only need to convert the `todo!()` panic
itself into a no-op (empty body or trailing comment). Existing `let _ = arg;` lines
remain and now serve their original purpose (suppress unused_variables) without a
panic following them.

Reads `crates/core/src/dispatch/mod.rs`, replaces each line matching
`        todo!("ITermDispatch::Name")` (within trait body range 53-622 ONLY) with a
trailing inline comment line that documents the lane-B no-op fold:

        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11.

Methods without pre-existing `let _ = arg;` lines (zero-arg methods like
`unknown_sequence`, `warning_bell`, `carriage_return`, etc.) become empty-bodied
with just the comment — fine for trait default.

Out of scope: comment lines (5, 44, 1091) and blanket impl (`:632+`) untouched.
"""
import re
from pathlib import Path

PATH = Path("crates/core/src/dispatch/mod.rs")
src = PATH.read_text(encoding="utf-8")
lines = src.split("\n")

TRAIT_START = 53
TRAIT_END = 622

TODO_RE = re.compile(r'^        todo!\("ITermDispatch::(\w+)"\)$')

REPLACEMENT = "        // lane B no-op fold (loop-08); lane C AdaptDispatch concrete impl defer P09/P11."

changes = 0
for i, line in enumerate(lines, start=1):
    if TRAIT_START <= i <= TRAIT_END and TODO_RE.match(line):
        lines[i - 1] = REPLACEMENT
        changes += 1

new_src = "\n".join(lines)
PATH.write_text(new_src, encoding="utf-8")
print(f"folded {changes} todo!() -> no-op comment in trait body 53-622")
