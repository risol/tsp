#!/usr/bin/env python3
"""
Auto-fix patch test initializers: add `dynamic_import_no_cache: false,`
to RunFlags / WatchFlagsWithPaths / CompileFlags struct literals that
don't already have it.

Skips:
  - pub struct definitions (we never touch them)
  - struct update syntax (any `..` inside the literal)
  - match patterns where fields are bound without `: value` (e.g. `RunFlags { script, .. }`)
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
        if depth != 0:
            raise RuntimeError(
                f"Unbalanced braces in {m.group(1)} at {m.start()}"
            )
        end_off = i - 1
        yield m.group(1), m.start(), body_start, end_off


def is_init_block(body: str) -> bool:
    """True if `body` looks like a struct INITIALIZER (not a match pattern
    or struct-update form). Heuristics:

    - Must contain at least one `field: value,` assignment.
    - Must NOT contain `..` (struct update or pattern rest).
    - Must NOT contain a bare ident (no colon) followed by `,` —
      which would be a match-pattern binding shorthand. We look for
      `^\\s*\\w+\\s*,` or `^\\s*\\w+\\s*\\n` (a field bound by name).
    """
    if ".." in body:
        return False
    # need at least one named assignment
    if not re.search(r"\b\w+\s*:", body):
        return False
    # detect pattern shorthand: an ident (no colon) followed by comma
    # on the same line. Match `\n  ident ,` or start-of-line `ident ,`.
    for line in body.splitlines():
        s = line.strip()
        if not s or s.startswith("//"):
            continue
        # allow `field: value,` and `field: value` and `}` (closing).
        # `}),` is the end-of-nested-struct tail, not shorthand.
        if s == "}" or s.startswith("}"):
            continue
        if ":" in s.split(",", 1)[0]:
            continue
        # No colon on the first token of the line — this is shorthand.
        return False
    return True


def line_indent_for_field(source: str, struct_open_offset: int) -> str:
    line_start = source.rfind("\n", 0, struct_open_offset) + 1
    eol = source.find("\n", struct_open_offset)
    first_line = source[line_start:eol if eol != -1 else len(source)]
    stripped = first_line.lstrip(" \t")
    return first_line[: len(first_line) - len(stripped)]


def patch_file(path: Path) -> tuple[int, list[str]]:
    src = path.read_text(encoding="utf-8")
    blocks = list(find_blocks(src))
    fixed = 0
    skipped: list[str] = []
    # Process from the end so offsets stay valid.
    for name, name_off, body_start, end_off in reversed(blocks):
        body = src[body_start:end_off]
        if "dynamic_import_no_cache" in body:
            continue
        if not is_init_block(body):
            skipped.append(
                f"{path}:{_line_of(src, name_off)}: {name} skipped "
                f"(match pattern / update syntax)"
            )
            continue

        # Pick the indent of an existing field if any.
        m = re.search(r"\n([ \t]+)\w+\s*:", body)
        if m:
            field_indent = m.group(1)
        else:
            open_indent = line_indent_for_field(src, name_off)
            field_indent = open_indent + "  "
        insert = f"\n{field_indent}dynamic_import_no_cache: false,"
        src = src[:end_off] + insert + src[end_off:]
        fixed += 1
    if fixed:
        path.write_text(src, encoding="utf-8")
    return fixed, skipped


def _line_of(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def main() -> int:
    total = 0
    for p in TARGETS:
        if not p.exists():
            print(f"missing: {p}", file=sys.stderr)
            continue
        n, skipped = patch_file(p)
        for s in skipped:
            print(s)
        print(f"{p}: patched {n} struct literal(s)")
        total += n
    print(f"total patched: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
