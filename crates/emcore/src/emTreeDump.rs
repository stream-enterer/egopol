//! Port of C++ `emTreeDump` package (`src/emTreeDump/emTreeDumpUtil.cpp`).
//!
//! Produces an `emTreeDumpRec`-faithful emRec serialization of the running
//! object graph. Used by the `td!` cheat and by the future `emCtrlSocket`
//! `dump` command.
//!
//! Schema matches C++ `emTreeDumpRec` field names and per-type visual style
//! constants (Frame / BgColor / FgColor) so a future port of
//! `emTreeDumpFilePanel` can consume the same file.

#![allow(non_snake_case)]

use crate::emPanelTree::{PanelId, PanelTree};
use crate::emRecParser::{RecStruct, RecValue};

/// C++ `emTreeDumpRec::FrameType` (include/emTreeDump/emTreeDumpRec.h).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Frame {
    None = 0,
    Rectangle = 1,
    RoundRect = 2,
    Ellipse = 3,
    Hexagon = 4,
}

impl Frame {
    pub fn as_str(self) -> &'static str {
        match self {
            Frame::None => "FRAME_NONE",
            Frame::Rectangle => "FRAME_RECTANGLE",
            Frame::RoundRect => "FRAME_ROUND_RECT",
            Frame::Ellipse => "FRAME_ELLIPSE",
            Frame::Hexagon => "FRAME_HEXAGON",
        }
    }
}

/// Per-object visual style (Frame + BgColor + FgColor). Color constants
/// come from the C++ `emTreeDumpFromObject` cascade in
/// `src/emTreeDump/emTreeDumpUtil.cpp`. Colors are packed 0xRRGGBB (alpha
/// dropped — emTreeDumpRec uses 24-bit color).
pub struct VisualStyle {
    pub frame: Frame,
    pub bg: i32,
    pub fg: i32,
}

impl VisualStyle {
    pub fn engine() -> Self {
        Self { frame: Frame::Rectangle, bg: 0x000000, fg: 0xEEEEEE }
    }
    pub fn context(_is_root: bool) -> Self {
        // C++ uses the same color for root and child context; is_root
        // affects only the Title string (handled at call site).
        Self { frame: Frame::Ellipse, bg: 0x777777, fg: 0xEEEEEE }
    }
    pub fn view(focused: bool) -> Self {
        let fg = if focused { 0xEEEE44 } else { 0xEEEEEE };
        Self { frame: Frame::RoundRect, bg: 0x448888, fg }
    }
    pub fn window() -> Self {
        // Window branch overlays the view branch in C++; frame stays
        // ROUND_RECT (from view), only Bg is overridden.
        Self { frame: Frame::RoundRect, bg: 0x222288, fg: 0xEEEEEE }
    }
    pub fn panel(
        viewed: bool,
        in_viewed_path: bool,
        in_focused_path: bool,
        in_active_path: bool,
    ) -> Self {
        let bg = if viewed {
            0x338833
        } else if in_viewed_path {
            0x225522
        } else {
            0x445544
        };
        let fg = if in_focused_path {
            0xEEEE44
        } else if in_active_path {
            0xEEEE88
        } else {
            0xEEEEEE
        };
        Self { frame: Frame::Rectangle, bg, fg }
    }
    pub fn model() -> Self {
        Self { frame: Frame::Hexagon, bg: 0x440000, fg: 0xBBBBBB }
    }
    pub fn file_model() -> Self {
        Self { frame: Frame::Hexagon, bg: 0x440033, fg: 0xBBBBBB }
    }
}

/// Construct an `emTreeDumpRec`-shaped `RecStruct` with Frame / BgColor /
/// FgColor / Title / Text populated, plus empty `Commands` and `Files`
/// arrays. `Children` is NOT inserted here — callers must call
/// `set_children` explicitly (even with an empty Vec) to populate it.
///
/// Commands and Files are always empty in this port (see spec §(A)
/// Schema — keep in mind for future emTreeDumpFilePanel port); Children
/// must be added by the caller via `set_children`.
pub fn empty_rec(title: String, text: String, style: VisualStyle) -> RecStruct {
    let mut rec = RecStruct::new();
    rec.set_ident("Frame", style.frame.as_str());
    rec.set_int("BgColor", style.bg);
    rec.set_int("FgColor", style.fg);
    rec.set_str("Title", &title);
    rec.set_str("Text", &text);
    // Empty Commands and Files — Children is deferred to `set_children`.
    rec.SetValue("Commands", RecValue::Array(Vec::new()));
    rec.SetValue("Files", RecValue::Array(Vec::new()));
    rec
}

/// Sets the Children field of `rec`. Must be called exactly once per rec;
/// `empty_rec` does not pre-populate Children. Callers that have no
/// children should still call with an empty Vec for schema completeness.
pub fn set_children(rec: &mut RecStruct, children: Vec<RecValue>) {
    rec.SetValue("Children", RecValue::Array(children));
}

/// Full emPanel branch. Mirrors C++ `emTreeDumpFromObject`'s emPanel cascade
/// at `src/emTreeDump/emTreeDumpUtil.cpp:246-315`. Appends engine + context
/// fields first (C++ order), then emPanel fields, then recurses into
/// children.
///
/// Arguments:
///   `tree`:            mutable — needed for `take_behavior`/`put_behavior`.
///   `id`:              panel to dump.
///   `current_frame`:   view's current frame counter (for `LastPaintFrame` output).
///   `focused_id`:      id of the currently focused panel, or `None`.
///   `view_home_w` / `view_home_h`: view's home dimensions (for
///                      `GetUpdatePriority` / `GetMemoryLimit`).
///   `window_focused`:  the view's parent window's focused flag.
///
/// Visibility is `pub` rather than `pub(crate)` pending wiring to a real
/// consumer (the `td!` cheat / `emCtrlSocket` `dump`). Downgrading now
/// trips `dead_code` under `-D warnings` because all current callers are
/// `#[cfg(test)]`.
///
/// Note: C++ emTreeDumpFromObject emits `Engine Priority` in the
/// emEngine branch which applies to every emPanel via inheritance.
/// The Rust port does not unify panels with engines, so this field is
/// omitted rather than rendered as a misleading placeholder. If a
/// future task wires panel-as-engine, add the line back here.
#[allow(clippy::too_many_arguments)]
pub fn dump_panel(
    tree: &mut PanelTree,
    id: PanelId,
    current_frame: u64,
    focused_id: Option<PanelId>,
    view_home_w: f64,
    view_home_h: f64,
    window_focused: bool,
) -> RecStruct {
    // --- Derive all read-only state first (avoids borrow conflicts) ---

    // in_focused_path: walk parent chain from focused_id, check membership.
    let in_focused_path = match focused_id {
        Some(fid) => {
            let mut cur = Some(fid);
            let mut found = false;
            while let Some(c) = cur {
                if c == id {
                    found = true;
                    break;
                }
                cur = tree.panels[c].parent;
            }
            found
        }
        None => false,
    };

    let height = tree.get_height(id);
    let (ex, ey, ew, eh) = tree.GetEssenceRect(id);
    let update_priority = tree.GetUpdatePriority(id, view_home_w, view_home_h, window_focused);
    let memory_limit = tree.GetMemoryLimit(id, view_home_w, view_home_h, 2_048_000_000, None);

    // take_behavior to extract subtype fields and type_name.
    let (type_name, subtype_pairs) = if let Some(behavior) = tree.take_behavior(id) {
        let n = behavior.type_name().to_string();
        let p = behavior.dump_state();
        tree.put_behavior(id, behavior);
        (n, p)
    } else {
        ("(no behavior)".to_string(), Vec::new())
    };

    let panel_title = tree.get_title(id);
    // Snapshot PanelData fields we need after this point.
    let data = &tree.panels[id];
    let name = data.name.clone();
    let layout = data.layout_rect;
    let is_viewed = data.viewed;
    let is_in_viewed_path = data.in_viewed_path;
    let in_active_path = data.in_active_path;
    let is_active = data.is_active;
    let focusable = data.focusable;
    let enable_switch = data.enable_switch;
    let enabled = data.enabled;
    let paint_count = data.paint_count;
    let last_paint_frame = data.last_paint_frame;
    let viewed_xywh = if is_viewed {
        Some((data.viewed_x, data.viewed_y, data.viewed_width, data.viewed_height))
    } else {
        None
    };
    let clip_x1y1x2y2 = if is_viewed {
        Some((data.clip_x, data.clip_y, data.clip_x + data.clip_w, data.clip_y + data.clip_h))
    } else {
        None
    };
    let is_focused = focused_id == Some(id);

    // --- Build the Text body (C++ emTreeDumpUtil.cpp:256-307 order) ---

    let mut text = String::new();

    // Name, Title, Layout/Height/Essence, Viewed flags, Clip, Enable, etc.
    text.push_str(&format!("\nName: {}", name));
    text.push_str(&format!("\nTitle: {}", panel_title));
    text.push_str(&format!(
        "\nLayout XYWH: {}",
        fmt_xywh(layout.x, layout.y, layout.w, layout.h)
    ));
    text.push_str(&format!("\nHeight: {}", fmt_g(height)));
    text.push_str(&format!("\nEssence XYWH: {}", fmt_xywh(ex, ey, ew, eh)));
    text.push_str(&format!("\nViewed: {}", yes_no(is_viewed)));
    text.push_str(&format!("\nInViewedPath: {}", yes_no(is_in_viewed_path)));
    text.push_str("\nViewed XYWH: ");
    text.push_str(&match viewed_xywh {
        Some((x, y, w, h)) => fmt_xywh(x, y, w, h),
        None => "-".to_string(),
    });
    text.push_str("\nClip X1Y1X2Y2: ");
    text.push_str(&match clip_x1y1x2y2 {
        Some((x1, y1, x2, y2)) => fmt_xywh(x1, y1, x2, y2),
        None => "-".to_string(),
    });
    text.push_str(&format!("\nEnableSwitch: {}", yes_no(enable_switch)));
    text.push_str(&format!("\nEnabled: {}", yes_no(enabled)));
    text.push_str(&format!("\nFocusable: {}", yes_no(focusable)));
    text.push_str(&format!("\nActive: {}", yes_no(is_active)));
    text.push_str(&format!("\nInActivePath: {}", yes_no(in_active_path)));
    text.push_str(&format!("\nFocused: {}", yes_no(is_focused)));
    text.push_str(&format!("\nInFocusedPath: {}", yes_no(in_focused_path)));
    text.push_str(&format!("\nUpdate Priority: {}", fmt_g(update_priority)));
    text.push_str(&format!("\nMemory Limit: {}", memory_limit));

    // RUST_ONLY: (language-forced-utility) paint-counter fields — not
    // present in C++ dump. C++ uses gdb for per-panel paint inspection;
    // the Rust port lacks an equivalent live-inspection path, so paint
    // attribution is baked into the data model and surfaced here.
    text.push_str(&format!("\nPaintCount: {}", paint_count));
    text.push_str(&format!(
        "\nLastPaintFrame: {} (current: {})",
        last_paint_frame, current_frame
    ));

    // Subtype fields (PanelBehavior::dump_state).
    for (label, value) in &subtype_pairs {
        text.push_str(&format!("\n{}: {}", label, value));
    }

    // --- Compose rec ---

    let title = format!("Panel:\n{}\n\"{}\"", type_name, name);
    let style = VisualStyle::panel(is_viewed, is_in_viewed_path, in_focused_path, in_active_path);
    let mut rec = empty_rec(title, text, style);

    // --- Recurse into children ---

    let child_ids: Vec<PanelId> = tree.children(id).collect();
    let mut children: Vec<RecValue> = Vec::with_capacity(child_ids.len());
    for child_id in child_ids {
        let child_rec = dump_panel(
            tree,
            child_id,
            current_frame,
            focused_id,
            view_home_w,
            view_home_h,
            window_focused,
        );
        children.push(RecValue::Struct(child_rec));
    }
    set_children(&mut rec, children);

    rec
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// Format a single f64 in C++ `%.9G` style: 9 significant digits, with
/// scientific notation when the exponent is `< -4` or `>= 9`, trailing
/// zeros stripped, and a trailing `.` stripped when the mantissa is
/// integer. Matches `printf("%.9G", v)` for the contracts required by
/// the tree dump; exponent field uses `E[+-]dd` (2-digit minimum) as
/// printf does on glibc.
fn fmt_g(v: f64) -> String {
    const PRECISION: i32 = 9;

    if v == 0.0 {
        // Handles +0.0 and -0.0 uniformly; C's %G prints "0" here.
        return "0".to_string();
    }
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }

    let exp = v.abs().log10().floor() as i32;

    if !(-4..PRECISION).contains(&exp) {
        // Scientific. Rust `{:E}` precision = digits after decimal; %G
        // precision = total significant digits, so subtract 1.
        let raw = format!("{:.*E}", (PRECISION - 1) as usize, v);
        // raw looks like "1.230000000E2" or "-1.23E-5".
        let (mantissa, exp_str) = raw.split_once('E').expect("format!E always contains E");
        // Strip trailing zeros from the mantissa; then strip a trailing
        // '.' if the mantissa collapses to an integer.
        let mut m = mantissa.trim_end_matches('0').trim_end_matches('.').to_string();
        if m.is_empty() || m == "-" {
            m.push('0');
        }
        // Normalize exponent: C printf emits E[+-]dd with at least 2
        // digits. Rust's {:E} emits e.g. "E2" or "E-5" (no sign for
        // non-negative, no zero-pad).
        let (sign, digits) = if let Some(rest) = exp_str.strip_prefix('-') {
            ('-', rest)
        } else if let Some(rest) = exp_str.strip_prefix('+') {
            ('+', rest)
        } else {
            ('+', exp_str)
        };
        let padded = if digits.len() < 2 {
            format!("0{}", digits)
        } else {
            digits.to_string()
        };
        format!("{}E{}{}", m, sign, padded)
    } else {
        // Fixed. digits-after-decimal = precision - 1 - exp (clamped ≥ 0).
        let digits_after = (PRECISION - 1 - exp).max(0) as usize;
        let raw = format!("{:.*}", digits_after, v);
        if raw.contains('.') {
            let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
            if trimmed.is_empty() || trimmed == "-" {
                "0".to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            raw
        }
    }
}

fn fmt_xywh(x: f64, y: f64, w: f64, h: f64) -> String {
    format!("{}, {}, {}, {}", fmt_g(x), fmt_g(y), fmt_g(w), fmt_g(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_as_str_matches_cpp_names() {
        assert_eq!(Frame::None.as_str(), "FRAME_NONE");
        assert_eq!(Frame::Rectangle.as_str(), "FRAME_RECTANGLE");
        assert_eq!(Frame::RoundRect.as_str(), "FRAME_ROUND_RECT");
        assert_eq!(Frame::Ellipse.as_str(), "FRAME_ELLIPSE");
        assert_eq!(Frame::Hexagon.as_str(), "FRAME_HEXAGON");
    }

    #[test]
    fn empty_rec_has_all_top_level_fields() {
        let rec = empty_rec("t".into(), "txt".into(), VisualStyle::engine());
        assert!(rec.get_ident("Frame").is_some());
        assert!(rec.get_int("BgColor").is_some());
        assert!(rec.get_int("FgColor").is_some());
        assert_eq!(rec.get_str("Title"), Some("t"));
        assert_eq!(rec.get_str("Text"), Some("txt"));
        assert!(rec.get_array("Commands").is_some());
        assert!(rec.get_array("Files").is_some());
        // Children is NOT populated by empty_rec — callers must call
        // set_children explicitly.
        assert!(rec.get_array("Children").is_none());
    }

    #[test]
    fn set_children_populates_children_field() {
        let mut rec = empty_rec("t".into(), "".into(), VisualStyle::engine());
        assert!(rec.get_array("Children").is_none());
        set_children(&mut rec, Vec::new());
        let arr = rec.get_array("Children").expect("Children exists");
        assert!(arr.is_empty());
    }

    #[test]
    fn visual_style_panel_viewed_bg_matches_cpp() {
        // C++ emTreeDumpUtil.cpp:249: IsViewed() ⇒ emColor(51,136,51) = 0x338833
        let s = VisualStyle::panel(true, false, false, false);
        assert_eq!(s.bg, 0x338833);
    }

    #[test]
    fn visual_style_panel_focused_fg_matches_cpp() {
        // C++ emTreeDumpUtil.cpp:252: IsInFocusedPath() ⇒ emColor(238,238,68) = 0xEEEE44
        let s = VisualStyle::panel(false, false, true, false);
        assert_eq!(s.fg, 0xEEEE44);
    }

    #[test]
    fn visual_style_context_matches_cpp() {
        let s = VisualStyle::context(true);
        assert_eq!(s.frame, Frame::Ellipse);
        assert_eq!(s.bg, 0x777777);
        assert_eq!(s.fg, 0xEEEEEE);
        let s2 = VisualStyle::context(false);
        assert_eq!(s2.bg, 0x777777);
    }

    #[test]
    fn visual_style_view_focused_matches_cpp() {
        let s = VisualStyle::view(true);
        assert_eq!(s.frame, Frame::RoundRect);
        assert_eq!(s.bg, 0x448888);
        assert_eq!(s.fg, 0xEEEE44);
        let s2 = VisualStyle::view(false);
        assert_eq!(s2.fg, 0xEEEEEE);
    }

    #[test]
    fn visual_style_window_matches_cpp() {
        let s = VisualStyle::window();
        assert_eq!(s.frame, Frame::RoundRect);
        assert_eq!(s.bg, 0x222288);
        assert_eq!(s.fg, 0xEEEEEE);
    }

    #[test]
    fn visual_style_model_matches_cpp() {
        let s = VisualStyle::model();
        assert_eq!(s.frame, Frame::Hexagon);
        assert_eq!(s.bg, 0x440000);
        assert_eq!(s.fg, 0xBBBBBB);
    }

    #[test]
    fn visual_style_file_model_matches_cpp() {
        let s = VisualStyle::file_model();
        assert_eq!(s.frame, Frame::Hexagon);
        assert_eq!(s.bg, 0x440033);
        assert_eq!(s.fg, 0xBBBBBB);
    }

    #[test]
    fn visual_style_panel_in_viewed_path_bg_matches_cpp() {
        // C++ emTreeDumpUtil.cpp: IsInViewedPath() ⇒ emColor(34,85,34) = 0x225522
        let s = VisualStyle::panel(false, true, false, false);
        assert_eq!(s.bg, 0x225522);
        // Default branch
        let s2 = VisualStyle::panel(false, false, false, false);
        assert_eq!(s2.bg, 0x445544);
        // Active-path fg
        let s3 = VisualStyle::panel(false, false, false, true);
        assert_eq!(s3.fg, 0xEEEE88);
    }

    #[test]
    fn set_children_replaces_children_array() {
        let mut rec = empty_rec("t".into(), "".into(), VisualStyle::engine());
        set_children(&mut rec, vec![RecValue::Int(7)]);
        let arr = rec.get_array("Children").expect("Children exists");
        assert_eq!(arr.len(), 1);
        assert!(matches!(arr[0], RecValue::Int(7)));
    }

    #[test]
    fn fmt_g_matches_c_percent_g() {
        // Expected values verified against `printf("%.9G", x)` on glibc.
        assert_eq!(fmt_g(0.0), "0");
        assert_eq!(fmt_g(1.0), "1");
        assert_eq!(fmt_g(1.5), "1.5");
        assert_eq!(fmt_g(123456789.0), "123456789");
        assert_eq!(fmt_g(0.5), "0.5");
        assert_eq!(fmt_g(-1.5), "-1.5");
        assert_eq!(fmt_g(0.0001), "0.0001");
        // < 1e-4 → scientific
        assert_eq!(fmt_g(0.00001), "1E-05");
        // >= 1e9 → scientific
        assert_eq!(fmt_g(1e9), "1E+09");
        // 9 significant digits (C printf rounds half-to-even here).
        assert_eq!(fmt_g(1.2345678901), "1.23456789");
    }

    // --- dump_panel tests ---

    use crate::emPanel::PanelBehavior;

    /// Minimal no-op behavior for exercising the `take_behavior` path in
    /// `dump_panel`. All trait methods use defaults.
    struct NoopBehavior;
    impl PanelBehavior for NoopBehavior {}

    #[test]
    fn dump_panel_leaf_has_all_labels() {
        let mut tree = PanelTree::new();
        let root = tree.create_root_deferred_view("root");
        tree.put_behavior(root, Box::new(NoopBehavior));

        let rec = dump_panel(&mut tree, root, 0, None, 1.0, 1.0, false);

        let title = rec.get_str("Title").expect("Title exists").to_string();
        let text = rec.get_str("Text").expect("Text exists").to_string();

        assert!(title.starts_with("Panel:\n"), "Title: {}", title);
        assert!(title.contains("\"root\""), "Title: {}", title);

        assert!(!text.contains("Engine Priority"));

        for label in [
            "Name:",
            "Title:",
            "Layout XYWH",
            "Height:",
            "Essence XYWH",
            "Viewed: no",
            "InViewedPath: no",
            "Viewed XYWH: -",
            "Clip X1Y1X2Y2: -",
            "EnableSwitch",
            "Enabled",
            "Focusable",
            "Active",
            "InActivePath",
            "Focused: no",
            "InFocusedPath: no",
            "Update Priority",
            "Memory Limit:",
            "PaintCount: 0",
            "LastPaintFrame: 0 (current: 0)",
        ] {
            assert!(text.contains(label), "Text missing label `{}`:\n{}", label, text);
        }

        let children = rec.get_array("Children").expect("Children exists");
        assert!(children.is_empty());
    }

    #[test]
    fn dump_panel_recurses_into_children() {
        let mut tree = PanelTree::new();
        let root = tree.create_root_deferred_view("root");
        tree.put_behavior(root, Box::new(NoopBehavior));
        let c1 = tree.create_child(root, "alpha", None);
        tree.put_behavior(c1, Box::new(NoopBehavior));
        let c2 = tree.create_child(root, "beta", None);
        tree.put_behavior(c2, Box::new(NoopBehavior));

        let rec = dump_panel(&mut tree, root, 0, None, 1.0, 1.0, false);

        let children = rec.get_array("Children").expect("Children exists");
        assert_eq!(children.len(), 2);

        // Each child rec's Text must contain its own Name: line.
        let mut names: Vec<String> = Vec::new();
        for child in children {
            let s = match child {
                RecValue::Struct(s) => s,
                other => panic!("child is not Struct: {:?}", other),
            };
            let text = s.get_str("Text").expect("child Text exists");
            if text.contains("\nName: alpha") {
                names.push("alpha".into());
            }
            if text.contains("\nName: beta") {
                names.push("beta".into());
            }
        }
        names.sort();
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn dump_panel_appends_subtype_dump_state_pairs() {
        struct LoadingBehavior;
        impl PanelBehavior for LoadingBehavior {
            fn dump_state(&self) -> Vec<(&'static str, String)> {
                vec![
                    ("loading_pct", "42".to_string()),
                    ("loading_done", "false".to_string()),
                ]
            }
        }

        let mut tree = PanelTree::new();
        let root = tree.create_root_deferred_view("root");
        tree.put_behavior(root, Box::new(LoadingBehavior));

        let rec = dump_panel(&mut tree, root, 0, None, 1.0, 1.0, false);

        let text = rec.get_str("Text").expect("Text exists").to_string();
        assert!(
            text.contains("\nloading_pct: 42"),
            "Text missing loading_pct pair:\n{}",
            text
        );
        assert!(
            text.contains("\nloading_done: false"),
            "Text missing loading_done pair:\n{}",
            text
        );
        // Insertion order: loading_pct must come before loading_done.
        let idx_pct = text.find("\nloading_pct:").expect("loading_pct present");
        let idx_done = text.find("\nloading_done:").expect("loading_done present");
        assert!(
            idx_pct < idx_done,
            "dump_state Vec order not preserved: pct@{} done@{}",
            idx_pct,
            idx_done
        );
    }
}
