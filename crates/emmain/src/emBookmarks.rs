use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emConfigModel::emConfigModel;
use emcore::emContext::emContext;
use emcore::emInstallInfo::{emGetInstallPath, InstallDirType};
use emcore::emRec::{RecError, RecStruct, RecValue};
use emcore::emRecRecord::Record;
use emcore::emRecRecTypes::emColorRec;
use emcore::emSignal::SignalId;
use slotmap::Key as _;

// DIVERGED: C++ uses class inheritance (emBookmarkEntryRec → emBookmarkRec,
// emBookmarkGroupRec). Rust flattens to struct composition — `emBookmarkEntryBase`
// is embedded by value in each concrete struct. The field names and default
// colors are identical to the C++ originals.

// Default colors matching C++ emBookmarkEntryRec constructor defaults:
// emBookmarkRec:      BgColor = emLook().GetButtonBgColor() = 0x596790FF
//                     FgColor = emLook().GetButtonFgColor() = 0xF2F2F7FF
// emBookmarkGroupRec: BgColor = emLook().GetBgColor()       = 0x515E84FF
//                     FgColor = emLook().GetFgColor()        = 0xEFF0F4FF

/// Default background color for a single bookmark (C++ emLook::GetButtonBgColor).
const BOOKMARK_BG_DEFAULT: emColor = emColor::from_packed(0x596790FF);
/// Default foreground color for a single bookmark (C++ emLook::GetButtonFgColor).
const BOOKMARK_FG_DEFAULT: emColor = emColor::from_packed(0xF2F2F7FF);
/// Default background color for a bookmark group (C++ emLook::GetBgColor).
const GROUP_BG_DEFAULT: emColor = emColor::from_packed(0x515E84FF);
/// Default foreground color for a bookmark group (C++ emLook::GetFgColor).
const GROUP_FG_DEFAULT: emColor = emColor::from_packed(0xEFF0F4FF);

// ── emBookmarkEntryBase ───────────────────────────────────────────────────────

/// Common bookmark entry fields (C++ emBookmarkEntryRec).
#[derive(Debug, Clone, PartialEq)]
pub struct emBookmarkEntryBase {
    pub Name: String,
    pub Description: String,
    pub Icon: String,
    pub BgColor: emColor,
    pub FgColor: emColor,
}

impl emBookmarkEntryBase {
    fn new(bg: emColor, fg: emColor) -> Self {
        Self {
            Name: String::new(),
            Description: String::new(),
            Icon: String::new(),
            BgColor: bg,
            FgColor: fg,
        }
    }

    fn from_rec_with_defaults(rec: &RecStruct, bg_default: emColor, fg_default: emColor) -> Self {
        let bg = rec
            .get_struct("bgcolor")
            .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
            .unwrap_or(bg_default);
        let fg = rec
            .get_struct("fgcolor")
            .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
            .unwrap_or(fg_default);
        Self {
            Name: rec.get_str("name").unwrap_or("").to_string(),
            Description: rec.get_str("description").unwrap_or("").to_string(),
            Icon: rec.get_str("icon").unwrap_or("").to_string(),
            BgColor: bg,
            FgColor: fg,
        }
    }

    fn write_into(&self, s: &mut RecStruct) {
        s.set_str("name", &self.Name);
        s.set_str("description", &self.Description);
        s.set_str("icon", &self.Icon);
        s.SetValue(
            "bgcolor",
            RecValue::Struct(emColorRec::ToRecStruct(self.BgColor, true)),
        );
        s.SetValue(
            "fgcolor",
            RecValue::Struct(emColorRec::ToRecStruct(self.FgColor, true)),
        );
    }
}

impl Default for emBookmarkEntryBase {
    fn default() -> Self {
        Self::new(BOOKMARK_BG_DEFAULT, BOOKMARK_FG_DEFAULT)
    }
}

// ── emBookmarkRec ─────────────────────────────────────────────────────────────

/// A single bookmark (C++ emBookmarkRec).
#[derive(Debug, Clone, PartialEq)]
pub struct emBookmarkRec {
    pub entry: emBookmarkEntryBase,
    pub Hotkey: String,
    pub LocationIdentity: String,
    pub LocationRelX: f64,
    pub LocationRelY: f64,
    pub LocationRelA: f64,
    pub VisitAtProgramStart: bool,
}

impl emBookmarkRec {
    fn entry_bg_default() -> emColor {
        BOOKMARK_BG_DEFAULT
    }
    fn entry_fg_default() -> emColor {
        BOOKMARK_FG_DEFAULT
    }

    pub fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        let entry = emBookmarkEntryBase::from_rec_with_defaults(
            rec,
            Self::entry_bg_default(),
            Self::entry_fg_default(),
        );
        Ok(Self {
            entry,
            Hotkey: rec.get_str("hotkey").unwrap_or("").to_string(),
            LocationIdentity: rec
                .get_str("locationidentity")
                .unwrap_or("")
                .to_string(),
            LocationRelX: rec.get_double("locationrelx").unwrap_or(0.0),
            LocationRelY: rec.get_double("locationrely").unwrap_or(0.0),
            LocationRelA: rec.get_double("locationrela").unwrap_or(0.0).max(0.0),
            VisitAtProgramStart: rec.get_bool("visitatprogramstart").unwrap_or(false),
        })
    }

    pub fn to_rec(&self) -> RecStruct {
        let mut s = RecStruct::new();
        self.entry.write_into(&mut s);
        s.set_str("hotkey", &self.Hotkey);
        s.set_str("locationidentity", &self.LocationIdentity);
        s.set_double("locationrelx", self.LocationRelX);
        s.set_double("locationrely", self.LocationRelY);
        s.set_double("locationrela", self.LocationRelA);
        s.set_bool("visitatprogramstart", self.VisitAtProgramStart);
        s
    }
}

impl Default for emBookmarkRec {
    fn default() -> Self {
        Self {
            entry: emBookmarkEntryBase::new(Self::entry_bg_default(), Self::entry_fg_default()),
            Hotkey: String::new(),
            LocationIdentity: String::new(),
            LocationRelX: 0.0,
            LocationRelY: 0.0,
            LocationRelA: 0.0,
            VisitAtProgramStart: false,
        }
    }
}

// ── emBookmarkGroupRec ────────────────────────────────────────────────────────

/// A bookmark group with nested children (C++ emBookmarkGroupRec).
#[derive(Debug, Clone, PartialEq)]
pub struct emBookmarkGroupRec {
    pub entry: emBookmarkEntryBase,
    pub Bookmarks: Vec<emBookmarkEntryUnion>,
}

impl emBookmarkGroupRec {
    fn entry_bg_default() -> emColor {
        GROUP_BG_DEFAULT
    }
    fn entry_fg_default() -> emColor {
        GROUP_FG_DEFAULT
    }

    pub fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        let entry = emBookmarkEntryBase::from_rec_with_defaults(
            rec,
            Self::entry_bg_default(),
            Self::entry_fg_default(),
        );
        let bookmarks = if let Some(arr) = rec.get_array("bookmarks") {
            arr.iter()
                .filter_map(|v| emBookmarkEntryUnion::from_rec_value(v).ok())
                .collect()
        } else {
            Vec::new()
        };
        Ok(Self {
            entry,
            Bookmarks: bookmarks,
        })
    }

    pub fn to_rec(&self) -> RecStruct {
        let mut s = RecStruct::new();
        self.entry.write_into(&mut s);
        let items: Vec<RecValue> = self
            .Bookmarks
            .iter()
            .map(|e| e.to_rec_value())
            .collect();
        s.SetValue("bookmarks", RecValue::Array(items));
        s
    }
}

impl Default for emBookmarkGroupRec {
    fn default() -> Self {
        Self {
            entry: emBookmarkEntryBase::new(Self::entry_bg_default(), Self::entry_fg_default()),
            Bookmarks: Vec::new(),
        }
    }
}

// ── emBookmarkEntryUnion ──────────────────────────────────────────────────────

/// Union of bookmark or group (C++ emUnionRec with BOOKMARK/GROUP variants).
#[derive(Debug, Clone, PartialEq)]
pub enum emBookmarkEntryUnion {
    Bookmark(emBookmarkRec),
    Group(emBookmarkGroupRec),
}

impl emBookmarkEntryUnion {
    fn from_rec_value(val: &RecValue) -> Result<Self, RecError> {
        match val {
            RecValue::Union(variant, inner) if variant == "bookmark" => {
                match inner.as_ref() {
                    RecValue::Struct(s) => Ok(Self::Bookmark(emBookmarkRec::from_rec(s)?)),
                    _ => Err(RecError::InvalidValue {
                        field: "bookmark".into(),
                        message: "expected struct inside union".into(),
                    }),
                }
            }
            RecValue::Union(variant, inner) if variant == "group" => {
                match inner.as_ref() {
                    RecValue::Struct(s) => Ok(Self::Group(emBookmarkGroupRec::from_rec(s)?)),
                    _ => Err(RecError::InvalidValue {
                        field: "group".into(),
                        message: "expected struct inside union".into(),
                    }),
                }
            }
            RecValue::Union(variant, _) => Err(RecError::InvalidValue {
                field: "entry".into(),
                message: format!("unknown union variant: {variant}"),
            }),
            _ => Err(RecError::InvalidValue {
                field: "entry".into(),
                message: "expected union value".into(),
            }),
        }
    }

    fn to_rec_value(&self) -> RecValue {
        match self {
            Self::Bookmark(bm) => RecValue::Union(
                "bookmark".to_string(),
                Box::new(RecValue::Struct(bm.to_rec())),
            ),
            Self::Group(grp) => RecValue::Union(
                "group".to_string(),
                Box::new(RecValue::Struct(grp.to_rec())),
            ),
        }
    }
}

// ── emBookmarksRec ────────────────────────────────────────────────────────────

/// Root record: array of bookmark entries (C++ emBookmarksRec).
#[derive(Debug, Clone, PartialEq)]
pub struct emBookmarksRec {
    pub entries: Vec<emBookmarkEntryUnion>,
}

impl emBookmarksRec {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Find the first bookmark with `VisitAtProgramStart == true` (recursive).
    ///
    /// Port of C++ `emBookmarksRec::SearchStartLocation`.
    pub fn SearchStartLocation(&self) -> Option<&emBookmarkRec> {
        search_start_location_in(&self.entries)
    }

    /// Find a bookmark matching `hotkey` string (recursive).
    ///
    /// Port of C++ `emBookmarksRec::SearchBookmarkByHotkey`.
    pub fn SearchBookmarkByHotkey(&self, hotkey: &str) -> Option<&emBookmarkRec> {
        if hotkey.is_empty() {
            return None;
        }
        search_bookmark_by_hotkey_in(&self.entries, hotkey)
    }

    /// Set `VisitAtProgramStart` on the given bookmark; clears all others first.
    ///
    /// Port of C++ `emBookmarksRec::SetStartLocation`.
    pub fn SetStartLocation(&mut self, target_identity: &str) {
        self.ClearStartLocation();
        set_start_location_in(&mut self.entries, target_identity);
    }

    /// Clear all `VisitAtProgramStart` flags (recursive).
    ///
    /// Port of C++ `emBookmarksRec::ClearStartLocation`.
    pub fn ClearStartLocation(&mut self) {
        clear_start_location_in(&mut self.entries);
    }
}

impl Default for emBookmarksRec {
    fn default() -> Self {
        Self::new()
    }
}

impl Record for emBookmarksRec {
    fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        let entries = if let Some(arr) = rec.get_array("entries") {
            arr.iter()
                .filter_map(|v| emBookmarkEntryUnion::from_rec_value(v).ok())
                .collect()
        } else {
            Vec::new()
        };
        Ok(Self { entries })
    }

    fn to_rec(&self) -> RecStruct {
        let mut s = RecStruct::new();
        let items: Vec<RecValue> = self
            .entries
            .iter()
            .map(|e| e.to_rec_value())
            .collect();
        s.SetValue("entries", RecValue::Array(items));
        s
    }

    fn SetToDefault(&mut self) {
        *self = Self::default();
    }

    fn IsSetToDefault(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── Private recursive helpers ─────────────────────────────────────────────────

fn search_start_location_in(entries: &[emBookmarkEntryUnion]) -> Option<&emBookmarkRec> {
    for entry in entries {
        match entry {
            emBookmarkEntryUnion::Bookmark(bm) => {
                if bm.VisitAtProgramStart {
                    return Some(bm);
                }
            }
            emBookmarkEntryUnion::Group(grp) => {
                if let Some(bm) = search_start_location_in(&grp.Bookmarks) {
                    return Some(bm);
                }
            }
        }
    }
    None
}

fn search_bookmark_by_hotkey_in<'a>(
    entries: &'a [emBookmarkEntryUnion],
    hotkey: &str,
) -> Option<&'a emBookmarkRec> {
    for entry in entries {
        match entry {
            emBookmarkEntryUnion::Bookmark(bm) => {
                if !bm.Hotkey.is_empty() && bm.Hotkey == hotkey {
                    return Some(bm);
                }
            }
            emBookmarkEntryUnion::Group(grp) => {
                if let Some(bm) = search_bookmark_by_hotkey_in(&grp.Bookmarks, hotkey) {
                    return Some(bm);
                }
            }
        }
    }
    None
}

fn clear_start_location_in(entries: &mut [emBookmarkEntryUnion]) {
    for entry in entries {
        match entry {
            emBookmarkEntryUnion::Bookmark(bm) => {
                bm.VisitAtProgramStart = false;
            }
            emBookmarkEntryUnion::Group(grp) => {
                clear_start_location_in(&mut grp.Bookmarks);
            }
        }
    }
}

fn set_start_location_in(entries: &mut [emBookmarkEntryUnion], target_identity: &str) -> bool {
    for entry in entries {
        match entry {
            emBookmarkEntryUnion::Bookmark(bm) => {
                if bm.LocationIdentity == target_identity {
                    bm.VisitAtProgramStart = true;
                    return true;
                }
            }
            emBookmarkEntryUnion::Group(grp) => {
                if set_start_location_in(&mut grp.Bookmarks, target_identity) {
                    return true;
                }
            }
        }
    }
    false
}

// ── emBookmarksModel ──────────────────────────────────────────────────────────

/// Model wrapper for emBookmarks.
///
/// Port of C++ `emBookmarksModel` (extends emConfigModel + emBookmarksRec).
/// Backed by `emConfigModel` for file persistence at
/// `~/.eaglemode/emMain/bookmarks.rec`.
pub struct emBookmarksModel {
    config_model: emConfigModel<emBookmarksRec>,
}

impl emBookmarksModel {
    /// Acquire the singleton `emBookmarksModel` from the context registry.
    ///
    /// Port of C++ `emBookmarksModel::Acquire`.
    pub fn Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            let path =
                emGetInstallPath(InstallDirType::UserConfig, "emMain", Some("bookmarks.rec"))
                    .unwrap_or_else(|_| {
                        let home =
                            std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        std::path::PathBuf::from(home)
                            .join(".eaglemode")
                            .join("emMain")
                            .join("bookmarks.rec")
                    });

            let mut model =
                emConfigModel::new(emBookmarksRec::default(), path, SignalId::null());

            if let Err(e) = model.TryLoadOrInstall() {
                log::warn!("emBookmarksModel: failed to load or install: {e}");
            }

            Self {
                config_model: model,
            }
        })
    }

    pub fn GetFormatName(&self) -> &str {
        "emBookmarks"
    }

    pub fn GetChangeSignal(&self) -> SignalId {
        self.config_model.GetChangeSignal()
    }

    pub fn GetRec(&self) -> &emBookmarksRec {
        self.config_model.GetRec()
    }

    pub fn Set(&mut self, rec: emBookmarksRec) {
        self.config_model.Set(rec);
    }

    pub fn IsUnsaved(&self) -> bool {
        self.config_model.IsUnsaved()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_rec_defaults() {
        let bm = emBookmarkRec::default();
        assert_eq!(bm.entry.Name, "");
        assert_eq!(bm.LocationIdentity, "");
        assert!((bm.LocationRelX).abs() < 1e-10);
        assert!((bm.LocationRelY).abs() < 1e-10);
        assert!((bm.LocationRelA).abs() < 1e-10);
        assert!(!bm.VisitAtProgramStart);
    }

    #[test]
    fn test_bookmark_rec_round_trip() {
        let mut bm = emBookmarkRec::default();
        bm.entry.Name = "Home".to_string();
        bm.LocationIdentity = "::VcItem:Home:".to_string();
        bm.Hotkey = "F5".to_string();
        let rec = bm.to_rec();
        let loaded = emBookmarkRec::from_rec(&rec).unwrap();
        assert_eq!(loaded.entry.Name, "Home");
        assert_eq!(loaded.LocationIdentity, "::VcItem:Home:");
        assert_eq!(loaded.Hotkey, "F5");
    }

    #[test]
    fn test_search_start_location() {
        let mut bm1 = emBookmarkRec::default();
        bm1.entry.Name = "First".to_string();
        let mut bm2 = emBookmarkRec::default();
        bm2.entry.Name = "Start".to_string();
        bm2.VisitAtProgramStart = true;
        let bookmarks = emBookmarksRec {
            entries: vec![
                emBookmarkEntryUnion::Bookmark(bm1),
                emBookmarkEntryUnion::Bookmark(bm2),
            ],
        };
        let start = bookmarks.SearchStartLocation();
        assert!(start.is_some());
        assert_eq!(start.unwrap().entry.Name, "Start");
    }

    #[test]
    fn test_group_with_nested_bookmarks() {
        let bm = emBookmarkRec::default();
        let group = emBookmarkGroupRec {
            entry: emBookmarkEntryBase {
                Name: "MyGroup".to_string(),
                ..emBookmarkEntryBase::default()
            },
            Bookmarks: vec![emBookmarkEntryUnion::Bookmark(bm)],
        };
        let root = emBookmarksRec {
            entries: vec![emBookmarkEntryUnion::Group(group)],
        };
        assert_eq!(root.entries.len(), 1);
    }

    #[test]
    fn test_bookmarks_rec_round_trip() {
        let mut bm = emBookmarkRec::default();
        bm.entry.Name = "Test".to_string();
        bm.LocationIdentity = "::test:".to_string();
        bm.VisitAtProgramStart = true;

        let root = emBookmarksRec {
            entries: vec![emBookmarkEntryUnion::Bookmark(bm)],
        };

        let rec = root.to_rec();
        let loaded = emBookmarksRec::from_rec(&rec).unwrap();
        assert_eq!(loaded.entries.len(), 1);

        if let emBookmarkEntryUnion::Bookmark(loaded_bm) = &loaded.entries[0] {
            assert_eq!(loaded_bm.entry.Name, "Test");
            assert_eq!(loaded_bm.LocationIdentity, "::test:");
            assert!(loaded_bm.VisitAtProgramStart);
        } else {
            panic!("expected bookmark");
        }
    }

    #[test]
    fn test_search_start_location_in_group() {
        let mut bm = emBookmarkRec::default();
        bm.entry.Name = "Nested".to_string();
        bm.VisitAtProgramStart = true;

        let group = emBookmarkGroupRec {
            entry: emBookmarkEntryBase::default(),
            Bookmarks: vec![emBookmarkEntryUnion::Bookmark(bm)],
        };

        let root = emBookmarksRec {
            entries: vec![emBookmarkEntryUnion::Group(group)],
        };

        let found = root.SearchStartLocation();
        assert!(found.is_some());
        assert_eq!(found.unwrap().entry.Name, "Nested");
    }

    #[test]
    fn test_clear_start_location() {
        let mut bm1 = emBookmarkRec::default();
        bm1.VisitAtProgramStart = true;
        let mut bm2 = emBookmarkRec::default();
        bm2.VisitAtProgramStart = true;

        let mut root = emBookmarksRec {
            entries: vec![
                emBookmarkEntryUnion::Bookmark(bm1),
                emBookmarkEntryUnion::Bookmark(bm2),
            ],
        };

        root.ClearStartLocation();

        for entry in &root.entries {
            if let emBookmarkEntryUnion::Bookmark(bm) = entry {
                assert!(!bm.VisitAtProgramStart);
            }
        }
    }

    #[test]
    fn test_set_start_location() {
        let mut bm1 = emBookmarkRec::default();
        bm1.LocationIdentity = "::home:".to_string();
        bm1.VisitAtProgramStart = true;

        let mut bm2 = emBookmarkRec::default();
        bm2.LocationIdentity = "::work:".to_string();

        let mut root = emBookmarksRec {
            entries: vec![
                emBookmarkEntryUnion::Bookmark(bm1),
                emBookmarkEntryUnion::Bookmark(bm2),
            ],
        };

        root.SetStartLocation("::work:");

        let found = root.SearchStartLocation().unwrap();
        assert_eq!(found.LocationIdentity, "::work:");
    }

    #[test]
    fn test_search_bookmark_by_hotkey() {
        let mut bm1 = emBookmarkRec::default();
        bm1.Hotkey = "F1".to_string();
        bm1.entry.Name = "Bm1".to_string();

        let mut bm2 = emBookmarkRec::default();
        bm2.Hotkey = "F2".to_string();
        bm2.entry.Name = "Bm2".to_string();

        let root = emBookmarksRec {
            entries: vec![
                emBookmarkEntryUnion::Bookmark(bm1),
                emBookmarkEntryUnion::Bookmark(bm2),
            ],
        };

        let found = root.SearchBookmarkByHotkey("F2");
        assert!(found.is_some());
        assert_eq!(found.unwrap().entry.Name, "Bm2");

        assert!(root.SearchBookmarkByHotkey("F3").is_none());
        assert!(root.SearchBookmarkByHotkey("").is_none());
    }

    #[test]
    fn test_default_colors() {
        let bm = emBookmarkRec::default();
        assert_eq!(bm.entry.BgColor, BOOKMARK_BG_DEFAULT);
        assert_eq!(bm.entry.FgColor, BOOKMARK_FG_DEFAULT);

        let grp = emBookmarkGroupRec::default();
        assert_eq!(grp.entry.BgColor, GROUP_BG_DEFAULT);
        assert_eq!(grp.entry.FgColor, GROUP_FG_DEFAULT);
    }

    #[test]
    fn test_group_rec_round_trip() {
        let mut bm = emBookmarkRec::default();
        bm.entry.Name = "Inner".to_string();

        let group = emBookmarkGroupRec {
            entry: emBookmarkEntryBase {
                Name: "OuterGroup".to_string(),
                ..emBookmarkEntryBase::new(GROUP_BG_DEFAULT, GROUP_FG_DEFAULT)
            },
            Bookmarks: vec![emBookmarkEntryUnion::Bookmark(bm)],
        };

        let rec = group.to_rec();
        let loaded = emBookmarkGroupRec::from_rec(&rec).unwrap();
        assert_eq!(loaded.entry.Name, "OuterGroup");
        assert_eq!(loaded.Bookmarks.len(), 1);

        if let emBookmarkEntryUnion::Bookmark(inner_bm) = &loaded.Bookmarks[0] {
            assert_eq!(inner_bm.entry.Name, "Inner");
        } else {
            panic!("expected bookmark inside group");
        }
    }

    #[test]
    fn test_acquire_singleton() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let m1 = emBookmarksModel::Acquire(&ctx);
        let m2 = emBookmarksModel::Acquire(&ctx);
        assert!(Rc::ptr_eq(&m1, &m2));
    }

    #[test]
    fn test_model_format_name() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let model = emBookmarksModel::Acquire(&ctx);
        let model = model.borrow();
        assert_eq!(model.GetFormatName(), "emBookmarks");
    }
}
