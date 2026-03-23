#!/usr/bin/env python3
"""Fix all import paths after Phase 3 flatten.

Handles:
1. `use crate::{old_module}::X` → `use crate::emCore::{new_module}::X`
2. `crate::{old_module}::X::Y` (inline full paths) → `crate::emCore::{new_module}::X::Y`
3. `use super::{old_name}::X` → `use crate::emCore::{new_module}::X` (for emCore files)
4. `super::{old_name}::X` inline references
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
SRC = ZUICCHINI / "src"
EMCORE = SRC / "emCore"

OLD_MODULES = [
    "foundation", "input", "layout", "model", "panel",
    "render", "scheduler", "widget", "window",
]

# Old submodule names → new emCore module names
# e.g., "color" (from foundation/color.rs) → "emColor" (now emCore/emColor.rs)
OLD_SUBMOD_TO_NEW = {}


def build_indices():
    """Build lookup tables for import rewriting."""
    global OLD_SUBMOD_TO_NEW

    # 1. Build type → new_module index
    type_to_mod = {}
    for rs in sorted(EMCORE.glob("*.rs")):
        if rs.name == "mod.rs":
            continue
        stem = rs.stem
        content = rs.read_text()
        for m in re.finditer(
            r"pub(?:\(crate\))?\s+(?:struct|enum|trait|fn|const|static|type)\s+(\w+)",
            content,
        ):
            type_to_mod[m.group(1)] = stem

    # 2. Complete old submodule → new module mapping
    # Built from the Phase 3 move list: old_filename_stem → new_filename_stem
    OLD_SUBMOD_TO_NEW.update({
        "at_matrix": "emATMatrix",
        "border": "emBorder",
        "alignment": "emBorderAlignment",
        "button": "emButton",
        "check_box": "emCheckBox",
        "check_button": "emCheckButton",
        "clip_rects": "emClipRects",
        "clipboard": "emClipboard",
        "color": "emColor",
        "color_field": "emColorField",
        "field_panel": "emColorFieldFieldPanel",
        "x11_colors": "emColorX11Colors",
        "config_model": "emConfigModel",
        "context": "emContext",
        "core_config": "emCoreConfig",
        "core_config_panel": "emCoreConfigPanel",
        "cursor": "emCursor",
        "dialog": "emDialog",
        "engine": "emEngine",
        "error_panel": "emErrorPanel",
        "file_dialog": "emFileDialog",
        "file_model": "emFileModel",
        "file_panel": "emFilePanel",
        "file_selection_box": "emFileSelectionBox",
        "em_font": "emFontCache",
        "bitmap_font": "emFontCacheBitmapFont",
        "fp_plugin": "emFpPlugin",
        "app": "emGUIFramework",
        "group": "emGroup",
        "image": "emImage",
        "image_file_model": "emImageFile",
        "image_file_panel": "emImageFileImageFilePanel",
        "event": "emInput",
        "hotkey": "emInputHotkey",
        "state": "emInputState",
        "install_info": "emInstallInfo",
        "job": "emJob",
        "label": "emLabel",
        "linear_group": "emLinearGroup",
        "linear": "emLinearLayout",
        "list_box": "emListBox",
        "look": "emLook",
        "mini_ipc": "emMiniIpc",
        "model_base": "emModel",
        "pack_group": "emPackGroup",
        "pack": "emPackLayout",
        "painter": "emPainter",
        "draw_list": "emPainterDrawList",
        "interpolation": "emPainterInterpolation",
        "scanline": "emPainterScanline",
        "scanline_avx2": "emPainterScanlineAvx2",
        "scanline_tool": "emPainterScanlineTool",
        "behavior": "emPanel",
        "ctx": "emPanelCtx",
        "tree": "emPanelTree",
        "pri_sched_agent": "emPriSchedAgent",
        "process": "emProcess",
        "radio_box": "emRadioBox",
        "radio_button": "emRadioButton",
        "raster_group": "emRasterGroup",
        "raster": "emRasterLayout",
        "em_rec": "emRec",
        "rec_types": "emRecRecTypes",
        "record": "emRecRecord",
        "rec_file_model": "emRecFileModel",
        "thread_pool": "emRenderThreadPool",
        "resource_cache": "emRes",
        "tga": "emResTga",
        "scalar_field": "emScalarField",
        "core": "emScheduler",
        "screen": "emScreen",
        "signal": "emSignal",
        "sig_model": "emSigModel",
        "dlog": "emStd1",
        "checksum": "emStd2",
        "stroke": "emStroke",
        "stroke_end": "emStrokeEnd",
        "sub_view_panel": "emSubViewPanel",
        "text_field": "emTextField",
        "texture": "emTexture",
        "tiling": "emTiling",
        "timer": "emTimer",
        "tunnel": "emTunnel",
        "watched_var": "emVarModel",
        "var_sig_model": "emVarSigModel",
        "view": "emView",
        "animator": "emViewAnimator",
        "input_filter": "emViewInputFilter",
        "software_compositor": "emViewRenderer",
        "compositor": "emViewRendererCompositor",
        "tile_cache": "emViewRendererTileCache",
        "zui_window": "emWindow",
        "platform": "emWindowPlatform",
        "state_saver": "emWindowStateSaver",
        "splitter": "emSplitter",
        "toolkit_images": "toolkit_images",
        "rect": "rect",
        "fixed": "fixed",
    })

    return type_to_mod


def fix_file(path: Path, type_to_mod: dict):
    """Fix all imports in a single file."""
    content = path.read_text()
    original = content

    # Pattern 1: `use crate::{old_module}::{submod}::{stuff}`
    # e.g., `use crate::foundation::color::Color` → `use crate::emCore::emColor::Color`
    # But also: `use crate::foundation::{Color, Image}` → needs to resolve each type

    # Pattern 2: `crate::{old_module}::{Type}::CONST` (inline)
    # e.g., `crate::foundation::Color::BLACK` → `crate::emCore::emColor::Color::BLACK`

    # Strategy: find all `crate::{old_module}::` references and rewrite them

    for old_mod in OLD_MODULES:
        # Handle `use crate::{old}::X` where X is a type name
        # And `use crate::{old}::{X, Y, Z}`
        def rewrite_use(m):
            rest = m.group(1)
            # Could be a single name or a brace group
            if rest.startswith("{"):
                # Multi-import: {Foo, Bar, Baz}
                inner = rest[1:rest.index("}")]
                items = [x.strip() for x in inner.split(",") if x.strip()]
                # Group by new module
                by_mod = defaultdict(list)
                for item in items:
                    # Handle renamed imports: Foo as Bar
                    name = item.split(" as ")[0].strip()
                    new_mod = type_to_mod.get(name)
                    if new_mod:
                        by_mod[new_mod].append(item)
                    else:
                        by_mod["UNKNOWN"].append(item)

                if len(by_mod) == 1 and "UNKNOWN" not in by_mod:
                    mod_name = list(by_mod.keys())[0]
                    items_str = ", ".join(list(by_mod.values())[0])
                    return f"use crate::emCore::{mod_name}::{{{items_str}}}"
                else:
                    # Multiple modules or unknown — generate multiple use lines
                    lines = []
                    for mod_name, mod_items in sorted(by_mod.items()):
                        if mod_name == "UNKNOWN":
                            for item in mod_items:
                                lines.append(f"use crate::emCore::{item}")
                        elif len(mod_items) == 1:
                            lines.append(f"use crate::emCore::{mod_name}::{mod_items[0]}")
                        else:
                            items_str = ", ".join(mod_items)
                            lines.append(f"use crate::emCore::{mod_name}::{{{items_str}}}")
                    return ";\n".join(lines)
            else:
                # Single import: Foo or foo::Bar
                first = rest.split("::")[0]
                remaining = "::".join(rest.split("::")[1:])

                # Check if first is a type name
                new_mod = type_to_mod.get(first)
                if new_mod:
                    if remaining:
                        return f"use crate::emCore::{new_mod}::{first}::{remaining}"
                    else:
                        return f"use crate::emCore::{new_mod}::{first}"

                # Check if first is an old submodule name
                new_submod = OLD_SUBMOD_TO_NEW.get(first)
                if new_submod:
                    if remaining:
                        return f"use crate::emCore::{new_submod}::{remaining}"
                    else:
                        return f"use crate::emCore::{new_submod}"

                # Unknown — just replace the old module prefix
                return f"use crate::emCore::{rest}"

        content = re.sub(
            rf"use crate::{old_mod}::(.+?)(?=;|\n)",
            rewrite_use,
            content,
        )

        # Handle inline full-path references: crate::{old_mod}::Type::stuff
        def rewrite_inline(m):
            rest = m.group(1)
            first = rest.split("::")[0]
            remaining = "::".join(rest.split("::")[1:])

            new_mod = type_to_mod.get(first)
            if new_mod:
                if remaining:
                    return f"crate::emCore::{new_mod}::{first}::{remaining}"
                else:
                    return f"crate::emCore::{new_mod}::{first}"

            new_submod = OLD_SUBMOD_TO_NEW.get(first)
            if new_submod:
                if remaining:
                    return f"crate::emCore::{new_submod}::{remaining}"
                else:
                    return f"crate::emCore::{new_submod}"

            return f"crate::emCore::{rest}"

        content = re.sub(
            rf"crate::{old_mod}::([\w:]+)",
            rewrite_inline,
            content,
        )

    # Fix super:: references within emCore files
    if "emCore" in str(path):
        # super:: in emCore files means the crate root, which is wrong
        # Most super::old_submod::Type patterns need to become sibling module refs

        def rewrite_super(m):
            rest = m.group(1)
            first = rest.split("::")[0]
            remaining = "::".join(rest.split("::")[1:])

            # Check if first is an old submodule name (e.g., "stroke", "texture")
            new_submod = OLD_SUBMOD_TO_NEW.get(first)
            if new_submod:
                if remaining:
                    return f"super::{new_submod}::{remaining}"
                else:
                    return f"super::{new_submod}"

            # Check if it's a type name
            new_mod = type_to_mod.get(first)
            if new_mod and new_mod != path.stem:
                if remaining:
                    return f"super::{new_mod}::{first}::{remaining}"
                else:
                    return f"super::{new_mod}::{first}"

            # Leave as-is if we can't resolve
            return m.group(0)

        content = re.sub(
            r"(?<!\w)super::([\w:]+)",
            rewrite_super,
            content,
        )

    if content != original:
        path.write_text(content)
        return True
    return False


def main():
    print("=== Fixing imports ===")
    type_to_mod = build_indices()
    print(f"Type index: {len(type_to_mod)} types")
    print(f"Submodule index: {len(OLD_SUBMOD_TO_NEW)} old→new mappings")

    changed = 0

    # Fix emCore files
    for rs in sorted(EMCORE.rglob("*.rs")):
        if fix_file(rs, type_to_mod):
            changed += 1

    # Fix debug files
    for rs in sorted((SRC / "debug").rglob("*.rs")):
        if fix_file(rs, type_to_mod):
            changed += 1

    # Fix sosumi-7
    sosumi_src = ZUICCHINI.parent / "sosumi-7" / "src"
    if sosumi_src.exists():
        for rs in sorted(sosumi_src.rglob("*.rs")):
            if fix_file(rs, type_to_mod):
                changed += 1

    # Fix test files
    tests_dir = ZUICCHINI / "tests"
    if tests_dir.exists():
        for rs in sorted(tests_dir.rglob("*.rs")):
            if fix_file(rs, type_to_mod):
                changed += 1

    # Fix examples
    examples_dir = ZUICCHINI / "examples"
    if examples_dir.exists():
        for rs in sorted(examples_dir.rglob("*.rs")):
            if fix_file(rs, type_to_mod):
                changed += 1

    print(f"Fixed imports in {changed} files")


if __name__ == "__main__":
    main()
