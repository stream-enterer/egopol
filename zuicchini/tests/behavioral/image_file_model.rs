use std::path::PathBuf;

use zuicchini::emCore::emImage::emImage;
use zuicchini::emCore::emFileModel::FileState;
use zuicchini::emCore::emImageFile::{ImageFileData, emImageFileModel};
use zuicchini::emCore::emScheduler::EngineScheduler;
use zuicchini::emCore::emImageFileImageFilePanel::emImageFilePanel;

fn make_model() -> emImageFileModel {
    let mut sched = EngineScheduler::new();
    let change = sched.create_signal();
    let update = sched.create_signal();
    let data_change = sched.create_signal();
    emImageFileModel::new(PathBuf::from("test.png"), change, update, data_change)
}

#[test]
fn initial_state_is_waiting() {
    let m = make_model();
    assert!(Match!(m.state(), &FileState::Waiting));
}

#[test]
fn no_data_initially() {
    let m = make_model();
    assert!(m.GetImage().is_none());
    assert!(m.GetComment().is_none());
    assert!(m.GetFileFormatInfo().is_none());
}

#[test]
fn saving_quality_default_100() {
    let m = make_model();
    assert_eq!(m.GetSavingQuality(), 100);
}

#[test]
fn set_saving_quality() {
    let mut m = make_model();
    m.set_saving_quality(75);
    assert_eq!(m.GetSavingQuality(), 75);
}

#[test]
fn set_saving_quality_clamped() {
    let mut m = make_model();
    m.set_saving_quality(200);
    assert_eq!(m.GetSavingQuality(), 100);
}

#[test]
fn set_image_changes_data() {
    let mut m = make_model();
    let data = ImageFileData::default();
    m.file_model_mut().complete_load(data);
    assert!(Match!(m.state(), &FileState::Loaded));

    let img = emImage::new(10, 10, 4);
    let changed = m.set_image(img);
    assert!(changed);
    assert!(Match!(m.state(), &FileState::Unsaved));
}

#[test]
fn set_image_same_value_no_change() {
    let mut m = make_model();
    let data = ImageFileData {
        GetImage: emImage::new(10, 10, 4),
        GetComment: String::new(),
        GetFileFormatInfo: String::new(),
    };
    m.file_model_mut().complete_load(data);

    let same_img = emImage::new(10, 10, 4);
    let changed = m.set_image(same_img);
    assert!(!changed);
    assert!(Match!(m.state(), &FileState::Loaded));
}

#[test]
fn set_comment_changes_data() {
    let mut m = make_model();
    m.file_model_mut().complete_load(ImageFileData::default());

    let changed = m.set_comment("hello".to_string());
    assert!(changed);
    assert_eq!(m.GetComment(), Some("hello"));
    assert!(Match!(m.state(), &FileState::Unsaved));
}

#[test]
fn set_comment_same_value_no_change() {
    let mut m = make_model();
    let data = ImageFileData {
        GetImage: emImage::new(0, 0, 4),
        GetComment: "hello".to_string(),
        GetFileFormatInfo: String::new(),
    };
    m.file_model_mut().complete_load(data);

    let changed = m.set_comment("hello".to_string());
    assert!(!changed);
    assert!(Match!(m.state(), &FileState::Loaded));
}

#[test]
fn set_format_info_changes_data() {
    let mut m = make_model();
    m.file_model_mut().complete_load(ImageFileData::default());

    let changed = m.SetFileFormatInfo("PNG 8-bit".to_string());
    assert!(changed);
    assert_eq!(m.GetFileFormatInfo(), Some("PNG 8-bit"));
}

#[test]
fn set_format_info_same_value_no_change() {
    let mut m = make_model();
    let data = ImageFileData {
        GetImage: emImage::new(0, 0, 4),
        GetComment: String::new(),
        GetFileFormatInfo: "PNG".to_string(),
    };
    m.file_model_mut().complete_load(data);

    let changed = m.SetFileFormatInfo("PNG".to_string());
    assert!(!changed);
}

#[test]
fn reset_data_clears() {
    let mut m = make_model();
    m.file_model_mut().complete_load(ImageFileData::default());
    assert!(Match!(m.state(), &FileState::Loaded));

    m.reset_data();
    assert!(Match!(m.state(), &FileState::Waiting));
    assert!(m.GetImage().is_none());
}

#[test]
fn set_on_no_data_returns_false() {
    let mut m = make_model();
    assert!(!m.set_image(emImage::new(5, 5, 4)));
    assert!(!m.set_comment("test".to_string()));
    assert!(!m.SetFileFormatInfo("test".to_string()));
}

// ── emImageFilePanel tests ─────────────────────────────────────────

#[test]
fn essence_rect_no_image_returns_none() {
    let panel = emImageFilePanel::new();
    assert!(panel.GetEssenceRect(100.0, 100.0).is_none());
}

#[test]
fn essence_rect_square_image_in_square_panel() {
    let mut panel = emImageFilePanel::new();
    panel.set_current_image(Some(emImage::new(100, 100, 4)));

    let (x, y, w, h) = panel.GetEssenceRect(200.0, 200.0).unwrap();
    assert!((x - 0.0).abs() < 1e-10);
    assert!((y - 0.0).abs() < 1e-10);
    assert!((w - 200.0).abs() < 1e-10);
    assert!((h - 200.0).abs() < 1e-10);
}

#[test]
fn essence_rect_landscape_image_in_square_panel() {
    let mut panel = emImageFilePanel::new();
    panel.set_current_image(Some(emImage::new(200, 100, 4)));

    let (x, y, w, h) = panel.GetEssenceRect(200.0, 200.0).unwrap();
    // Landscape GetImage fits width, centered vertically
    assert!((w - 200.0).abs() < 1e-10);
    assert!((h - 100.0).abs() < 1e-10);
    assert!((x - 0.0).abs() < 1e-10);
    assert!((y - 50.0).abs() < 1e-10);
}

#[test]
fn essence_rect_portrait_image_in_square_panel() {
    let mut panel = emImageFilePanel::new();
    panel.set_current_image(Some(emImage::new(100, 200, 4)));

    let (x, y, w, h) = panel.GetEssenceRect(200.0, 200.0).unwrap();
    // Portrait GetImage fits height, centered horizontally
    assert!((h - 200.0).abs() < 1e-10);
    assert!((w - 100.0).abs() < 1e-10);
    assert!((x - 50.0).abs() < 1e-10);
    assert!((y - 0.0).abs() < 1e-10);
}

#[test]
fn essence_rect_wide_panel() {
    let mut panel = emImageFilePanel::new();
    panel.set_current_image(Some(emImage::new(100, 100, 4)));

    let (x, y, w, h) = panel.GetEssenceRect(400.0, 200.0).unwrap();
    // Square GetImage in wide panel: fits height
    assert!((h - 200.0).abs() < 1e-10);
    assert!((w - 200.0).abs() < 1e-10);
    assert!((x - 100.0).abs() < 1e-10);
    assert!((y - 0.0).abs() < 1e-10);
}

#[test]
fn essence_rect_zero_dim_image() {
    let mut panel = emImageFilePanel::new();
    panel.set_current_image(Some(emImage::new(0, 0, 4)));
    assert!(panel.GetEssenceRect(100.0, 100.0).is_none());
}
