#!/usr/bin/env python3
"""Phase 4 Part 1: Rename all types to match C++ em-prefixed names.

Reads inventory.json for the 82 pending type renames.
Does codebase-wide replacement, skipping string literals and include! paths.
Validates with cargo check + clippy.

Efficient: one pass per file, combined regex for all renames.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
WORKSPACE = ZUICCHINI.parent

# External crate prefixes — never rename types after these
EXTERNAL_PREFIXES = re.compile(
    r"(?:wgpu|winit|raw_window_handle|slotmap|libloading|naga|"
    r"image|log|env_logger|pollster|bytemuck|ab_glyph)"
    r"::"
)


def build_rename_regex(rename_map: dict) -> tuple[re.Pattern, dict]:
    """Build a single compiled regex that matches any of the old type names.

    Returns (pattern, lookup_dict) where pattern matches \b(Name1|Name2|...)\b
    and lookup_dict maps each old name to its new name.
    """
    # Sort by length descending to match longer names first
    sorted_names = sorted(rename_map.keys(), key=len, reverse=True)
    escaped = [re.escape(name) for name in sorted_names]
    pattern = re.compile(r"\b(" + "|".join(escaped) + r")\b")
    return pattern, rename_map


def split_code_and_strings(content: str) -> list[tuple[str, bool]]:
    """Split file content into segments: (text, is_code).

    Strings, raw strings, and include_bytes!/include_str! paths are marked
    as is_code=False and won't be renamed. Comments ARE code (we rename in
    doc comments).
    """
    # Regex that matches string literals and include macros.
    # These segments are protected from renaming.
    token_re = re.compile(
        r'(include_(?:bytes|str)!\s*\("[^"]*"\))'   # include macros
        r"|" r'("(?:[^"\\\n]|\\.)*")'                # regular strings (single-line only)
    )

    segments = []
    last_end = 0

    for m in token_re.finditer(content):
        # Code before this match
        if m.start() > last_end:
            segments.append((content[last_end:m.start()], True))
        # The string/include — not code
        segments.append((m.group(0), False))
        last_end = m.end()

    # Remaining code after last match
    if last_end < len(content):
        segments.append((content[last_end:], True))

    return segments


def apply_renames(content: str, pattern: re.Pattern, lookup: dict) -> str:
    """Apply type renames to a file, skipping string literals."""
    segments = split_code_and_strings(content)

    result = []
    for text, is_code in segments:
        if is_code:
            # Simple replacement — external crate false renames are handled
            # in post-processing below
            text = pattern.sub(lambda m: lookup[m.group(1)], text)
        result.append(text)

    new_content = "".join(result)

    # Post-process: revert any external crate false renames that slipped through
    new_content = re.sub(
        r"(wgpu|winit|raw_window_handle|slotmap|naga)::em([A-Z]\w*)",
        r"\1::\2",
        new_content,
    )
    # Also revert enum variants from external crates
    new_content = re.sub(
        r"(BindingType|TextureFormat|LoadOp|ShaderStages|TextureSampleType|"
        r"TextureViewDimension|SamplerBindingType|BufferBindingType|"
        r"PrimitiveTopology|FrontFace|PolygonMode|CompareFunction|"
        r"BlendFactor|BlendOperation|TextureUsages|BufferUsages|"
        r"VertexFormat|IndexFormat|FilterMode|AddressMode|Features|"
        r"PowerPreference|PresentMode|CompositeAlphaMode)::em([A-Z]\w*)",
        r"\1::\2",
        new_content,
    )

    return new_content


def main():
    with open(ZUICCHINI / "scripts" / "inventory.json") as f:
        inventory = json.load(f)

    renames = [r for r in inventory["type_renames"] if r["status"] == "pending"]
    print(f"Type renames: {len(renames)} pending")

    # Build rename map
    rename_map = {}
    for r in renames:
        old, new = r["current_rust_type"], r["target_rust_type"]
        if old != new and old not in rename_map:
            rename_map[old] = new

    print(f"Unique renames: {len(rename_map)}")

    # Build combined regex
    pattern, lookup = build_rename_regex(rename_map)

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

    # Apply renames — one pass per file
    changed_files = 0
    for rs_file in sorted(rs_files):
        content = rs_file.read_text()
        new_content = apply_renames(content, pattern, lookup)
        if new_content != content:
            rs_file.write_text(new_content)
            changed_files += 1

    print(f"Changed {changed_files} files")

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

    print("\nRunning cargo clippy...")
    result = subprocess.run(
        ["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
        cwd=WORKSPACE,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("FAIL: cargo clippy")
        print(result.stderr[-3000:])
        return 1
    print("cargo clippy: OK")

    print(f"\nPhase 4 Part 1 complete: {len(rename_map)} types renamed in {changed_files} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
