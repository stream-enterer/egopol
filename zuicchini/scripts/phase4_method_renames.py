#!/usr/bin/env python3
"""Phase 4 Parts 2+3: Apply method rename maps produced by agents.

Each agent reads a (emFoo.h, emFoo.rs) pair and outputs a JSON rename map.
This script applies all renames in one pass, handling string-literal protection
and external crate safety (same approach as phase4_type_renames.py).

Input: scripts/method_renames.json with structure:
{
  "renames": [
    {"old": "old_name", "new": "NewName", "header": "emFoo.h", "file": "emFoo.rs"},
    ...
  ],
  "diverged": [
    {"header": "emFoo.h", "file": "emFoo.rs", "cpp_name": "Bar", "reason": "..."},
    ...
  ]
}
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
WORKSPACE = ZUICCHINI.parent


def apply_renames(rename_map: dict[str, str], rs_files: list[Path]) -> int:
    """Apply all method renames across all files in one pass.

    Uses combined regex with string-literal protection.
    """
    if not rename_map:
        return 0

    # Build combined pattern, sorted by length descending
    sorted_names = sorted(rename_map.keys(), key=len, reverse=True)
    escaped = [re.escape(name) for name in sorted_names]
    pattern = re.compile(r"\b(" + "|".join(escaped) + r")\b")

    # String literal regex (protect from renaming)
    string_re = re.compile(
        r'(include_(?:bytes|str)!\s*\("[^"]*"\))'
        r"|" r'("(?:[^"\\\n]|\\.)*")'
    )

    changed = 0
    for rs_file in sorted(rs_files):
        content = rs_file.read_text()
        original = content

        # Split into code vs string segments
        segments = []
        last_end = 0
        for m in string_re.finditer(content):
            if m.start() > last_end:
                segments.append((content[last_end:m.start()], True))
            segments.append((m.group(0), False))
            last_end = m.end()
        if last_end < len(content):
            segments.append((content[last_end:], True))

        # Apply renames only to code segments
        result = []
        for text, is_code in segments:
            if is_code:
                text = pattern.sub(lambda m: rename_map[m.group(1)], text)
            result.append(text)

        content = "".join(result)

        if content != original:
            rs_file.write_text(content)
            changed += 1

    return changed


def apply_diverged(diverged_list: list[dict]):
    """Add DIVERGED comments to Rust files."""
    # Group by file
    from collections import defaultdict
    by_file = defaultdict(list)
    for d in diverged_list:
        by_file[d["file"]].append(d)

    for filename, items in by_file.items():
        filepath = ZUICCHINI / "src" / "emCore" / filename
        if not filepath.exists():
            continue

        content = filepath.read_text()

        # Check which DIVERGED comments already exist
        new_items = []
        for item in items:
            if f"DIVERGED: {item['cpp_name']}" not in content:
                new_items.append(item)

        if not new_items:
            continue

        # Insert DIVERGED block after imports
        lines = content.split("\n")
        insert_line = 0
        for i, line in enumerate(lines):
            if line.startswith("use ") or line.startswith("pub use "):
                insert_line = i + 1
        while insert_line < len(lines) and lines[insert_line].strip() == "":
            insert_line += 1

        diverged_lines = [f"// DIVERGED: {item['cpp_name']} — {item['reason']}" for item in new_items]
        block = ["", "// ── C++ methods not directly mapped ──"] + diverged_lines + [""]

        for j, line in enumerate(block):
            lines.insert(insert_line + j, line)

        filepath.write_text("\n".join(lines))


def main():
    rename_file = ZUICCHINI / "scripts" / "method_renames.json"
    if not rename_file.exists():
        print("No method_renames.json found. Run agents first to produce rename maps.")
        return 1

    with open(rename_file) as f:
        data = json.load(f)

    renames = data.get("renames", [])
    diverged = data.get("diverged", [])

    # Build rename map
    rename_map = {}
    for r in renames:
        old, new = r["old"], r["new"]
        if old != new:
            if old in rename_map and rename_map[old] != new:
                print(f"  CONFLICT: {old} → {rename_map[old]} vs {new}")
                continue
            rename_map[old] = new

    print(f"Renames: {len(rename_map)}")
    print(f"DIVERGED: {len(diverged)}")

    # Collect all Rust files
    rs_files = []
    for search_dir in [
        ZUICCHINI / "src",
        ZUICCHINI / "tests",
        ZUICCHINI / "examples",
        ZUICCHINI / "benches",
        WORKSPACE / "sosumi-7" / "src",
        WORKSPACE / "tests",
    ]:
        if search_dir.exists():
            rs_files.extend(search_dir.rglob("*.rs"))

    print(f"Files to scan: {len(rs_files)}")

    # Apply renames
    changed = apply_renames(rename_map, rs_files)
    print(f"Changed {changed} files")

    # Apply DIVERGED comments
    if diverged:
        apply_diverged(diverged)
        print(f"Added DIVERGED comments")

    # Validate
    print("\nRunning cargo check...")
    result = subprocess.run(
        ["cargo", "check", "--workspace"],
        cwd=WORKSPACE,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("FAIL: cargo check")
        print(result.stderr[-3000:])
        return 1
    print("cargo check: OK")

    return 0


if __name__ == "__main__":
    sys.exit(main())
