use zuicchini::emCore::emInput::InputKey;
use zuicchini::emCore::emInputHotkey::Hotkey;
use zuicchini::emCore::emInputState::emInputState;

#[test]
fn hotkey_parse_ctrl_c() {
    let hk = Hotkey::TryParse("Ctrl+C").unwrap();
    assert!(hk.ctrl);
    assert!(!hk.alt);
    assert!(!hk.shift);
    assert_eq!(hk.key, InputKey::Key('c'));
}

#[test]
fn hotkey_parse_ctrl_shift_s() {
    let hk = Hotkey::TryParse("Ctrl+Shift+S").unwrap();
    assert!(hk.ctrl);
    assert!(hk.shift);
    assert!(!hk.alt);
    assert_eq!(hk.key, InputKey::Key('s'));
}

#[test]
fn hotkey_parse_alt_f4() {
    let hk = Hotkey::TryParse("Alt+F4").unwrap();
    assert!(hk.alt);
    assert_eq!(hk.key, InputKey::F4);
}

#[test]
fn hotkey_parse_single_key() {
    let hk = Hotkey::TryParse("Escape").unwrap();
    assert!(!hk.ctrl);
    assert!(!hk.alt);
    assert!(!hk.shift);
    assert_eq!(hk.key, InputKey::Escape);
}

#[test]
fn hotkey_parse_invalid() {
    assert!(Hotkey::TryParse("").is_none());
    assert!(Hotkey::TryParse("NotAKey+X").is_none());
}

#[test]
fn hotkey_matches() {
    let hk = Hotkey::TryParse("Ctrl+C").unwrap();

    let mut state = emInputState::new();
    state.press(InputKey::Ctrl);
    assert!(hk.Match(InputKey::Key('c'), &state));

    // Without Ctrl held, should not match
    let state2 = emInputState::new();
    assert!(!hk.Match(InputKey::Key('c'), &state2));
}

#[test]
fn input_state_modifiers() {
    let mut state = emInputState::new();
    assert!(!state.GetShift());
    assert!(!state.GetCtrl());
    assert!(!state.GetAlt());
    assert!(!state.GetMeta());

    state.press(InputKey::Shift);
    assert!(state.GetShift());

    state.press(InputKey::Ctrl);
    assert!(state.GetCtrl());

    state.release(InputKey::Shift);
    assert!(!state.GetShift());
    assert!(state.GetCtrl());
}

#[test]
fn input_state_mouse() {
    let mut state = emInputState::new();
    assert_eq!(state.mouse_x, 0.0);
    state.set_mouse(100.0, 200.0);
    assert_eq!(state.mouse_x, 100.0);
    assert_eq!(state.mouse_y, 200.0);
}

#[test]
fn input_state_key_tracking() {
    let mut state = emInputState::new();
    state.press(InputKey::Key('a'));
    assert!(state.IsPressed(InputKey::Key('a')));
    assert!(!state.IsPressed(InputKey::Key('b')));

    state.release(InputKey::Key('a'));
    assert!(!state.IsPressed(InputKey::Key('a')));
}

#[test]
fn input_state_touches() {
    let mut state = emInputState::new();
    state.SetTouch(1, 100.0, 200.0);
    state.SetTouch(2, 300.0, 400.0);
    assert_eq!(state.GetTouchCount().len(), 2);

    state.RemoveTouch(1);
    assert_eq!(state.GetTouchCount().len(), 1);
    assert_eq!(state.GetTouchCount()[0].0, 2);
}

#[test]
fn hotkey_parse_meta() {
    let hk = Hotkey::TryParse("Meta+A").unwrap();
    assert!(hk.meta);
    assert_eq!(hk.key, InputKey::Key('a'));

    let hk2 = Hotkey::TryParse("Cmd+A").unwrap();
    assert!(hk2.meta);
}
