use zuicchini::emCore::emCursor::Cursor;

#[test]
fn all_19_variants_exist() {
    let variants = [
        Cursor::Normal,
        Cursor::Invisible,
        Cursor::Wait,
        Cursor::Crosshair,
        Cursor::Text,
        Cursor::Hand,
        Cursor::ArrowN,
        Cursor::ArrowS,
        Cursor::ArrowE,
        Cursor::ArrowW,
        Cursor::ArrowNE,
        Cursor::ArrowNW,
        Cursor::ArrowSE,
        Cursor::ArrowSW,
        Cursor::ResizeNS,
        Cursor::ResizeEW,
        Cursor::ResizeNESW,
        Cursor::ResizeNWSE,
        Cursor::Move,
    ];
    assert_eq!(variants.len(), 19);
}

#[test]
fn as_str_returns_correct_names() {
    assert_eq!(Cursor::Normal.as_str(), "Normal");
    assert_eq!(Cursor::Invisible.as_str(), "Invisible");
    assert_eq!(Cursor::Wait.as_str(), "Wait");
    assert_eq!(Cursor::Crosshair.as_str(), "Crosshair");
    assert_eq!(Cursor::Text.as_str(), "Text");
    assert_eq!(Cursor::Hand.as_str(), "Hand");
    assert_eq!(Cursor::ArrowN.as_str(), "ArrowN");
    assert_eq!(Cursor::ArrowS.as_str(), "ArrowS");
    assert_eq!(Cursor::ArrowE.as_str(), "ArrowE");
    assert_eq!(Cursor::ArrowW.as_str(), "ArrowW");
    assert_eq!(Cursor::ArrowNE.as_str(), "ArrowNE");
    assert_eq!(Cursor::ArrowNW.as_str(), "ArrowNW");
    assert_eq!(Cursor::ArrowSE.as_str(), "ArrowSE");
    assert_eq!(Cursor::ArrowSW.as_str(), "ArrowSW");
    assert_eq!(Cursor::ResizeNS.as_str(), "ResizeNS");
    assert_eq!(Cursor::ResizeEW.as_str(), "ResizeEW");
    assert_eq!(Cursor::ResizeNESW.as_str(), "ResizeNESW");
    assert_eq!(Cursor::ResizeNWSE.as_str(), "ResizeNWSE");
    assert_eq!(Cursor::Move.as_str(), "Move");
}

#[test]
fn display_matches_as_str() {
    let variants = [
        Cursor::Normal,
        Cursor::Invisible,
        Cursor::Wait,
        Cursor::Crosshair,
        Cursor::Text,
        Cursor::Hand,
        Cursor::ArrowN,
        Cursor::ArrowS,
        Cursor::ArrowE,
        Cursor::ArrowW,
        Cursor::ArrowNE,
        Cursor::ArrowNW,
        Cursor::ArrowSE,
        Cursor::ArrowSW,
        Cursor::ResizeNS,
        Cursor::ResizeEW,
        Cursor::ResizeNESW,
        Cursor::ResizeNWSE,
        Cursor::Move,
    ];
    for v in &variants {
        assert_eq!(format!("{v}"), v.as_str());
    }
}

#[test]
fn cursor_is_copy_clone_eq_hash() {
    let a = Cursor::Hand;
    let b = a; // Copy
    let c = a.clone(); // Clone
    assert_eq!(a, b); // PartialEq
    assert_eq!(a, c);

    // Hash: insert into HashSet
    let mut set = std::collections::HashSet::new();
    set.insert(a);
    set.insert(b);
    assert_eq!(set.len(), 1);
}

#[test]
fn cursor_debug_format() {
    let dbg = format!("{:?}", Cursor::ResizeNWSE);
    assert!(dbg.contains("ResizeNWSE"));
}

#[test]
fn all_as_str_values_unique() {
    let variants = [
        Cursor::Normal,
        Cursor::Invisible,
        Cursor::Wait,
        Cursor::Crosshair,
        Cursor::Text,
        Cursor::Hand,
        Cursor::ArrowN,
        Cursor::ArrowS,
        Cursor::ArrowE,
        Cursor::ArrowW,
        Cursor::ArrowNE,
        Cursor::ArrowNW,
        Cursor::ArrowSE,
        Cursor::ArrowSW,
        Cursor::ResizeNS,
        Cursor::ResizeEW,
        Cursor::ResizeNESW,
        Cursor::ResizeNWSE,
        Cursor::Move,
    ];
    let mut names: Vec<&str> = variants.iter().map(|c| c.as_str()).collect();
    let original_len = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), original_len, "as_str() values must be unique");
}
