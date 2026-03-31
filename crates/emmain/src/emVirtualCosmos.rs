use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emInstallInfo::emGetConfigDirOverloadable;
use emcore::emRec::{RecError, RecStruct, RecValue};
use emcore::emRecRecord::Record;
use emcore::emRecRecTypes::emColorRec;

// ── emVirtualCosmosItemRec ────────────────────────────────────────────────────

/// A single VirtualCosmos item record.
///
/// Port of C++ `emVirtualCosmosItemRec`.
#[derive(Debug, Clone, PartialEq)]
pub struct emVirtualCosmosItemRec {
    /// Item name (derived from filename without extension).
    pub Name: String,
    pub Title: String,
    pub PosX: f64,
    pub PosY: f64,
    pub Width: f64,
    pub ContentTallness: f64,
    pub BorderScaling: f64,
    pub BackgroundColor: emColor,
    pub BorderColor: emColor,
    pub TitleColor: emColor,
    pub Focusable: bool,
    pub FileName: String,
    pub CopyToUser: bool,
    pub Alternative: i32,
    /// Resolved path to the content file (set by TryPrepareItemFile).
    pub(crate) ItemFilePath: String,
}

impl emVirtualCosmosItemRec {
    /// Resolve and store the item file path.
    ///
    /// Port of C++ `emVirtualCosmosItemRec::TryPrepareItemFile`.
    ///
    /// If `CopyToUser` is false the path is `orig_dir/FileName`. If
    /// `CopyToUser` is true the path would be `user_dir/FileName` (with copy
    /// from orig_dir if absent), but the copy step is not implemented — a
    /// warning is logged instead.
    pub fn TryPrepareItemFile(&mut self, orig_dir: &str, user_dir: &str) {
        let src_path = PathBuf::from(orig_dir).join(&self.FileName);

        if !self.CopyToUser {
            self.ItemFilePath = src_path.to_string_lossy().into_owned();
            return;
        }

        // CopyToUser: ideally copy from orig_dir to user_dir if absent.
        // The copy logic is not yet implemented.
        log::warn!(
            "emVirtualCosmosItemRec: CopyToUser is true for '{}' — \
             copy from '{}' to '{}' is not implemented; using orig_dir path",
            self.FileName,
            orig_dir,
            user_dir,
        );
        self.ItemFilePath = src_path.to_string_lossy().into_owned();
    }
}

impl Default for emVirtualCosmosItemRec {
    fn default() -> Self {
        Self {
            Name: String::new(),
            Title: String::new(),
            PosX: 0.0,
            PosY: 0.0,
            Width: 0.1,
            ContentTallness: 1.0,
            BorderScaling: 1.0,
            BackgroundColor: emColor::from_packed(0xAAAAAAFF),
            BorderColor: emColor::from_packed(0xAAAAAAFF),
            TitleColor: emColor::from_packed(0x000000FF),
            Focusable: true,
            FileName: "unnamed".to_string(),
            CopyToUser: false,
            Alternative: 0,
            ItemFilePath: String::new(),
        }
    }
}

impl Record for emVirtualCosmosItemRec {
    fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        let bg = rec
            .get_struct("backgroundcolor")
            .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
            .unwrap_or_else(|| emColor::from_packed(0xAAAAAAFF));
        let border_color = rec
            .get_struct("bordercolor")
            .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
            .unwrap_or_else(|| emColor::from_packed(0xAAAAAAFF));
        let title_color = rec
            .get_struct("titlecolor")
            .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
            .unwrap_or_else(|| emColor::from_packed(0x000000FF));

        Ok(Self {
            Name: rec.get_str("name").unwrap_or("").to_string(),
            Title: rec.get_str("title").unwrap_or("").to_string(),
            PosX: rec
                .get_double("posx")
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
            PosY: rec
                .get_double("posy")
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
            Width: rec
                .get_double("width")
                .unwrap_or(0.1)
                .clamp(1e-10, 1.0),
            ContentTallness: rec
                .get_double("contenttallness")
                .unwrap_or(1.0)
                .clamp(1e-10, 1e10),
            BorderScaling: rec
                .get_double("borderscaling")
                .unwrap_or(1.0)
                .clamp(0.0, 1e10),
            BackgroundColor: bg,
            BorderColor: border_color,
            TitleColor: title_color,
            Focusable: rec.get_bool("focusable").unwrap_or(true),
            FileName: rec
                .get_str("filename")
                .unwrap_or("unnamed")
                .to_string(),
            CopyToUser: rec.get_bool("copytouser").unwrap_or(false),
            Alternative: rec
                .get_int("alternative")
                .unwrap_or(0)
                .max(0),
            ItemFilePath: String::new(),
        })
    }

    fn to_rec(&self) -> RecStruct {
        let mut s = RecStruct::new();
        s.set_str("name", &self.Name);
        s.set_str("title", &self.Title);
        s.set_double("posx", self.PosX);
        s.set_double("posy", self.PosY);
        s.set_double("width", self.Width);
        s.set_double("contenttallness", self.ContentTallness);
        s.set_double("borderscaling", self.BorderScaling);
        s.SetValue(
            "backgroundcolor",
            RecValue::Struct(emColorRec::ToRecStruct(self.BackgroundColor, true)),
        );
        s.SetValue(
            "bordercolor",
            RecValue::Struct(emColorRec::ToRecStruct(self.BorderColor, true)),
        );
        s.SetValue(
            "titlecolor",
            RecValue::Struct(emColorRec::ToRecStruct(self.TitleColor, true)),
        );
        s.set_bool("focusable", self.Focusable);
        s.set_str("filename", &self.FileName);
        s.set_bool("copytouser", self.CopyToUser);
        s.set_int("alternative", self.Alternative);
        s
    }

    fn SetToDefault(&mut self) {
        *self = Self::default();
    }

    fn IsSetToDefault(&self) -> bool {
        *self == Self::default()
    }
}

// ── emVirtualCosmosModel ──────────────────────────────────────────────────────

/// A loaded VirtualCosmos item (file name, modification time, parsed record).
pub struct LoadedItem {
    pub file_name: String,
    pub mtime: std::time::SystemTime,
    pub item_rec: emVirtualCosmosItemRec,
}

/// Model that loads `.emVcItem` files from the VcItems config directory.
///
/// Port of C++ `emVirtualCosmosModel`.
pub struct emVirtualCosmosModel {
    items_dir: String,
    item_files_dir: String,
    items: Vec<LoadedItem>,
    /// Indices into `items`, sorted by position (PosY then PosX).
    item_recs: Vec<usize>,
}

impl emVirtualCosmosModel {
    /// Acquire the singleton `emVirtualCosmosModel` from the context registry.
    ///
    /// Port of C++ `emVirtualCosmosModel::Acquire`.
    pub fn Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            let mut model = Self {
                items_dir: String::new(),
                item_files_dir: String::new(),
                items: Vec::new(),
                item_recs: Vec::new(),
            };
            model.Reload();
            model
        })
    }

    /// Build a model directly from a list of pre-loaded items (for tests).
    pub fn from_items(items: Vec<LoadedItem>) -> Self {
        let mut model = Self {
            items_dir: String::new(),
            item_files_dir: String::new(),
            items,
            item_recs: Vec::new(),
        };
        model.sort_item_recs();
        model
    }

    /// Reload items from disk.
    ///
    /// Port of C++ `emVirtualCosmosModel::Reload`.
    pub fn Reload(&mut self) {
        let items_dir = emGetConfigDirOverloadable("emMain", Some("VcItems"))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let item_files_dir = emGetConfigDirOverloadable("emMain", Some("VcItemFiles"))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        self.items_dir = items_dir.clone();
        self.item_files_dir = item_files_dir.clone();

        let dir_entries = match std::fs::read_dir(&items_dir) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("emVirtualCosmosModel: cannot read dir '{}': {}", items_dir, e);
                self.items.clear();
                self.item_recs.clear();
                return;
            }
        };

        let mut new_items: Vec<LoadedItem> = Vec::new();

        for entry in dir_entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !file_name.to_lowercase().ends_with(".emvcitem") {
                continue;
            }

            let path = entry.path();
            let mtime = match std::fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(e) => {
                    log::warn!(
                        "emVirtualCosmosModel: cannot stat '{}': {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            // Derive item name: filename without extension.
            let name = std::path::Path::new(&file_name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            let rec = match emcore::emRecRecTypes::emRecFileReader::read(&path) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!(
                        "emVirtualCosmosModel: failed to read '{}': {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let mut item_rec = match emVirtualCosmosItemRec::from_rec(&rec) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!(
                        "emVirtualCosmosModel: failed to parse '{}': {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            item_rec.Name = name;
            item_rec.TryPrepareItemFile(&item_files_dir, &item_files_dir);

            new_items.push(LoadedItem {
                file_name,
                mtime,
                item_rec,
            });
        }

        self.items = new_items;
        self.sort_item_recs();
    }

    /// Sort the `item_recs` index by PosY then PosX (matching C++ CompareItemRecs).
    fn sort_item_recs(&mut self) {
        let mut indices: Vec<usize> = (0..self.items.len()).collect();
        indices.sort_by(|&a, &b| {
            let ra = &self.items[a].item_rec;
            let rb = &self.items[b].item_rec;
            ra.PosY
                .partial_cmp(&rb.PosY)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    ra.PosX
                        .partial_cmp(&rb.PosX)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        self.item_recs = indices;
    }

    /// Return an iterator over item records in position order.
    ///
    /// Port of C++ `emVirtualCosmosModel::GetItemRec`.
    pub fn GetItemRecs(&self) -> impl Iterator<Item = &emVirtualCosmosItemRec> {
        self.item_recs.iter().map(|&i| &self.items[i].item_rec)
    }

    /// Return the number of loaded items.
    ///
    /// Port of C++ `emVirtualCosmosModel::GetItemCount`.
    pub fn GetItemCount(&self) -> usize {
        self.items.len()
    }

    /// Return the items directory path used during the last `Reload`.
    pub fn GetItemsDir(&self) -> &str {
        &self.items_dir
    }

    /// Return the item-files directory path used during the last `Reload`.
    pub fn GetItemFilesDir(&self) -> &str {
        &self.item_files_dir
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use emcore::emRec::RecStruct;
    use emcore::emRecRecord::Record;

    #[test]
    fn test_item_rec_defaults() {
        let item = emVirtualCosmosItemRec::default();
        assert_eq!(item.Title, "");
        assert!((item.PosX).abs() < 1e-10);
        assert!((item.PosY).abs() < 1e-10);
        assert!((item.Width - 0.1).abs() < 1e-10);
        assert!((item.ContentTallness - 1.0).abs() < 1e-10);
        assert!((item.BorderScaling - 1.0).abs() < 1e-10);
        assert!(item.Focusable);
        assert_eq!(item.FileName, "unnamed");
        assert!(!item.CopyToUser);
        assert_eq!(item.Alternative, 0);
    }

    #[test]
    fn test_item_rec_round_trip() {
        let mut item = emVirtualCosmosItemRec::default();
        item.Title = "Home".to_string();
        item.PosX = 0.5;
        item.PosY = 0.3;
        item.Width = 0.2;
        item.FileName = "Home".to_string();
        let rec = item.to_rec();
        let loaded = emVirtualCosmosItemRec::from_rec(&rec).unwrap();
        assert_eq!(loaded.Title, "Home");
        assert!((loaded.PosX - 0.5).abs() < 1e-10);
        assert_eq!(loaded.FileName, "Home");
    }

    #[test]
    fn test_item_rec_clamp_width() {
        let mut rec = RecStruct::new();
        rec.set_double("Width", 5.0); // above max 1.0
        let item = emVirtualCosmosItemRec::from_rec(&rec).unwrap();
        assert!((item.Width - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_model_empty_dir() {
        let model = emVirtualCosmosModel::from_items(vec![]);
        assert_eq!(model.GetItemCount(), 0);
    }

    #[test]
    fn test_model_sorted_by_position() {
        let mut item1 = emVirtualCosmosItemRec::default();
        item1.PosX = 0.8;
        item1.PosY = 0.5;
        item1.Name = "B".to_string();

        let mut item2 = emVirtualCosmosItemRec::default();
        item2.PosX = 0.2;
        item2.PosY = 0.1;
        item2.Name = "A".to_string();

        let model = emVirtualCosmosModel::from_items(vec![
            LoadedItem {
                file_name: "B.emVcItem".to_string(),
                mtime: std::time::SystemTime::UNIX_EPOCH,
                item_rec: item1,
            },
            LoadedItem {
                file_name: "A.emVcItem".to_string(),
                mtime: std::time::SystemTime::UNIX_EPOCH,
                item_rec: item2,
            },
        ]);

        let sorted: Vec<_> = model.GetItemRecs().collect();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].Name, "A"); // PosY=0.1 comes first
        assert_eq!(sorted[1].Name, "B"); // PosY=0.5 comes second
    }

    #[test]
    fn test_item_rec_color_defaults() {
        let item = emVirtualCosmosItemRec::default();
        assert_eq!(item.BackgroundColor, emColor::from_packed(0xAAAAAAFF));
        assert_eq!(item.BorderColor, emColor::from_packed(0xAAAAAAFF));
        assert_eq!(item.TitleColor, emColor::from_packed(0x000000FF));
    }

    #[test]
    fn test_item_rec_color_round_trip() {
        let mut item = emVirtualCosmosItemRec::default();
        item.BackgroundColor = emColor::rgba(10, 20, 30, 200);
        item.TitleColor = emColor::rgba(255, 0, 0, 255);
        let rec = item.to_rec();
        let loaded = emVirtualCosmosItemRec::from_rec(&rec).unwrap();
        assert_eq!(loaded.BackgroundColor, item.BackgroundColor);
        assert_eq!(loaded.TitleColor, item.TitleColor);
    }

    #[test]
    fn test_item_rec_clamp_posx() {
        let mut rec = RecStruct::new();
        rec.set_double("posx", 1.5); // above max 1.0
        let item = emVirtualCosmosItemRec::from_rec(&rec).unwrap();
        assert!((item.PosX - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_try_prepare_item_file_no_copy() {
        let mut item = emVirtualCosmosItemRec::default();
        item.FileName = "foo.tga".to_string();
        item.CopyToUser = false;
        item.TryPrepareItemFile("/orig", "/user");
        assert_eq!(item.ItemFilePath, "/orig/foo.tga");
    }
}
