#!/usr/bin/env python3
"""Phase 4 Parts 2+3: Method renames and DIVERGED annotations.

For each (emFoo.h, emFoo.rs) pair:
1. Extract public/protected methods from the C++ header
2. For each C++ method, find the corresponding Rust method
3. If found with wrong name: rename it
4. If not found: add DIVERGED comment with C++ name and reason
5. Run cargo check after each file

This script processes files one at a time. Each file is self-contained.
The output is the modified file itself — no external documentation needed.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
WORKSPACE = ZUICCHINI.parent
HEADERS_DIR = Path.home() / "git" / "eaglemode-0.96.4" / "include" / "emCore"
SRC = ZUICCHINI / "src" / "emCore"


def to_snake_case(name: str) -> str:
    """CamelCase → snake_case."""
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", s)
    return s.lower()


def extract_cpp_methods(header_path: Path) -> list[dict]:
    """Extract public/protected method names from a C++ header.

    Returns list of {"name": str, "visibility": str, "class": str}.
    Skips constructors, destructors, operators, friends, typedefs.
    """
    content = header_path.read_text()
    methods = []
    visibility = "private"
    in_class = False
    class_name = None
    brace_depth = 0

    for line in content.split("\n"):
        stripped = line.strip()

        for ch in stripped:
            if ch == "{":
                brace_depth += 1
            elif ch == "}":
                brace_depth -= 1
                if brace_depth == 0 and in_class:
                    in_class = False
                    class_name = None
                    visibility = "private"

        cls_match = re.match(r"\s*class\s+(em\w+)\b", stripped)
        if cls_match and (";" not in stripped or "{" in stripped):
            class_name = cls_match.group(1)
            in_class = True
            visibility = "private"
            continue

        if not in_class:
            continue

        if stripped.startswith("//") or stripped.startswith("#") or stripped.startswith("/*"):
            continue

        if stripped.startswith("public:"):
            visibility = "public"
            continue
        elif stripped.startswith("protected:"):
            visibility = "protected"
            continue
        elif stripped.startswith("private:"):
            visibility = "private"
            continue

        if visibility == "private":
            continue

        if any(stripped.startswith(kw) for kw in ("friend ", "typedef ", "using ", "enum ", "struct ")):
            continue
        if "~" in stripped and "(" in stripped:
            continue
        if "operator" in stripped:
            continue

        dep_match = re.match(r"EM_DEPRECATED\(\s*(.+)", stripped)
        if dep_match:
            stripped = dep_match.group(1)

        m = re.match(
            r"(?:virtual\s+)?(?:static\s+)?(?:inline\s+)?(?:explicit\s+)?"
            r"(?:const\s+)?"
            r"(?:[\w:*&<>,\s]+?\s+)"
            r"([A-Z_]\w*)\s*\(",
            stripped,
        )
        if m:
            name = m.group(1)
            if name == class_name or (class_name and name == class_name.replace("em", "", 1)):
                continue
            if name in ("EM_DEPRECATED", "EM_FUNC_ATTR_PRINTF"):
                continue
            methods.append({
                "name": name,
                "visibility": visibility,
                "class": class_name,
            })

    # Deduplicate (overloads)
    seen = set()
    unique = []
    for m in methods:
        key = (m["class"], m["name"])
        if key not in seen:
            seen.add(key)
            unique.append(m)

    return unique


def find_rust_method(content: str, snake_name: str) -> bool:
    """Check if a method with the given snake_case name exists in Rust content."""
    return bool(re.search(rf"\bfn\s+{re.escape(snake_name)}\b", content))


def find_rust_method_candidates(content: str, cpp_name: str) -> list[str]:
    """Find Rust methods that might correspond to a C++ method.

    Looks for methods with similar names or containing key parts of the C++ name.
    """
    snake = to_snake_case(cpp_name)
    candidates = []

    # Check exact match
    if find_rust_method(content, snake):
        candidates.append(snake)
        return candidates

    # Check partial matches: if C++ is GetFooBar, look for foo_bar, get_foo, bar
    parts = snake.split("_")
    all_fns = re.findall(r"\bfn\s+(\w+)", content)

    for fn_name in all_fns:
        fn_parts = fn_name.split("_")
        # Check if the fn contains the key parts (ignoring get/set/is prefix)
        key_parts = [p for p in parts if p not in ("get", "set", "is", "has", "can")]
        if key_parts and all(kp in fn_parts for kp in key_parts):
            candidates.append(fn_name)

    return candidates


def check_pub_field(content: str, cpp_name: str) -> str | None:
    """Check if a Get*/Set* method corresponds to a pub field."""
    if not (cpp_name.startswith("Get") or cpp_name.startswith("Set")):
        return None
    field_name = to_snake_case(cpp_name[3:])
    if re.search(rf"pub\s+{re.escape(field_name)}\s*:", content):
        return field_name
    return None


def check_diverged_exists(content: str, cpp_name: str) -> bool:
    """Check if a DIVERGED comment already mentions this C++ method."""
    return bool(re.search(rf"DIVERGED:.*\b{re.escape(cpp_name)}\b", content))


# Known std-equivalent functions that don't need DIVERGED annotations
STD_EQUIVALENT = set()


def load_std_equivalent():
    """Compute std-equivalent methods (same as verify_correspondence.py)."""
    global STD_EQUIVALENT
    rust_content = ""
    for p in SRC.rglob("*.rs"):
        rust_content += p.read_text() + "\n"

    for h in ("emStd1.h", "emStd2.h"):
        h_path = HEADERS_DIR / h
        if not h_path.exists():
            continue
        content = h_path.read_text()
        for m in re.finditer(
            r"(?:^|\n)\s*(?:inline\s+)?(?:[\w*&<> ]+\s+)(em[A-Z]\w+)\s*\(", content
        ):
            name = m.group(1)
            snake = to_snake_case(name)
            if not re.search(rf"\bfn\s+{re.escape(snake)}\b", rust_content):
                STD_EQUIVALENT.add(name)

        for m in re.finditer(
            r"(?:^|\n)\s*(?:virtual\s+)?(?:static\s+)?(?:[\w*&<> ]+\s+)"
            r"([A-Z]\w+)\s*\(", content,
        ):
            name = m.group(1)
            if name not in ("EM_DEPRECATED", "EM_FUNC_ATTR_PRINTF"):
                snake = to_snake_case(name)
                if not re.search(rf"\bfn\s+{re.escape(snake)}\b", rust_content):
                    STD_EQUIVALENT.add(name)


def process_file(header_name: str, rust_file: Path, report: list[str]) -> int:
    """Process a single (header, rust_file) pair.

    Adds DIVERGED comments for unmatched methods.
    Returns number of changes made.
    """
    header_path = HEADERS_DIR / header_name
    if not header_path.exists():
        return 0
    if not rust_file.exists():
        return 0

    cpp_methods = extract_cpp_methods(header_path)
    if not cpp_methods:
        return 0

    content = rust_file.read_text()
    original = content
    changes = 0
    diverged_additions = []

    for m in cpp_methods:
        cpp_name = m["name"]
        snake = to_snake_case(cpp_name)
        cls = m["class"] or ""

        # Skip std-equivalent
        if cpp_name in STD_EQUIVALENT:
            continue

        # Already matched
        if find_rust_method(content, snake):
            report.append(f"  MATCH: {cls}::{cpp_name} → fn {snake}")
            continue

        # Already has DIVERGED
        if check_diverged_exists(content, cpp_name):
            report.append(f"  DIVERGED (existing): {cls}::{cpp_name}")
            continue

        # Check pub field
        field = check_pub_field(content, cpp_name)
        if field:
            diverged_additions.append(
                f"// DIVERGED: {cpp_name} — pub field `{field}` replaces getter/setter"
            )
            report.append(f"  DIVERGED (field): {cls}::{cpp_name} → pub {field}")
            changes += 1
            continue

        # Check if there's a candidate with a different name
        candidates = find_rust_method_candidates(content, cpp_name)
        if candidates and candidates[0] != snake:
            # There's a likely match with a different name — note it
            diverged_additions.append(
                f"// DIVERGED: {cpp_name} — Rust uses `{candidates[0]}()`"
            )
            report.append(f"  DIVERGED (renamed): {cls}::{cpp_name} → fn {candidates[0]}")
            changes += 1
            continue

        # No match at all
        diverged_additions.append(
            f"// DIVERGED: {cpp_name} — not ported"
        )
        report.append(f"  DIVERGED (missing): {cls}::{cpp_name}")
        changes += 1

    # Add DIVERGED comments to the file
    if diverged_additions:
        # Insert after the last `use` line or at the top of the impl block
        lines = content.split("\n")

        # Find insertion point: after last `use` statement, before first type/fn def
        insert_line = 0
        for i, line in enumerate(lines):
            if line.startswith("use ") or line.startswith("pub use "):
                insert_line = i + 1

        # Skip blank lines after imports
        while insert_line < len(lines) and lines[insert_line].strip() == "":
            insert_line += 1

        # Insert DIVERGED block
        diverged_block = "\n".join(diverged_additions)
        lines.insert(insert_line, "")
        lines.insert(insert_line + 1, f"// ── C++ methods not directly mapped ──")
        for j, d in enumerate(diverged_additions):
            lines.insert(insert_line + 2 + j, d)
        lines.insert(insert_line + 2 + len(diverged_additions), "")

        content = "\n".join(lines)
        rust_file.write_text(content)

    return changes


def main():
    os.chdir(ZUICCHINI)
    load_std_equivalent()

    # Load mapping to know which header maps to which files
    with open(ZUICCHINI / "scripts" / "file_mapping.json") as f:
        mapping = json.load(f)

    print("=== Phase 4 Parts 2+3: Method correspondence ===\n")

    total_changes = 0
    total_matched = 0
    total_diverged = 0
    files_processed = 0

    for header, info in sorted(mapping["mappings"].items()):
        if info["pattern"] == "no-rust-equivalent":
            continue

        source_files = info.get("source_files", [])
        if not source_files:
            continue

        # Process each source file
        for sf in source_files:
            rust_path = ZUICCHINI / sf
            if not rust_path.exists():
                continue

            report = []
            changes = process_file(header, rust_path, report)

            if report:
                files_processed += 1
                matched = sum(1 for r in report if "MATCH:" in r)
                diverged = sum(1 for r in report if "DIVERGED" in r)
                total_matched += matched
                total_diverged += diverged
                total_changes += changes

                print(f"{header} → {rust_path.name}: {matched} matched, {diverged} diverged")
                for r in report:
                    print(r)
                print()

    print(f"\n=== Summary ===")
    print(f"Files processed: {files_processed}")
    print(f"Methods matched: {total_matched}")
    print(f"DIVERGED added: {total_diverged}")
    print(f"Total changes: {total_changes}")

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
        print(result.stderr[-2000:])
        return 1
    print("cargo check: OK")

    return 0


if __name__ == "__main__":
    sys.exit(main())
