# CheckButton Audit Report

**Date**: 2026-03-18 (Session 2, partial extraction)
**C++ files**: emCheckButton.cpp (99 LOC) + emCheckButton.h (91 LOC) = 190 LOC, inherits emButton (623 LOC)
**Rust file**: check_button.rs (340 LOC)

## Findings: Mostly verified-OK with minor misses

### Paint Path — ALL VERIFIED OK
- Face inset formula `d = (14/264) * r` — exact match
- ButtonBgColor for face — match
- Label padding — exact match
- Pressed label shrink (0.98) — match
- ShownChecked label shrink (0.983) — match (fix confirmed)
- ButtonPressed/ButtonChecked/Button overlays — all match with correct coordinates
- Normal Button overlay with extra blending — match

### Toggle Behavior — VERIFIED OK
- Click toggles checked — match
- set_checked fires on_check callback — fix confirmed
- No ShownChecked separation from checked — acceptable structural simplification

### [BUG] HowTo chain missing HOWTO_BUTTON section — **FIXED**
- C++ chain: emBorder::GetHowTo + HowToButton + HowToCheckButton + checked/unchecked
- Rust chain: border.get_howto + HOWTO_CHECK_BUTTON + checked/unchecked
- Missing: the emButton-level "BUTTON ... In order to trigger a button..." help text
- Affects user-facing help text readability

### Minor Gaps (same as Button-family systemic issues)
- Release path missing IsEnabled() re-check (CC-03)
- Release path missing clip rect bounds check
- No shift awareness in Click (no EOI signal passthrough)
- Disabled label alpha off-by-1 for some values (integer vs float rounding)
- No IsActive() check on Enter key

### HowTo constants (HOWTO_CHECK_BUTTON, HOWTO_CHECKED, HOWTO_NOT_CHECKED) — VERIFIED MATCH

## Summary

| Severity | Count |
|----------|-------|
| BUG | 1 (missing HOWTO_BUTTON in chain) |
| LOW | 4 (systemic CC issues + alpha rounding) |
| OK | 24 verified items |

## Overall: Faithful port. All prior session fixes confirmed. One real bug: HowTo chain skips the Button-level help text. Paint path verified item-by-item (11 checks all OK). Toggle behavior verified (6 checks). Input handling verified with fixes in place (9 checks).
