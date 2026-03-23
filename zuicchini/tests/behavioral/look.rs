use std::rc::Rc;
use zuicchini::emCore::emColor::emColor;
use zuicchini::emCore::emLook::emLook;

/// emLook::Apply — apply look to target, optionally recursive.
#[test]
fn apply_replaces_target_look() {
    let mut target = emLook::new(); // default look
    let mut custom = emLook::default();
    custom.bg_color = emColor::RED;

    custom.apply(&mut target, false);
    assert_eq!(target.bg_color, emColor::RED);
}

#[test]
fn apply_preserves_all_fields() {
    let mut custom = emLook::default();
    custom.bg_color = emColor::rgba(0x11, 0x22, 0x33, 0xFF);
    custom.fg_color = emColor::rgba(0xAA, 0xBB, 0xCC, 0xFF);
    custom.button_bg_color = emColor::rgba(0x44, 0x55, 0x66, 0xFF);
    custom.input_hl_color = emColor::rgba(0x77, 0x88, 0x99, 0xFF);

    let mut target = emLook::new();
    custom.apply(&mut target, true);
    assert_eq!(target.bg_color, custom.bg_color);
    assert_eq!(target.fg_color, custom.fg_color);
    assert_eq!(target.button_bg_color, custom.button_bg_color);
    assert_eq!(target.input_hl_color, custom.input_hl_color);
}

#[test]
fn apply_all_updates_multiple_targets() {
    let mut custom = emLook::default();
    custom.bg_color = emColor::GREEN;

    let mut t1 = emLook::new();
    let mut t2 = emLook::new();
    custom.apply_all(&mut [&mut t1, &mut t2]);
    assert_eq!(t1.bg_color, emColor::GREEN);
    assert_eq!(t2.bg_color, emColor::GREEN);
}

#[test]
fn look_is_cloneable() {
    let look = emLook::default();
    let cloned = look.clone();
    assert_eq!(look, cloned);
}

#[test]
fn apply_creates_independent_rc() {
    let mut custom = emLook::default();
    custom.bg_color = emColor::BLUE;

    let mut target = emLook::new();
    let original_ptr = Rc::as_ptr(&target);
    custom.apply(&mut target, false);
    // Should be a new Rc, not the same allocation
    assert_ne!(Rc::as_ptr(&target), original_ptr);
    assert_eq!(target.bg_color, emColor::BLUE);
}
