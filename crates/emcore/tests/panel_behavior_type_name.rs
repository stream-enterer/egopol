//! Tests for type-name capture in PanelTree::set_behavior and
//! PanelCtx::create_child_with.
//!
//! RUST_ONLY: (language-forced-utility) Rust trait objects do not preserve
//! concrete-type information through their vtables; C++ has full RTTI. These
//! tests verify the monomorphized type_name capture path.

use emcore::emPanel::PanelBehavior;
use emcore::emPanelTree::PanelTree;

struct DummyBehavior;

impl PanelBehavior for DummyBehavior {}

#[test]
fn set_behavior_captures_concrete_type_name() {
    let mut tree = PanelTree::new();
    let root = tree.create_root("root", false);
    tree.set_behavior(root, DummyBehavior);
    let name = tree
        .behavior_type_name(root)
        .expect("panel must exist and have a type name");
    assert!(
        name.ends_with("DummyBehavior"),
        "expected type name ending with 'DummyBehavior', got {name:?}"
    );
}

#[test]
fn set_behavior_dyn_uses_explicit_name() {
    let mut tree = PanelTree::new();
    let root = tree.create_root("root", false);
    tree.set_behavior_dyn(root, Box::new(DummyBehavior), "test::ExplicitName");
    let name = tree
        .behavior_type_name(root)
        .expect("panel must exist and have a type name");
    assert_eq!(name, "test::ExplicitName");
}
