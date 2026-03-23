use zuicchini::emCore::emLook::emLook;
use zuicchini::emCore::emRadioButton::{emRadioButton, RadioGroup};

/// emRadioButton::Mechanism::AddAll(emPanel*)
/// Adds multiple button slots to the group at once.
#[test]
fn add_all_increases_count() {
    let group = RadioGroup::new();
    assert_eq!(group.borrow().GetCount(), 0);
    group.borrow_mut().AddAll(5);
    assert_eq!(group.borrow().GetCount(), 5);
}

#[test]
fn add_all_zero_is_noop() {
    let group = RadioGroup::new();
    group.borrow_mut().AddAll(0);
    assert_eq!(group.borrow().GetCount(), 0);
}

#[test]
fn add_all_preserves_existing_buttons() {
    let look = emLook::new();
    let group = RadioGroup::new();
    let _r0 = emRadioButton::new("A", look.clone(), group.clone(), 0);
    assert_eq!(group.borrow().GetCount(), 1);

    // AddAll adds 3 more slots
    group.borrow_mut().AddAll(3);
    assert_eq!(group.borrow().GetCount(), 4);
}

#[test]
fn add_all_preserves_selection() {
    let group = RadioGroup::new();
    group.borrow_mut().AddAll(3);
    group.borrow_mut().Select(1);
    assert_eq!(group.borrow().GetChecked(), Some(1));

    // Adding more doesn't change selection
    group.borrow_mut().AddAll(2);
    assert_eq!(group.borrow().GetChecked(), Some(1));
    assert_eq!(group.borrow().GetCount(), 5);
}

/// emRadioButton::Mechanism::GetButton(int)
/// Returns the button index at the given GetPos, or None if out of range.
#[test]
fn get_button_valid_index() {
    let group = RadioGroup::new();
    group.borrow_mut().AddAll(3);

    assert_eq!(group.borrow().GetButton(0), Some(0));
    assert_eq!(group.borrow().GetButton(1), Some(1));
    assert_eq!(group.borrow().GetButton(2), Some(2));
}

#[test]
fn get_button_out_of_range() {
    let group = RadioGroup::new();
    group.borrow_mut().AddAll(2);

    assert_eq!(group.borrow().GetButton(2), None);
    assert_eq!(group.borrow().GetButton(100), None);
}

#[test]
fn get_button_empty_group() {
    let group = RadioGroup::new();
    assert_eq!(group.borrow().GetButton(0), None);
}

#[test]
fn get_button_with_real_buttons() {
    let look = emLook::new();
    let group = RadioGroup::new();
    let _r0 = emRadioButton::new("A", look.clone(), group.clone(), 0);
    let _r1 = emRadioButton::new("B", look.clone(), group.clone(), 1);

    assert_eq!(group.borrow().GetButton(0), Some(0));
    assert_eq!(group.borrow().GetButton(1), Some(1));
    assert_eq!(group.borrow().GetButton(2), None);
}
