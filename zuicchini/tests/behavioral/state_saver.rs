use zuicchini::foundation::{parse_rec, write_rec, RecStruct};
use zuicchini::model::{RecError, Record};
use zuicchini::emCore::emWindowStateSaver::WindowGeometry;

#[test]
fn window_geometry_default_values() {
    let geo = WindowGeometry::default();
    assert_eq!(geo.x, 100);
    assert_eq!(geo.y, 100);
    assert_eq!(geo.width, 1280);
    assert_eq!(geo.height, 720);
    assert!(!geo.maximized);
    assert!(!geo.fullscreen);
}

#[test]
fn window_geometry_round_trip_to_rec() {
    let original = WindowGeometry {
        x: 200,
        y: 300,
        width: 1920,
        height: 1080,
        maximized: true,
        fullscreen: false,
    };

    let rec = original.to_rec();
    let restored = WindowGeometry::from_rec(&rec).expect("from_rec");
    assert_eq!(restored, original);
}

#[test]
fn window_geometry_round_trip_through_text() {
    let original = WindowGeometry {
        x: -50,
        y: 0,
        width: 800,
        height: 600,
        maximized: false,
        fullscreen: true,
    };

    let rec = original.to_rec();
    let text = write_rec(&rec);
    let parsed = parse_rec(&text).expect("parse");
    let restored = WindowGeometry::from_rec(&parsed).expect("from_rec");
    assert_eq!(restored, original);
}

#[test]
fn window_geometry_negative_coordinates() {
    let geo = WindowGeometry {
        x: -100,
        y: -200,
        width: 640,
        height: 480,
        maximized: false,
        fullscreen: false,
    };

    let rec = geo.to_rec();
    let restored = WindowGeometry::from_rec(&rec).expect("from_rec");
    assert_eq!(restored.x, -100);
    assert_eq!(restored.y, -200);
}

#[test]
fn window_geometry_from_rec_missing_x_fails() {
    let mut rec = RecStruct::new();
    rec.set_int("y", 100);
    rec.set_int("width", 800);
    rec.set_int("height", 600);

    let result = WindowGeometry::from_rec(&rec);
    assert!(matches!(result, Err(RecError::MissingField(_))));
}

#[test]
fn window_geometry_from_rec_missing_width_fails() {
    let mut rec = RecStruct::new();
    rec.set_int("x", 100);
    rec.set_int("y", 100);
    rec.set_int("height", 600);

    let result = WindowGeometry::from_rec(&rec);
    assert!(matches!(result, Err(RecError::MissingField(_))));
}

#[test]
fn window_geometry_bools_default_false_when_absent() {
    let mut rec = RecStruct::new();
    rec.set_int("x", 10);
    rec.set_int("y", 20);
    rec.set_int("width", 800);
    rec.set_int("height", 600);

    let geo = WindowGeometry::from_rec(&rec).expect("from_rec");
    assert!(!geo.maximized);
    assert!(!geo.fullscreen);
}

#[test]
fn window_geometry_is_default() {
    let geo = WindowGeometry::default();
    assert!(geo.is_default());

    let modified = WindowGeometry {
        x: 999,
        ..WindowGeometry::default()
    };
    assert!(!modified.is_default());
}

#[test]
fn window_geometry_set_to_default() {
    let mut geo = WindowGeometry {
        x: 999,
        y: 888,
        width: 100,
        height: 50,
        maximized: true,
        fullscreen: true,
    };
    geo.set_to_default();
    assert_eq!(geo, WindowGeometry::default());
}

#[test]
fn window_geometry_all_fields_serialized() {
    let geo = WindowGeometry {
        x: 42,
        y: 84,
        width: 1600,
        height: 900,
        maximized: true,
        fullscreen: true,
    };

    let rec = geo.to_rec();
    assert_eq!(rec.get_int("x"), Some(42));
    assert_eq!(rec.get_int("y"), Some(84));
    assert_eq!(rec.get_int("width"), Some(1600));
    assert_eq!(rec.get_int("height"), Some(900));
    assert_eq!(rec.get_bool("maximized"), Some(true));
    assert_eq!(rec.get_bool("fullscreen"), Some(true));
}
