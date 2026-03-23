#!/usr/bin/env python3
"""Phase 3: Flatten all Rust source files into src/emCore/ with emFoo.rs naming.

Reads file_mapping.json for header→file associations.
Uses the actual filesystem (post-Phase-2) to find source files.
Generates unique names for split files: emFoo.rs (primary) + emFoo{Suffix}.rs.

Steps:
1. Build move list from mapping + filesystem
2. Create src/emCore/ and src/emCore/shaders/
3. Move all files via git mv
4. Create marker files
5. Generate src/emCore/mod.rs
6. Update src/lib.rs
7. Rewrite all imports
8. Validate
"""

import json
import os
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
SRC = ZUICCHINI / "src"
EMCORE = SRC / "emCore"


def run(cmd: str, label: str = "", check: bool = True):
    """Run a shell command."""
    result = subprocess.run(cmd, shell=True, cwd=ZUICCHINI, capture_output=True, text=True)
    if check and result.returncode != 0:
        print(f"FAIL: {label or cmd}")
        print(result.stderr[-2000:])
        sys.exit(1)
    return result


def cargo_check(label: str):
    print(f"  cargo check: {label}")
    result = subprocess.run(
        ["cargo", "check", "--workspace"],
        cwd=ZUICCHINI.parent,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"FAIL: {label}")
        print(result.stderr[-3000:])
        sys.exit(1)
    print(f"  OK")


def to_camel_suffix(filename: str) -> str:
    """Convert a snake_case filename to CamelCase suffix.

    at_matrix -> AtMatrix
    x11_colors -> X11Colors
    scanline_avx2 -> ScanlineAvx2
    stroke_end -> StrokeEnd
    """
    stem = Path(filename).stem
    parts = stem.split("_")
    return "".join(p.capitalize() for p in parts)


def build_move_list(mapping_data: dict) -> list[dict]:
    """Build the definitive file move list.

    For each header:
    - rename (1 source): source → emFoo.rs
    - split (N sources): first source → emFoo.rs (primary), rest → emFoo{Suffix}.rs
    - extract (shared source, post-Phase-2 extraction): use the new extracted file
    - no-rust-equivalent: no move, just marker file

    For rust-only files: source → target per rust_only entries.
    """
    mappings = mapping_data["mappings"]
    rust_only = mapping_data.get("rust_only", {})

    moves = []  # list of {"source": str, "target": str, "header": str, "role": str}
    markers = []  # list of {"path": str, "type": str}

    # Track which source files have been assigned to avoid double-moves
    assigned_sources = set()

    for header, info in sorted(mappings.items()):
        pattern = info["pattern"]
        target_base = header.replace(".h", "")  # "emFoo"

        if pattern == "no-rust-equivalent":
            marker = info.get("marker_file")
            if marker:
                markers.append({"path": marker, "type": "no_rust_equivalent"})
            continue

        source_files = info.get("source_files", [])
        if not source_files:
            # Headers with only mod.rs code — check if a post-Phase-2 file exists
            # e.g., emTiling.h → layout/tiling.rs was created in Phase 2
            continue

        if pattern == "extract":
            # Post-Phase-2: the extracted code is in a new file
            # e.g., emLinearGroup.h → layout/linear_group.rs (created in Phase 2)
            # The mapping still says source = layout/linear.rs (pre-extraction)
            # Find the actual post-Phase-2 file
            post_phase2_file = find_extracted_file(header, source_files)
            if post_phase2_file:
                target = f"src/emCore/{target_base}.rs"
                moves.append({
                    "source": post_phase2_file,
                    "target": target,
                    "header": header,
                    "role": "primary",
                })
                assigned_sources.add(post_phase2_file)
            continue

        if pattern == "rename":
            source = source_files[0]
            # Check for mod.rs references (these contain the path in the source_files string)
            if "mod.rs" in source:
                # Find the post-Phase-2 file (e.g., layout/tiling.rs)
                post_file = find_modrs_extracted_file(header)
                if post_file:
                    target = f"src/emCore/{target_base}.rs"
                    moves.append({
                        "source": post_file,
                        "target": target,
                        "header": header,
                        "role": "primary",
                    })
                    assigned_sources.add(post_file)
                continue

            target = f"src/emCore/{target_base}.rs"
            moves.append({
                "source": source,
                "target": target,
                "header": header,
                "role": "primary",
            })
            assigned_sources.add(source)

        elif pattern == "split":
            # First source is primary → emFoo.rs
            # Rest get suffixes → emFoo{Suffix}.rs
            primary = source_files[0]
            target = f"src/emCore/{target_base}.rs"
            moves.append({
                "source": primary,
                "target": target,
                "header": header,
                "role": "primary",
            })
            assigned_sources.add(primary)

            for sf in source_files[1:]:
                suffix = to_camel_suffix(sf.split("/")[-1])
                split_target = f"src/emCore/{target_base}{suffix}.rs"
                moves.append({
                    "source": sf,
                    "target": split_target,
                    "header": header,
                    "role": "split",
                })
                assigned_sources.add(sf)

    # Rust-only files
    for rf, info in rust_only.items():
        target = info.get("target_rs")
        marker = info.get("marker_file")
        if target:
            source = "src/" + rf
            if source not in assigned_sources:
                moves.append({
                    "source": source,
                    "target": target,
                    "header": None,
                    "role": "rust_only",
                })
                assigned_sources.add(source)
        if marker:
            markers.append({"path": marker, "type": "rust_only"})

    # Handle special files that need moving but aren't in the mapping
    # widget/mod.rs functions, foundation/mod.rs functions
    # These stay with their module until the flatten, then become part of emCore

    return moves, markers


def find_extracted_file(header: str, original_sources: list[str]) -> str | None:
    """Find the post-Phase-2 extracted file for an 'extract' pattern header.

    After Phase 2, extracted types live in new files:
    - emLinearGroup.h: linear.rs → linear_group.rs
    - emStrokeEnd.h: stroke.rs → stroke_end.rs
    - emModel.h: context.rs → model_base.rs
    - emSigModel.h: watched_var.rs → sig_model.rs
    - emVarSigModel.h: watched_var.rs → var_sig_model.rs
    - emGroup.h: → group.rs
    """
    name_map = {
        "emLinearGroup.h": "src/layout/linear_group.rs",
        "emPackGroup.h": "src/layout/pack_group.rs",
        "emRasterGroup.h": "src/layout/raster_group.rs",
        "emStrokeEnd.h": "src/render/stroke_end.rs",
        "emModel.h": "src/model/model_base.rs",
        "emSigModel.h": "src/model/sig_model.rs",
        "emVarSigModel.h": "src/model/var_sig_model.rs",
        "emGroup.h": "src/layout/group.rs",
    }
    path = name_map.get(header)
    if path and (ZUICCHINI / path).exists():
        return path
    return None


def find_modrs_extracted_file(header: str) -> str | None:
    """Find files extracted from mod.rs in Phase 2."""
    if header == "emTiling.h":
        path = "src/layout/tiling.rs"
        if (ZUICCHINI / path).exists():
            return path
    return None


def build_module_declarations(moves: list[dict]) -> str:
    """Generate the content of src/emCore/mod.rs from the move list."""
    # Collect all module names
    modules = set()
    for move in moves:
        target = move["target"]
        stem = Path(target).stem  # e.g., "emColor", "emPainterScanline"
        modules.add(stem)

    # Also find any existing modules we know about
    # (widget/mod.rs functions will need a home)

    lines = [
        "#![allow(non_snake_case)]\n",
        "#![allow(non_camel_case_types)]\n",
        "\n",
    ]

    # Sort modules and declare them
    # Determine visibility: pub for types that are part of the library API
    for mod_name in sorted(modules):
        lines.append(f"pub mod {mod_name};\n")

    return "".join(lines)


def collect_reexports() -> dict[str, list[str]]:
    """Read current mod.rs files to understand what's publicly exported.

    Returns {old_module: [list of pub symbols]}.
    """
    reexports = {}
    for mod_dir in ["foundation", "input", "layout", "model", "panel", "render",
                     "scheduler", "widget", "window"]:
        mod_path = SRC / mod_dir / "mod.rs"
        if not mod_path.exists():
            continue

        symbols = []
        content = mod_path.read_text()
        for m in re.finditer(r"pub use \w+::(\{[^}]+\}|\w+)", content):
            text = m.group(1)
            if text.startswith("{"):
                # Parse {Foo, Bar, Baz}
                inner = text.strip("{}")
                for item in inner.split(","):
                    item = item.strip()
                    if item:
                        symbols.append(item)
            else:
                symbols.append(text)

        reexports[mod_dir] = symbols

    return reexports


def rewrite_imports_in_file(file_path: Path, old_modules: list[str]):
    """Rewrite `use crate::{old_module}::X` to `use crate::emCore::X` in a file.

    Also handles `super::` references for files within emCore.
    """
    content = file_path.read_text()
    original = content

    # Replace use crate::{module}:: with use crate::emCore::
    for mod_name in old_modules:
        content = re.sub(
            rf"use crate::{mod_name}::",
            "use crate::emCore::",
            content,
        )

    # For files IN emCore, replace super:: references that pointed to old module
    # (super:: in emCore means crate root, which is wrong — should be crate::emCore::)
    if "emCore" in str(file_path):
        # super:: within emCore files should reference sibling modules
        # Most super:: usage was for old module parent → now needs crate::emCore::
        content = re.sub(
            r"use super::(\w+)::(\w+)",
            r"use crate::emCore::\1::\2",
            content,
        )

    if content != original:
        file_path.write_text(content)
        return True
    return False


def main():
    os.chdir(ZUICCHINI)

    print("=== Phase 3: Flatten to src/emCore/ ===")
    print()

    # Load mapping
    with open("scripts/file_mapping.json") as f:
        mapping_data = json.load(f)

    # Build move list
    moves, markers = build_move_list(mapping_data)

    print(f"Files to move: {len(moves)}")
    print(f"Markers to create: {len(markers)}")

    # Check for duplicate targets
    from collections import Counter
    tgt_counts = Counter(m["target"] for m in moves)
    dupes = {k: v for k, v in tgt_counts.items() if v > 1}
    if dupes:
        print(f"ERROR: Duplicate targets detected:")
        for t, c in sorted(dupes.items()):
            sources = [m["source"] for m in moves if m["target"] == t]
            print(f"  {t}: {sources}")
        sys.exit(1)

    # Check for duplicate sources
    src_counts = Counter(m["source"] for m in moves)
    src_dupes = {k: v for k, v in src_counts.items() if v > 1}
    if src_dupes:
        print(f"ERROR: Duplicate sources detected: {src_dupes}")
        sys.exit(1)

    print("\nMove list:")
    for m in sorted(moves, key=lambda x: x["target"]):
        print(f"  {m['source']:50s} → {m['target']}")

    # ─── Step 1: Create directories ─────────────────────────────
    print("\n1. Creating directories...")
    EMCORE.mkdir(exist_ok=True)
    (EMCORE / "shaders").mkdir(exist_ok=True)

    # ─── Step 2: Move files ──────────────────────────────────────
    print("\n2. Moving files...")
    for m in moves:
        src = ZUICCHINI / m["source"]
        tgt = ZUICCHINI / m["target"]
        if not src.exists():
            print(f"  WARNING: Source not found: {m['source']}")
            continue
        run(f"git mv {src} {tgt}", f"move {m['source']} → {m['target']}")

    # Move shader
    shader_src = SRC / "render" / "shaders" / "tile_composite.wgsl"
    shader_tgt = EMCORE / "shaders" / "tile_composite.wgsl"
    if shader_src.exists():
        run(f"git mv {shader_src} {shader_tgt}", "move shader")

    # ─── Step 3: Handle mod.rs functions ─────────────────────────
    # widget/mod.rs has trace_input_enabled() and check_mouse_round_rect()
    # foundation/mod.rs has set_fatal_error_graphical() and is_fatal_error_graphical()
    # These need to go somewhere in emCore.
    print("\n3. Handling mod.rs function code...")

    # Foundation mod.rs functions → append to emStd1.rs (which is dlog.rs renamed)
    fmod = SRC / "foundation" / "mod.rs"
    if fmod.exists():
        content = fmod.read_text()
        # Extract the function code (everything after the use/re-export lines)
        lines = content.splitlines(keepends=True)
        func_start = None
        for i, line in enumerate(lines):
            if line.startswith("use std::sync") or line.startswith("static "):
                func_start = i
                break
        if func_start is not None:
            func_code = "".join(lines[func_start:])
            # Append to emStd1.rs
            std1_path = EMCORE / "emStd1.rs"
            if std1_path.exists():
                existing = std1_path.read_text()
                std1_path.write_text(existing + "\n" + func_code)
                print("  Appended fatal_error functions to emStd1.rs")

    # Widget mod.rs functions → create a utility file
    wmod = SRC / "widget" / "mod.rs"
    if wmod.exists():
        content = wmod.read_text()
        lines = content.splitlines(keepends=True)
        func_start = None
        for i, line in enumerate(lines):
            if line.startswith("use crate::") and "pub use" not in lines[max(0, i - 1) if i > 0 else 0]:
                func_start = i
                break
        if func_start is not None:
            func_code = "".join(lines[func_start:])
            # These are widget utilities — put in emBorder.rs or a separate file
            # For now, create a widget_utils module
            utils_path = EMCORE / "widget_utils.rs"
            utils_path.write_text(func_code)
            print("  Created widget_utils.rs for mod.rs functions")

    # ─── Step 4: Create marker files ─────────────────────────────
    print("\n4. Creating marker files...")
    for marker in markers:
        path = ZUICCHINI / marker["path"]
        path.touch()
    print(f"  Created {len(markers)} marker files")

    # ─── Step 5: Generate mod.rs ─────────────────────────────────
    print("\n5. Generating src/emCore/mod.rs...")
    mod_content = build_module_declarations(moves)

    # Add widget_utils if it was created
    if (EMCORE / "widget_utils.rs").exists():
        mod_content += "pub(crate) mod widget_utils;\n"

    (EMCORE / "mod.rs").write_text(mod_content)

    # ─── Step 6: Update lib.rs ───────────────────────────────────
    print("\n6. Updating src/lib.rs...")
    lib_content = (
        "#[allow(non_snake_case)]\n"
        "pub mod emCore;\n"
        "\n"
        "mod debug;\n"
    )
    (SRC / "lib.rs").write_text(lib_content)

    # ─── Step 7: Clean up old directories ────────────────────────
    print("\n7. Cleaning up old directories...")
    for mod_dir in ["foundation", "input", "layout", "model", "panel", "render",
                     "scheduler", "widget", "window"]:
        mod_path = SRC / mod_dir
        if mod_path.exists():
            # Remove mod.rs (no longer needed)
            modrs = mod_path / "mod.rs"
            if modrs.exists():
                modrs.unlink()
            # Remove shaders dir if empty
            shaders = mod_path / "shaders"
            if shaders.exists() and not any(shaders.iterdir()):
                shaders.rmdir()
            # Remove dir if empty
            try:
                mod_path.rmdir()
            except OSError:
                remaining = list(mod_path.iterdir())
                print(f"  WARNING: {mod_dir}/ not empty: {[str(r) for r in remaining]}")

    # ─── Step 8: Rewrite imports ─────────────────────────────────
    print("\n8. Rewriting imports...")
    old_modules = [
        "foundation", "input", "layout", "model", "panel",
        "render", "scheduler", "widget", "window",
    ]
    changed_count = 0
    for rs_file in EMCORE.rglob("*.rs"):
        if rewrite_imports_in_file(rs_file, old_modules):
            changed_count += 1

    # Also fix imports in debug/ and any tests
    for rs_file in (SRC / "debug").rglob("*.rs"):
        if rewrite_imports_in_file(rs_file, old_modules):
            changed_count += 1

    # Fix sosumi-7 crate imports
    sosumi_src = ZUICCHINI.parent / "sosumi-7" / "src"
    if sosumi_src.exists():
        for rs_file in sosumi_src.rglob("*.rs"):
            content = rs_file.read_text()
            original = content
            for mod_name in old_modules:
                content = content.replace(
                    f"zuicchini::{mod_name}::",
                    "zuicchini::emCore::",
                )
            if content != original:
                rs_file.write_text(content)
                changed_count += 1

    print(f"  Rewrote imports in {changed_count} files")

    # ─── Step 9: Cargo check ────────────────────────────────────
    print("\n9. Validation...")
    cargo_check("Phase 3 structural check")

    print("\n=== Phase 3 Complete ===")


if __name__ == "__main__":
    main()
