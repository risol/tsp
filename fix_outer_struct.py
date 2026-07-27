#!/usr/bin/env python3
"""Fix the 13 specific missing dynamic_import_no_cache initializers
that the previous heuristic missed (outer RunFlags / CompileFlags whose
body contains an inner struct that already has the field).

For each target line, find the matching closing `}),` of the OUTER
struct and insert `dynamic_import_no_cache: false,` just before it.
"""

from __future__ import annotations
import re
import sys
from pathlib import Path

TARGETS: list[tuple[Path, str]] = [
    # (file, subcommand kind) — kind is "Run" or "Compile"
    (Path(r"D:\GitHub\tsp\deno\cli\args\flags.rs"), "Run"),
    (Path(r"D:\GitHub\tsp\deno\cli\args\flags.rs"), "Compile"),
]


def find_outer_close(src: str, struct_open: int, struct_name: str) -> int:
    """Given the offset of `StructName {`, walk forward to find the
    matching `}` while correctly handling nested braces (including
    nested struct literals and code blocks).
    """
    depth = 0
    i = struct_open
    saw_open = False
    while i < len(src):
        ch = src[i]
        if ch == "{":
            depth += 1
            saw_open = True
        elif ch == "}":
            depth -= 1
            if saw_open and depth == 0:
                return i
        i += 1
    raise RuntimeError(f"no close for {struct_name} at {struct_open}")


def patch(path: Path, kind: str) -> int:
    src = path.read_text(encoding="utf-8")
    fixed = 0
    # Find every `subcommand: DenoSubcommand::Run(RunFlags {` or
    # `subcommand: DenoSubcommand::Compile(CompileFlags {`
    pat = re.compile(
        r"subcommand:\s*DenoSubcommand::" + kind + r"\("
        r"(RunFlags|CompileFlags|WatchFlagsWithPaths)\s*\{"
    )
    matches = list(pat.finditer(src))
    # Process from end to preserve offsets.
    for m in reversed(matches):
        struct_name = m.group(1)
        open_offset = m.end() - 1  # position of `{`
        # Skip if outer struct already has the field directly
        close = find_outer_close(src, open_offset, struct_name)
        body = src[open_offset + 1:close]
        # Check if any TOP-LEVEL field (not inside nested braces) is
        # dynamic_import_no_cache. Walk the body, tracking depth, and
        # look for the field at depth 0 only.
        if _has_top_level_field(body, "dynamic_import_no_cache"):
            continue
        # Skip if this is a struct update (e.g. `RunFlags { script: ...,
        # ..Default::default() }`) — you cannot have any named field
        # after `..`.
        if _has_struct_update(body):
            continue
        # Compute indent: walk the body and find the first non-empty
        # line's leading whitespace (it tells us the field indent).
        field_indent = _first_field_indent(body)
        # Insert just before the closing `}`.
        # The closing `}` may be followed by `),` or `,` or `=>` etc.
        # We insert `\n<indent>dynamic_import_no_cache: false,` right
        # before the `}`.
        new_src = (
            src[:close]
            + f"\n{field_indent}dynamic_import_no_cache: false,"
            + src[close:]
        )
        src = new_src
        fixed += 1
    if fixed:
        path.write_text(src, encoding="utf-8")
    return fixed


def _has_top_level_field(body: str, field: str) -> bool:
    """True if `field` appears as a top-level field name (i.e. not
    inside a nested { ... } block)."""
    depth = 0
    i = 0
    n = len(body)
    while i < n:
        ch = body[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif depth == 0 and body.startswith(field, i):
            # Make sure it's a field name (followed by `:` optionally
            # with whitespace).
            j = i + len(field)
            if j < n and body[j] in (":", " ", "\t"):
                return True
        i += 1
    return False


def _has_struct_update(body: str) -> bool:
    """True if `..` appears at depth 0 in the body (struct update syntax).
    When this is present, no additional named field may follow."""
    depth = 0
    i = 0
    n = len(body)
    while i < n:
        ch = body[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif depth == 0 and ch == "." and i + 1 < n and body[i + 1] == ".":
            return True
        i += 1
    return False


def _first_field_indent(body: str) -> str:
    """Find the indent (leading whitespace) of the first non-empty
    line in the body, used as a template for new field indentation."""
    for line in body.splitlines():
        if line.strip():
            stripped = line.lstrip(" \t")
            return line[: len(line) - len(stripped)]
    return "  "  # fallback


def main() -> int:
    total = 0
    for path, kind in TARGETS:
        if not path.exists():
            print(f"missing: {path}", file=sys.stderr)
            continue
        n = patch(path, kind)
        print(f"{path} ({kind}): patched {n}")
        total += n
    print(f"total: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
