use std::cell::RefCell;
use std::rc::Rc;

use emcore::emClipboard::{emClipboard, emPrivateClipboard, LookupInherited};
use emcore::emContext::emContext;

#[test]
fn clipboard_trait_exists_and_is_object_safe() {
    let cb: Rc<RefCell<dyn emClipboard>> = Rc::new(RefCell::new(emPrivateClipboard::new()));
    // Verify trait object works
    cb.borrow_mut().PutText("test", false);
    assert_eq!(cb.borrow().GetText(false), "test");
}

#[test]
fn lookup_inherited_walks_parent_chain() {
    let root = emContext::NewRoot();
    emPrivateClipboard::Install(&root);

    let child = emContext::NewChild(&root);
    let grandchild = emContext::NewChild(&child);

    // LookupInherited from grandchild should find root's clipboard
    let cb = LookupInherited(&grandchild).expect("should find clipboard");
    cb.borrow_mut().PutText("inherited", false);
    assert_eq!(cb.borrow().GetText(false), "inherited");
}

#[test]
fn put_text_clipboard() {
    let mut cb = emPrivateClipboard::new();
    let id = cb.PutText("hello", false);
    assert_eq!(id, 0); // clipboard always returns 0
    assert_eq!(cb.GetText(false), "hello");
}

#[test]
fn put_text_selection() {
    let mut cb = emPrivateClipboard::new();
    let id1 = cb.PutText("sel1", true);
    assert!(id1 > 0);
    let id2 = cb.PutText("sel2", true);
    assert!(id2 > id1); // monotonically increasing
    assert_eq!(cb.GetText(true), "sel2");
}

#[test]
fn clear_clipboard() {
    let mut cb = emPrivateClipboard::new();
    cb.PutText("data", false);
    cb.Clear(false, 0);
    assert_eq!(cb.GetText(false), "");
}

#[test]
fn clear_selection_matching_id() {
    let mut cb = emPrivateClipboard::new();
    let id = cb.PutText("sel", true);
    cb.Clear(true, id);
    assert_eq!(cb.GetText(true), "");
}

#[test]
fn clear_selection_wrong_id_is_noop() {
    let mut cb = emPrivateClipboard::new();
    let id = cb.PutText("sel", true);
    cb.Clear(true, id - 1); // wrong ID
    assert_eq!(cb.GetText(true), "sel"); // still there
}

#[test]
fn get_text_empty_clipboard() {
    let cb = emPrivateClipboard::new();
    assert_eq!(cb.GetText(false), "");
    assert_eq!(cb.GetText(true), "");
}

#[test]
fn get_text_independent_buffers() {
    let mut cb = emPrivateClipboard::new();
    cb.PutText("clip", false);
    cb.PutText("sel", true);
    assert_eq!(cb.GetText(false), "clip");
    assert_eq!(cb.GetText(true), "sel");
}

#[test]
fn install_registers_clipboard_in_context() {
    let ctx = emContext::NewRoot();
    assert!(LookupInherited(&ctx).is_none());
    emPrivateClipboard::Install(&ctx);
    assert!(LookupInherited(&ctx).is_some());
}

#[test]
fn private_clipboard_default() {
    let cb = emPrivateClipboard::default();
    assert_eq!(cb.GetText(false), "");
    assert_eq!(cb.GetText(true), "");
}

#[test]
fn private_clipboard_separate_buffers() {
    let mut cb = emPrivateClipboard::new();
    cb.PutText("clipboard_data", false);
    assert_eq!(cb.GetText(false), "clipboard_data");
    assert_eq!(cb.GetText(true), ""); // selection is independent
}

#[test]
fn private_clipboard_install_idempotent() {
    let ctx = emContext::NewRoot();
    emPrivateClipboard::Install(&ctx);
    let cb1 = LookupInherited(&ctx).unwrap();
    cb1.borrow_mut().PutText("test", false);

    // Re-Install replaces (matching C++ behavior where re-Install overwrites)
    emPrivateClipboard::Install(&ctx);
    let cb2 = LookupInherited(&ctx).unwrap();
    // New clipboard has empty state
    assert_eq!(cb2.borrow().GetText(false), "");
}

#[test]
fn full_clipboard_lifecycle() {
    let mut cb = emPrivateClipboard::new();

    // Put and GetRec clipboard
    cb.PutText("clip1", false);
    assert_eq!(cb.GetText(false), "clip1");

    // Put and GetRec selection
    let sel_id = cb.PutText("sel1", true);
    assert_eq!(cb.GetText(true), "sel1");

    // Clear selection with correct ID
    cb.Clear(true, sel_id);
    assert_eq!(cb.GetText(true), "");

    // emClipboard unaffected
    assert_eq!(cb.GetText(false), "clip1");

    // Clear clipboard
    cb.Clear(false, 0);
    assert_eq!(cb.GetText(false), "");
}

#[test]
fn selection_id_increments_on_clear() {
    let mut cb = emPrivateClipboard::new();
    let id1 = cb.PutText("s1", true);
    cb.Clear(true, id1); // clears and increments sel_id
    let id2 = cb.PutText("s2", true);
    assert!(id2 > id1 + 1); // id incremented by Clear too
}
