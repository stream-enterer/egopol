#!/usr/bin/env python3
"""Diff C++ vs Rust panel trees from JSONL dumps.

Usage: python3 scripts/diff_panel_tree.py <test_name>
  Reads: target/golden-divergence/<test_name>.cpp_tree.jsonl
         target/golden-divergence/<test_name>.rust_tree.jsonl
"""
import json, sys

def load_tree(path):
    panels = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or not line.startswith('{'):
                continue
            p = json.loads(line)
            panels[p['path']] = p
    return panels

def main():
    name = sys.argv[1] if len(sys.argv) > 1 else "tktest_1x"
    base = "crates/eaglemode/target/golden-divergence"
    cpp = load_tree(f"{base}/{name}.cpp_tree.jsonl")
    rust = load_tree(f"{base}/{name}.rust_tree.jsonl")

    cpp_paths = set(cpp.keys())
    rust_paths = set(rust.keys())

    missing_in_rust = sorted(cpp_paths - rust_paths)
    extra_in_rust = sorted(rust_paths - cpp_paths)
    common = sorted(cpp_paths & rust_paths)

    print(f"C++ panels: {len(cpp)}, Rust panels: {len(rust)}")
    print(f"Common: {len(common)}, Missing in Rust: {len(missing_in_rust)}, Extra in Rust: {len(extra_in_rust)}")

    if missing_in_rust:
        print(f"\n=== MISSING IN RUST ({len(missing_in_rust)}) ===")
        # Group by parent path
        by_parent = {}
        for path in missing_in_rust:
            parts = path.rsplit(':', 1)
            parent = parts[0] if len(parts) > 1 else "<root>"
            by_parent.setdefault(parent, []).append(path)
        for parent in sorted(by_parent):
            print(f"\n  Under {parent}:")
            for p in by_parent[parent]:
                c = cpp[p]
                print(f"    {p}  depth={c['depth']} children={c['children']} ae={c['ae_expanded']} viewed={c['viewed']}")

    if extra_in_rust:
        print(f"\n=== EXTRA IN RUST ({len(extra_in_rust)}) ===")
        for p in extra_in_rust:
            r = rust[p]
            print(f"  {p}  depth={r['depth']} children={r['children']}")

    if common:
        diffs = []
        for path in common:
            c, r = cpp[path], rust[path]
            dd = []
            if c['children'] != r['children']:
                dd.append(f"children: C++={c['children']} Rust={r['children']}")
            if c['ae_expanded'] != r['ae_expanded']:
                dd.append(f"ae_expanded: C++={c['ae_expanded']} Rust={r['ae_expanded']}")
            if c['viewed'] != r['viewed']:
                dd.append(f"viewed: C++={c['viewed']} Rust={r['viewed']}")
            if dd:
                diffs.append((path, dd))
        if diffs:
            print(f"\n=== COMMON PANELS WITH DIFFERENCES ({len(diffs)}) ===")
            for path, dd in diffs:
                print(f"  {path}")
                for d in dd:
                    print(f"    {d}")

if __name__ == '__main__':
    main()
