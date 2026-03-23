use zuicchini::emCore::emCursor::emCursor;

#[test]
fn all_19_variants_exist() {
    let variants = [
        emCursor::Normal,
        emCursor::Invisible,
        emCursor::Wait,
        emCursor::Crosshair,
        emCursor::Text,
        emCursor::Hand,
        emCursor::ArrowN,
        emCursor::ArrowS,
        emCursor::ArrowE,
        emCursor::ArrowW,
        emCursor::ArrowNE,
        emCursor::ArrowNW,
        emCursor::ArrowSE,
        emCursor::ArrowSW,
        emCursor::ResizeNS,
        emCursor::ResizeEW,
        emCursor::ResizeNESW,
        emCursor::ResizeNWSE,
        emCursor::Move,
    ];
    assert_eq!(variants.len(), 19);
}

#[test]
fn as_str_returns_correct_names() {
    assert_eq!(emCursor::Normal.as_str(), "Normal");
    assert_eq!(emCursor::Invisible.as_str(), "Invisible");
    assert_eq!(emCursor::Wait.as_str(), "Wait");
    assert_eq!(emCursor::Crosshair.as_str(), "Crosshair");
    assert_eq!(emCursor::Text.as_str(), "Text");
    assert_eq!(emCursor::Hand.as_str(), "Hand");
    assert_eq!(emCursor::ArrowN.as_str(), "ArrowN");
    assert_eq!(emCursor::ArrowS.as_str(), "ArrowS");
    assert_eq!(emCursor::ArrowE.as_str(), "ArrowE");
    assert_eq!(emCursor::ArrowW.as_str(), "ArrowW");
    assert_eq!(emCursor::ArrowNE.as_str(), "ArrowNE");
    assert_eq!(emCursor::ArrowNW.as_str(), "ArrowNW");
    assert_eq!(emCursor::ArrowSE.as_str(), "ArrowSE");
    assert_eq!(emCursor::ArrowSW.as_str(), "ArrowSW");
    assert_eq!(emCursor::ResizeNS.as_str(), "ResizeNS");
    assert_eq!(emCursor::ResizeEW.as_str(), "ResizeEW");
    assert_eq!(emCursor::ResizeNESW.as_str(), "ResizeNESW");
    assert_eq!(emCursor::ResizeNWSE.as_str(), "ResizeNWSE");
    assert_eq!(emCursor::Move.as_str(), "Move");
}

#[test]
fn display_matches_as_str() {
    let variants = [
        emCursor::Normal,
        emCursor::Invisible,
        emCursor::Wait,
        emCursor::Crosshair,
        emCursor::Text,
        emCursor::Hand,
        emCursor::ArrowN,
        emCursor::ArrowS,
        emCursor::ArrowE,
        emCursor::ArrowW,
        emCursor::ArrowNE,
        emCursor::ArrowNW,
        emCursor::ArrowSE,
        emCursor::ArrowSW,
        emCursor::ResizeNS,
        emCursor::ResizeEW,
        emCursor::ResizeNESW,
        emCursor::ResizeNWSE,
        emCursor::Move,
    ];
    for v in &variants {
        assert_eq!(format!("{v}"), v.as_str());
    }
}

#[test]
fn cursor_is_copy_clone_eq_hash() {
    let a = emCursor::Hand;
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
    let dbg = format!("{:?}", emCursor::ResizeNWSE);
    assert!(dbg.contains("ResizeNWSE"));
}

#[test]
fn all_as_str_values_unique() {
    let variants = [
        emCursor::Normal,
        emCursor::Invisible,
        emCursor::Wait,
        emCursor::Crosshair,
        emCursor::Text,
        emCursor::Hand,
        emCursor::ArrowN,
        emCursor::ArrowS,
        emCursor::ArrowE,
        emCursor::ArrowW,
        emCursor::ArrowNE,
        emCursor::ArrowNW,
        emCursor::ArrowSE,
        emCursor::ArrowSW,
        emCursor::ResizeNS,
        emCursor::ResizeEW,
        emCursor::ResizeNESW,
        emCursor::ResizeNWSE,
        emCursor::Move,
    ];
    let mut names: Vec<&str> = variants.iter().map(|c| c.as_str()).collect();
    let original_len = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), original_len, "as_str() values must be unique");
}
