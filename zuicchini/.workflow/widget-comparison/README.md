# Widget Comparison Prepass

Precomputed artifacts for LLM-driven comparison of C++ emCore widgets against their Rust zuicchini ports.

## Artifacts

| File | Purpose |
|------|---------|
| `widget-pairs.md` | Complete mapping table: C++ class → Rust module, LOC counts, fidelity layer classification |
| `priority-order.md` | Ranked comparison order based on complexity, coverage gaps, size asymmetry, and recent churn |
| `golden-coverage.md` | Which widgets have golden test coverage (render, interaction, or both) and which have gaps |
| `bug-taxonomy.md` | 10-category checklist of common C++→Rust port bugs, ordered by frequency in this codebase |
| `rosetta-stone.md` | Known-correct translation patterns the LLM should NOT flag as bugs |
| `comparison-prompt.md` | Template prompt to run per widget, with file paths and output format |

## How to use

1. Pick next widget from `priority-order.md`
2. Fill in `comparison-prompt.md` template with widget-specific values
3. Feed the LLM: C++ source + header + Rust source + filled prompt + rosetta-stone + bug-taxonomy
4. Review findings; run suggested golden tests
5. Record results in `results/{{widget_name}}.md`

## Source locations

- **C++ emCore**: `/home/ar/.local/git/eaglemode-0.96.4/src/emCore/` and `.../include/emCore/`
- **Rust zuicchini**: `/home/ar/Development/sosumi-7/zuicchini/src/widget/`
- **Golden tests**: `/home/ar/Development/sosumi-7/zuicchini/tests/golden/`
- **Golden data generator**: `/home/ar/Development/sosumi-7/zuicchini/tests/golden/gen/`
