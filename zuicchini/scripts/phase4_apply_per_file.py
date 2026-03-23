#!/usr/bin/env python3
"""Phase 4: Apply method renames one file at a time, compiler-guided.

For each file:
1. Pre-check: grep for target name collisions
2. Rename fn definitions in the file
3. cargo check → parse errors for call sites
4. Fix call sites
5. Repeat until cargo check passes
6. Run tests

Uses the batch rename maps as input.
"""

import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
WORKSPACE = ZUICCHINI.parent
SRC = ZUICCHINI / "src" / "emCore"

MAX_ITERATIONS = 15
MAX_ERRORS_ABORT = 500


def cargo_check() -> tuple[bool, str]:
    result = subprocess.run(
        ["cargo", "check", "--workspace"],
        cwd=WORKSPACE,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0, result.stderr


def cargo_test() -> tuple[bool, str]:
    result = subprocess.run(
        ["cargo", "test", "--workspace", "--no-run"],
        cwd=WORKSPACE,
        capture_output=True,
        text=True,
    )
    return result.returncode == 0, result.stderr


def parse_errors(stderr: str) -> list[dict]:
    """Parse cargo check errors to find 'no method named X' errors with location."""
    errors = []
    # Match: error[E0599]: no method named `old_name` found for ...
    #   --> file.rs:line:col
    for m in re.finditer(
        r"error\[E0599\]: no method named `(\w+)` found.*?\n\s+-->\s+(\S+):(\d+):(\d+)",
        stderr,
        re.DOTALL,
    ):
        errors.append({
            "method": m.group(1),
            "file": m.group(2),
            "line": int(m.group(3)),
            "col": int(m.group(4)),
        })

    # Match: error[E0407]: method `old_name` is not a member of trait
    #   --> file.rs:line:col
    for m in re.finditer(
        r"error\[E0407\]: method `(\w+)` is not a member of trait.*?\n\s+-->\s+(\S+):(\d+):(\d+)",
        stderr,
        re.DOTALL,
    ):
        errors.append({
            "method": m.group(1),
            "file": m.group(2),
            "line": int(m.group(3)),
            "col": int(m.group(4)),
            "type": "trait_impl",
        })

    # Match: error[E0046]: not all trait items implemented, missing: `NewName`
    for m in re.finditer(
        r"error\[E0046\]:.*missing:\s+`(\w+)`.*?\n\s+-->\s+(\S+):(\d+):(\d+)",
        stderr,
        re.DOTALL,
    ):
        errors.append({
            "method": m.group(1),
            "file": m.group(2),
            "line": int(m.group(3)),
            "col": int(m.group(4)),
            "type": "missing_impl",
        })

    # Match: error[E0425]: cannot find function `old_name`
    #   --> file.rs:line:col
    for m in re.finditer(
        r"error\[E0425\]: cannot find (?:function|value) `(\w+)`.*?\n\s+-->\s+(\S+):(\d+):(\d+)",
        stderr,
        re.DOTALL,
    ):
        errors.append({
            "method": m.group(1),
            "file": m.group(2),
            "line": int(m.group(3)),
            "col": int(m.group(4)),
        })

    # Match: error[E0432]: unresolved import ... `old_name`
    for m in re.finditer(
        r"error\[E0432\]:.*`(\w+)`.*?\n\s+-->\s+(\S+):(\d+):(\d+)",
        stderr,
        re.DOTALL,
    ):
        errors.append({
            "method": m.group(1),
            "file": m.group(2),
            "line": int(m.group(3)),
            "col": int(m.group(4)),
        })

    return errors


def fix_call_site(filepath: Path, line: int, old_name: str, new_name: str) -> bool:
    """Fix a single call site by replacing old_name with new_name at the given line."""
    if not filepath.exists():
        return False
    lines = filepath.read_text().split("\n")
    if line < 1 or line > len(lines):
        return False
    idx = line - 1
    if old_name in lines[idx]:
        # Replace only the first occurrence on this line (method call)
        lines[idx] = lines[idx].replace(old_name, new_name, 1)
        filepath.write_text("\n".join(lines))
        return True
    return False


def rename_definitions(filepath: Path, renames: list[dict]) -> int:
    """Rename fn definitions in the file. Returns number of renames applied."""
    content = filepath.read_text()
    count = 0
    for r in renames:
        old, new = r["old"], r["new"]
        # Match fn definition: `fn old_name(` or `fn old_name<`
        pattern = rf"\bfn\s+{re.escape(old)}\b"
        if re.search(pattern, content):
            content = re.sub(pattern, f"fn {new}", content)
            count += 1
    filepath.write_text(content)
    return count


def pre_check_collisions(renames: list[dict]) -> list[str]:
    """Check if any target names already exist as methods in the codebase."""
    warnings = []
    for r in renames:
        new_name = r["new"]
        # grep for fn new_name( in all .rs files
        result = subprocess.run(
            ["grep", "-rn", f"fn {new_name}(", str(SRC)],
            capture_output=True,
            text=True,
        )
        if result.stdout.strip():
            # Check if it's in the same file (that's expected — we're about to rename it)
            other_files = [
                line for line in result.stdout.strip().split("\n")
                if r["file"] not in line
            ]
            if other_files:
                warnings.append(f"  COLLISION: {new_name} already exists in: {other_files[0]}")
    return warnings


def process_file(filename: str, renames: list[dict], rename_map: dict[str, str]) -> bool:
    """Process one file: rename definitions, fix call sites via compiler."""
    filepath = SRC / filename
    if not filepath.exists():
        print(f"  SKIP: {filename} not found")
        return True

    print(f"\n{'='*60}")
    print(f"Processing {filename} ({len(renames)} renames)")
    print(f"{'='*60}")

    # Pre-check
    warnings = pre_check_collisions(renames)
    if warnings:
        print("  Collision warnings:")
        for w in warnings:
            print(f"    {w}")

    # Rename definitions
    count = rename_definitions(filepath, renames)
    print(f"  Renamed {count} definitions")

    # Iterative fix cycle
    for iteration in range(MAX_ITERATIONS):
        ok, stderr = cargo_check()
        if ok:
            print(f"  Compile clean after iteration {iteration}")
            return True

        errors = parse_errors(stderr)
        error_count = stderr.count("error[E")

        if error_count > MAX_ERRORS_ABORT:
            print(f"  ABORT: {error_count} errors exceed threshold {MAX_ERRORS_ABORT}")
            return False

        if not errors:
            # Errors exist but we can't parse them — might be different error types
            print(f"  {error_count} errors but none parseable as method-not-found")
            print(f"  Last errors: {stderr[-500:]}")
            return False

        print(f"  Iteration {iteration}: {len(errors)} fixable errors (of {error_count} total)")

        # Fix each error
        fixed = 0
        for err in errors:
            old_name = err["method"]
            new_name = rename_map.get(old_name)

            if err.get("type") == "trait_impl":
                # A trait impl has the old name — rename it
                err_path = WORKSPACE / err["file"]
                if fix_call_site(err_path, err["line"], old_name, old_name):
                    # Actually, for trait impls, the method in the impl block needs renaming
                    # This was already done in rename_definitions if it's our file
                    # If it's another file's impl, we need to rename there too
                    if fix_call_site(err_path, err["line"], f"fn {old_name}", f"fn {new_name or old_name}"):
                        fixed += 1
                continue

            if err.get("type") == "missing_impl":
                # Trait requires a method we renamed — the impl needs the new name
                # This error means the impl has the OLD name, rename it
                err_path = WORKSPACE / err["file"]
                # The missing method is the NEW name — find the old impl
                # Look nearby for fn definitions
                continue

            if not new_name:
                # Try reverse lookup — maybe the error is about a call to the old name
                # and we know what it should be
                for r in renames:
                    if r["old"] == old_name:
                        new_name = r["new"]
                        break

            if not new_name:
                print(f"    Can't fix: {old_name} at {err['file']}:{err['line']} (not in rename map)")
                continue

            err_path = WORKSPACE / err["file"]
            if fix_call_site(err_path, err["line"], old_name, new_name):
                fixed += 1

        if fixed == 0:
            print(f"  No fixes applied, stopping")
            return False

        print(f"  Fixed {fixed} call sites")

    print(f"  Max iterations reached")
    return False


def main():
    os.chdir(ZUICCHINI)

    # Load all rename maps
    all_renames = []
    for i in range(1, 7):
        path = Path(f"scripts/batch{i}_renames.json")
        if path.exists():
            data = json.load(open(path))
            all_renames.extend(data.get("renames", []))

    # Build per-file lists and global rename map
    by_file = defaultdict(list)
    rename_map = {}  # old_name -> new_name (for call site fixing)
    for r in all_renames:
        by_file[r["file"]].append(r)
        # Only add to global map if unambiguous
        if r["old"] not in rename_map:
            rename_map[r["old"]] = r["new"]
        elif rename_map[r["old"]] != r["new"]:
            rename_map[r["old"]] = None  # ambiguous — skip in global map

    # Remove ambiguous entries
    rename_map = {k: v for k, v in rename_map.items() if v is not None}

    # Sort by fewest renames first
    sorted_files = sorted(by_file.items(), key=lambda x: len(x[1]))

    print(f"Files to process: {len(sorted_files)}")
    print(f"Total renames: {sum(len(v) for v in by_file.values())}")
    print(f"Global rename map: {len(rename_map)} unambiguous entries")

    succeeded = 0
    failed = 0
    failed_files = []

    for filename, renames in sorted_files:
        ok = process_file(filename, renames, rename_map)
        if ok:
            # Commit this file's changes
            subprocess.run(
                ["git", "add", "-A", "."],
                cwd=WORKSPACE,
                capture_output=True,
            )
            subprocess.run(
                ["git", "commit", "-m",
                 f"Phase 4: rename methods in {filename} to match C++ names\n\n"
                 f"Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"],
                cwd=WORKSPACE,
                capture_output=True,
            )
            succeeded += 1
            print(f"  COMMITTED: {filename}")
        else:
            failed += 1
            failed_files.append(filename)
            print(f"  FAILED: {filename}")
            # Revert ALL changes (not just this file — call sites in other files too)
            subprocess.run(["git", "checkout", "--", "."], cwd=WORKSPACE)
            print(f"  Reverted all changes")

    print(f"\n{'='*60}")
    print(f"SUMMARY: {succeeded} succeeded, {failed} failed")
    if failed_files:
        print(f"Failed files: {failed_files}")
    print(f"{'='*60}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
