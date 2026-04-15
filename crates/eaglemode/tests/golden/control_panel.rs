// Rust-only regression test for emMainControlPanel::LayoutChildren.
// Verifies that the panel creates the expected child tree matching C++ structure.
// No C++ golden data needed — these verify structural correctness.

use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emPanel::PanelBehavior;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelTree;

use emMain::emMainControlPanel::emMainControlPanel;

#[test]
fn control_panel_layout_children() {
    let ctx = emContext::NewRoot();
    let mut panel = emMainControlPanel::new(Rc::clone(&ctx));

    let mut tree = PanelTree::new();
    let root = tree.create_root("ctrl_root");
    // Give root a 1:1 layout so normalized coordinates are [0,1] x [0,1].
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);

    // Call LayoutChildren — this creates children AND positions them.
    {
        let mut pctx = PanelCtx::new(&mut tree, root);
        panel.LayoutChildren(&mut pctx);
    }

    // Top-level has 2 children: "general" and "bookmarks"
    // (matching C++ lMain with child 0 = general panel, child 1 = bookmarks).
    let children: Vec<_> = tree.children(root).collect();
    assert_eq!(
        children.len(),
        2,
        "Expected 2 top-level children (general + bookmarks), got {}",
        children.len()
    );

    // Both children should have non-zero layout rects.
    for (i, &child_id) in children.iter().enumerate() {
        let rect = tree
            .layout_rect(child_id)
            .unwrap_or_else(|| panic!("child {i} has no layout rect"));
        assert!(
            rect.w > 0.0 && rect.h > 0.0,
            "child {i}: expected non-zero rect, got {rect:?}"
        );
    }
}

#[test]
fn control_panel_child_names() {
    let ctx = emContext::NewRoot();
    let mut panel = emMainControlPanel::new(Rc::clone(&ctx));

    let mut tree = PanelTree::new();
    let root = tree.create_root("ctrl_root");
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);

    {
        let mut pctx = PanelCtx::new(&mut tree, root);
        panel.LayoutChildren(&mut pctx);
    }

    let children: Vec<_> = tree.children(root).collect();
    let names: Vec<&str> = children
        .iter()
        .map(|&id| tree.name(id).unwrap())
        .collect();

    // C++ top-level children: "general" (lMain) and content control panel.
    // Rust: "general" and "bookmarks".
    assert_eq!(names, vec!["general", "bookmarks"]);
}
