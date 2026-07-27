#!/usr/bin/env python3
"""Verify every RunFlags/WatchFlagsWithPaths/CompileFlags struct literal
in the patched files contains a dynamic_import_no_cache field.
"""

from __future__ import annotations
import re
import sys
from pathlib import Path

TARGETS = [
    Path(r"D:\GitHub\tsp\deno\cli\args\flags.rs"),
    Path(r"D:\GitHub\tsp\deno\cli\tools\compile.rs"),
]

STRUCTS = ("RunFlags", "WatchFlagsWithPaths", "CompileFlags")


def line_of(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def find_blocks(source: str):
    pat = re.compile(r"\b(" + "|".join(STRUCTS) + r")\s*\{")
    for m in pat.finditer(source):
        prefix = source[max(0, m.start() - 16):m.start()]
        if "pub struct" in prefix:
            continue
        body_start = m.end()
        depth = 1
        i = body_start
        while i < len(source) and depth > 0:
            ch = source[i]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            i += 1
        end_off = i - 1
        yield m.group(1), m.start(), body_start, end_off


def main() -> int:
    bad: list[tuple[str, int, str]] = []
    for path in TARGETS:
        src = path.read_text(encoding="utf-8")
        for name, name_off, body_start, end_off in find_blocks(src):
            body = src[body_start:end_off]
            if "dynamic_import_no_cache" not in body:
                line_no = line_of(src, name_off)
                snippet = src[name_off:min(end_off + 1, name_off + 200)]
                bad.append((str(path), line_no, snippet))
    if bad:
        for path, line, snip in bad:
            print(f"{path}:{line} STILL MISSING")
            print(f"  {snip!r}")
        print(f"failures: {len(bad)}")
        return 1
    print("all struct literals have dynamic_import_no_cache [OK]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
