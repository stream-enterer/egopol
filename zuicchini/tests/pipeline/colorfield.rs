//! Systematic interaction test for ColorField in expanded state at 1x zoom,
//! driven through the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies that auto-expansion creates the expected child panel structure
//! (RasterLayout container with ScalarField sliders for R, G, B, A, H, S, V
//! and a TextField for color name/hex), and that the expansion data is
//! correctly initialized from the widget's color.


use std::rc::Rc;

use zuicchini::foundation::Color;
use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelCtx, PanelState};
use zuicchini::render::Painter;
use zuicchini::widget::{ColorField, Look};

use super::support::pipeline::PipelineTestHarness;

/// PanelBehavior wrapper for ColorField so it can be installed into the
/// panel tree. Delegates paint/input/layout_children to the underlying widget.
struct ColorFieldBehavior {
    color_field: ColorField,
}

impl ColorFieldBehavior {
    fn new(look: Rc<Look>) -> Self {
        let mut cf = ColorField::new(look);
        cf.set_editable(true);
        cf.set_alpha_enabled(true);
        Self { color_field: cf }
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color_field.set_color(color);
        self
    }
}

impl PanelBehavior for ColorFieldBehavior {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        self.color_field.paint(painter, w, h);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        self.color_field.input(event, state, input_state)
    }

    fn get_cursor(&self) -> Cursor {
        Cursor::Normal
    }

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        self.color_field.layout_children(ctx, rect.w, rect.h);
    }
}

// ---------------------------------------------------------------------------
// Helper: collect child panel names under a given parent
// ---------------------------------------------------------------------------
fn child_names(h: &PipelineTestHarness, parent: zuicchini::panel::PanelId) -> Vec<String> {
    h.tree
        .children(parent)
        .filter_map(|id| h.tree.name(id).map(|n| n.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Test: expansion structure at 1x zoom
// ---------------------------------------------------------------------------

/// Verify that expanding a ColorField at 16x zoom creates the expected child
/// panel hierarchy:
///
/// ```text
/// color_field
///   emColorField::InnerStuff  (RasterLayout container)
///     r   (ScalarField - Red)
///     g   (ScalarField - Green)
///     b   (ScalarField - Blue)
///     a   (ScalarField - Alpha)
///     h   (ScalarField - Hue)
///     s   (ScalarField - Saturation)
///     v   (ScalarField - Value/brightness)
///     n   (TextField   - Name/hex)
/// ```
#[test]
fn colorfield_expanded_has_correct_child_structure() {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let behavior = ColorFieldBehavior::new(look);
    let panel_id = h.add_panel_with(root, "color_field", Box::new(behavior));

    // Initial tick for layout.
    h.tick();

    // Trigger auto-expansion at 16x zoom (well above the expansion threshold).
    h.expand_to(16.0);

    // The panel should be auto-expanded.
    assert!(
        h.is_expanded(panel_id),
        "ColorField panel should be auto-expanded at 16x zoom"
    );

    // The ColorField should have exactly 1 direct child: the RasterLayout container.
    let direct_children = child_names(&h, panel_id);
    assert_eq!(
        direct_children.len(),
        1,
        "Expanded ColorField should have 1 direct child (RasterLayout container), \
         but found {}: {:?}",
        direct_children.len(),
        direct_children
    );
    assert_eq!(
        direct_children[0], "emColorField::InnerStuff",
        "Direct child should be the RasterLayout container 'emColorField::InnerStuff'"
    );

    // Find the RasterLayout container and verify its children.
    let layout_id = h
        .tree
        .children(panel_id)
        .next()
        .expect("should have a child");

    let slider_names = child_names(&h, layout_id);
    assert_eq!(
        slider_names,
        vec!["r", "g", "b", "a", "h", "s", "v", "n"],
        "RasterLayout container should have 8 children: \
         r, g, b, a (RGBA), h, s, v (HSV), n (Name). Got: {:?}",
        slider_names
    );
}

// ---------------------------------------------------------------------------
// Test: expansion data matches the initial color
// ---------------------------------------------------------------------------

/// Verify that the expansion data (RGBA/HSV values) is correctly initialized
/// from the widget's color when auto-expansion creates the child panels.
#[test]
fn colorfield_expanded_data_matches_initial_color() {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let color = Color::rgba(100, 150, 200, 180);
    let behavior = ColorFieldBehavior::new(look).with_color(color);
    let panel_id = h.add_panel_with(root, "color_field", Box::new(behavior));

    h.tick();
    h.expand_to(16.0);

    assert!(h.is_expanded(panel_id));

    // Take the behavior to inspect the expansion data.
    let behavior = h.tree.take_behavior(panel_id).expect("behavior exists");
    let cfb = behavior
        .as_any()
        .downcast_ref::<ColorFieldBehavior>()
        .expect("should be ColorFieldBehavior");

    let exp = cfb
        .color_field
        .expansion()
        .expect("expansion data should exist after auto-expand");

    // Verify RGBA channels match the initial color.
    // C++ formula: (channel * 10000 + 127) / 255
    let expected_r = (100i64 * 10000 + 127) / 255;
    let expected_g = (150i64 * 10000 + 127) / 255;
    let expected_b = (200i64 * 10000 + 127) / 255;
    let expected_a = (180i64 * 10000 + 127) / 255;

    assert_eq!(exp.sf_red, expected_r, "Red channel mismatch");
    assert_eq!(exp.sf_green, expected_g, "Green channel mismatch");
    assert_eq!(exp.sf_blue, expected_b, "Blue channel mismatch");
    assert_eq!(exp.sf_alpha, expected_a, "Alpha channel mismatch");

    // Verify HSV values are reasonable for rgb(100, 150, 200).
    // Hue should be in the blue range (~210 degrees = 21000 in C++ units).
    assert!(
        exp.sf_hue > 18000 && exp.sf_hue < 24000,
        "Hue for rgb(100,150,200) should be ~210 degrees (21000), got {}",
        exp.sf_hue
    );
    // Saturation should be non-zero (it's a colored pixel, not grey).
    assert!(
        exp.sf_sat > 0,
        "Saturation should be > 0 for rgb(100,150,200), got {}",
        exp.sf_sat
    );
    // Value should be > 0 (it's not black).
    assert!(
        exp.sf_val > 0,
        "Value should be > 0 for rgb(100,150,200), got {}",
        exp.sf_val
    );

    // Put the behavior back.
    h.tree.put_behavior(panel_id, behavior);
}

// ---------------------------------------------------------------------------
// Test: expansion creates children for different initial colors
// ---------------------------------------------------------------------------

/// Verify that expansion works correctly with different initial colors
/// (black, white, pure red, transparent).
#[test]
fn colorfield_expanded_various_colors() {
    let test_cases: Vec<(&str, Color, Box<dyn Fn(i64, i64, i64, i64)>)> = vec![
        (
            "black",
            Color::BLACK,
            Box::new(|r, g, b, _a| {
                assert_eq!(r, 0, "Black: red should be 0");
                assert_eq!(g, 0, "Black: green should be 0");
                assert_eq!(b, 0, "Black: blue should be 0");
            }),
        ),
        (
            "white",
            Color::WHITE,
            Box::new(|r, g, b, _a| {
                assert_eq!(r, 10000, "White: red should be 10000");
                assert_eq!(g, 10000, "White: green should be 10000");
                assert_eq!(b, 10000, "White: blue should be 10000");
            }),
        ),
        (
            "pure_red",
            Color::RED,
            Box::new(|r, g, b, _a| {
                assert_eq!(r, 10000, "Red: red should be 10000");
                assert_eq!(g, 0, "Red: green should be 0");
                assert_eq!(b, 0, "Red: blue should be 0");
            }),
        ),
        (
            "transparent",
            Color::TRANSPARENT,
            Box::new(|_r, _g, _b, a| {
                assert_eq!(a, 0, "Transparent: alpha should be 0");
            }),
        ),
    ];

    for (label, color, check) in &test_cases {
        let mut h = PipelineTestHarness::new();
        let root = h.root();

        let look = Look::new();
        let behavior = ColorFieldBehavior::new(look).with_color(*color);
        let panel_id = h.add_panel_with(root, "color_field", Box::new(behavior));

        h.tick();
        h.expand_to(16.0);

        assert!(
            h.is_expanded(panel_id),
            "{label}: panel should be auto-expanded"
        );

        // Verify child structure exists.
        let child_count = h.tree.child_count(panel_id);
        assert_eq!(
            child_count, 1,
            "{label}: should have 1 direct child (RasterLayout)"
        );

        let layout_id = h.tree.children(panel_id).next().unwrap();
        let slider_count = h.tree.child_count(layout_id);
        assert_eq!(
            slider_count, 8,
            "{label}: RasterLayout should have 8 children"
        );

        // Inspect expansion data.
        let behavior = h.tree.take_behavior(panel_id).expect("behavior exists");
        let cfb = behavior
            .as_any()
            .downcast_ref::<ColorFieldBehavior>()
            .expect("should be ColorFieldBehavior");

        let exp = cfb
            .color_field
            .expansion()
            .expect("expansion data should exist");

        check(exp.sf_red, exp.sf_green, exp.sf_blue, exp.sf_alpha);

        h.tree.put_behavior(panel_id, behavior);
    }
}

// ---------------------------------------------------------------------------
// Test: child count before vs after expansion
// ---------------------------------------------------------------------------

/// Verify that the ColorField has no children when below the expansion
/// threshold, and gains children once expanded.
///
/// The default auto-expansion threshold is 150 (area). At 1x zoom the panel
/// fills the 800x600 viewport (area=480000), which already exceeds 150. To
/// test the non-expanded state we set a very high threshold so that 1x zoom
/// does NOT trigger expansion, then lower it (or zoom in) to trigger it.
#[test]
fn colorfield_no_children_before_expansion() {
    use zuicchini::panel::ViewConditionType;

    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let behavior = ColorFieldBehavior::new(look);
    let panel_id = h.add_panel_with(root, "color_field", Box::new(behavior));

    // Set a very high threshold so the panel is NOT auto-expanded at 1x.
    h.tree
        .set_auto_expansion_threshold(panel_id, 1e12, ViewConditionType::Area);
    h.tick_n(5);

    assert!(
        !h.is_expanded(panel_id),
        "ColorField should NOT be auto-expanded with threshold=1e12"
    );
    assert_eq!(
        h.tree.child_count(panel_id),
        0,
        "Non-expanded ColorField should have 0 children"
    );

    // Now lower the threshold back to default so that expansion is triggered.
    h.tree
        .set_auto_expansion_threshold(panel_id, 150.0, ViewConditionType::Area);
    h.tick_n(10);

    assert!(
        h.is_expanded(panel_id),
        "ColorField should be auto-expanded after lowering threshold"
    );
    assert!(
        h.tree.child_count(panel_id) >= 1,
        "Expanded ColorField should have at least 1 child"
    );
}

// ---------------------------------------------------------------------------
// Test: expansion name field contains hex string
// ---------------------------------------------------------------------------

/// Verify that the Name text field in the expansion is initialized with the
/// hex representation of the current color.
#[test]
fn colorfield_expanded_name_field_initialized() {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let color = Color::rgba(0xAB, 0xCD, 0xEF, 0xFF);
    let behavior = ColorFieldBehavior::new(look).with_color(color);
    let panel_id = h.add_panel_with(root, "color_field", Box::new(behavior));

    h.tick();
    h.expand_to(16.0);

    let behavior = h.tree.take_behavior(panel_id).expect("behavior exists");
    let cfb = behavior
        .as_any()
        .downcast_ref::<ColorFieldBehavior>()
        .expect("should be ColorFieldBehavior");

    let exp = cfb
        .color_field
        .expansion()
        .expect("expansion should exist");

    assert_eq!(
        exp.tf_name, "#ABCDEF",
        "Name field should be initialized with hex color string"
    );

    h.tree.put_behavior(panel_id, behavior);
}
