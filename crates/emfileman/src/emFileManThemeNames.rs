// SPLIT: emFileManTheme.h — emFileManThemeNames split into separate file per one-type-per-file rule.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::emFileManTheme::{GetThemesDirPath, THEME_FILE_ENDING};

struct ThemeAR {
    name: String,
    aspect_ratio: String,
    height: f64,
}

struct ThemeStyle {
    display_name: String,
    display_icon: String,
    theme_ars: Vec<ThemeAR>,
}

pub struct emFileManThemeNames {
    styles: Vec<ThemeStyle>,
    name_to_packed_index: BTreeMap<String, (usize, usize)>,
    change_generation: Rc<Cell<u64>>,
    theme_dir: PathBuf,
    theme_dir_mtime: u64,
}

/// Port of C++ `emFileManThemeNames::HeightToAspectRatioString`.
/// Tries denominators 1..10, finds best n:d ratio for the given height.
pub fn HeightToAspectRatioString(height: f64) -> String {
    let mut best_n: i32 = 1;
    let mut best_d: i32 = 1;
    for d in 1..=10_i32 {
        let mut n = (f64::from(d) / height + 0.5) as i32;
        if n < 1 {
            n = 1;
        }
        if (height * f64::from(n) / f64::from(d) - 1.0).abs()
            < (height * f64::from(best_n) / f64::from(best_d) - 1.0).abs() - 0.001
        {
            best_n = n;
            best_d = d;
        }
    }
    format!("{best_n}:{best_d}")
}

pub fn discover_themes_from_dir(dir: &Path) -> emFileManThemeNames {
    let mut owned: Vec<(String, String, String, f64)> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if !name_str.ends_with(THEME_FILE_ENDING) {
                continue;
            }
            let theme_name = name_str
                .strip_suffix(THEME_FILE_ENDING)
                .unwrap_or(&name_str)
                .to_string();
            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let mut display_name = String::new();
            let mut display_icon = String::new();
            let mut height: f64 = 0.0;
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("DisplayName") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        display_name = val.trim().trim_matches('"').to_string();
                    }
                } else if let Some(val) = trimmed.strip_prefix("DisplayIcon") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        display_icon = val.trim().trim_matches('"').to_string();
                    }
                } else if let Some(val) = trimmed.strip_prefix("Height") {
                    if let Some(val) = val.trim_start().strip_prefix('=') {
                        if let Ok(h) = val.trim().parse::<f64>() {
                            height = h;
                        }
                    }
                }
            }
            owned.push((theme_name, display_name, display_icon, height));
        }
    }
    let refs: Vec<(&str, &str, &str, f64)> = owned
        .iter()
        .map(|(n, dn, di, h)| (n.as_str(), dn.as_str(), di.as_str(), *h))
        .collect();
    emFileManThemeNames::from_themes(&refs)
}

fn dir_mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .map_err(|_| std::io::Error::other("time"))
        })
        .unwrap_or(0)
}

impl emFileManThemeNames {
    /// Construct from a slice of `(name, display_name, display_icon, height)` tuples.
    /// This mirrors the C++ constructor logic but takes pre-parsed theme data for testability.
    pub fn from_themes(entries: &[(&str, &str, &str, f64)]) -> Self {
        let mut sorted_entries: Vec<_> = entries.to_vec();
        sorted_entries.sort_by(|a, b| a.0.cmp(b.0));

        let mut styles: Vec<ThemeStyle> = Vec::new();
        let mut name_to_packed_index: BTreeMap<String, (usize, usize)> = BTreeMap::new();

        for &(name, display_name, display_icon, height) in &sorted_entries {
            let aspect_ratio = HeightToAspectRatioString(height);

            // Find existing style by display_name, or create new one
            let style_idx = styles
                .iter()
                .position(|s| s.display_name == display_name)
                .unwrap_or_else(|| {
                    styles.push(ThemeStyle {
                        display_name: display_name.to_string(),
                        display_icon: display_icon.to_string(),
                        theme_ars: Vec::new(),
                    });
                    styles.len() - 1
                });

            // Insert sorted by height within the style
            let style = &mut styles[style_idx];
            let ar_idx = style
                .theme_ars
                .iter()
                .position(|ar| ar.height > height)
                .unwrap_or(style.theme_ars.len());

            style.theme_ars.insert(
                ar_idx,
                ThemeAR {
                    name: name.to_string(),
                    aspect_ratio,
                    height,
                },
            );

            name_to_packed_index.insert(name.to_string(), (style_idx, ar_idx));
        }

        // Rebuild packed index after all insertions (insertion offsets may have shifted)
        name_to_packed_index.clear();
        for (style_idx, style) in styles.iter().enumerate() {
            for (ar_idx, ar) in style.theme_ars.iter().enumerate() {
                name_to_packed_index.insert(ar.name.clone(), (style_idx, ar_idx));
            }
        }

        Self {
            styles,
            name_to_packed_index,
            change_generation: Rc::new(Cell::new(0)),
            theme_dir: PathBuf::new(),
            theme_dir_mtime: 0,
        }
    }

    pub fn GetThemeStyleCount(&self) -> usize {
        self.styles.len()
    }

    pub fn GetThemeAspectRatioCount(&self, style_index: usize) -> usize {
        self.styles
            .get(style_index)
            .map_or(0, |s| s.theme_ars.len())
    }

    pub fn GetThemeName(&self, style_index: usize, ar_index: usize) -> Option<String> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.name.clone())
    }

    pub fn GetDefaultThemeName(&self) -> String {
        if self.IsExistingThemeName("Glass1") {
            return "Glass1".to_string();
        }
        self.GetThemeName(0, 0).unwrap_or_default()
    }

    pub fn GetThemeStyleDisplayName(&self, style_index: usize) -> Option<&str> {
        self.styles.get(style_index).map(|s| s.display_name.as_str())
    }

    pub fn GetThemeStyleDisplayIcon(&self, style_index: usize) -> Option<&str> {
        self.styles.get(style_index).map(|s| s.display_icon.as_str())
    }

    pub fn GetThemeAspectRatio(&self, style_index: usize, ar_index: usize) -> Option<&str> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.aspect_ratio.as_str())
    }

    pub fn GetThemeHeight(&self, style_index: usize, ar_index: usize) -> Option<f64> {
        self.styles
            .get(style_index)
            .and_then(|s| s.theme_ars.get(ar_index))
            .map(|ar| ar.height)
    }

    pub fn IsExistingThemeName(&self, name: &str) -> bool {
        self.name_to_packed_index.contains_key(name)
    }

    pub fn GetThemeStyleIndex(&self, name: &str) -> Option<usize> {
        self.name_to_packed_index.get(name).map(|&(s, _)| s)
    }

    pub fn GetThemeAspectRatioIndex(&self, name: &str) -> Option<usize> {
        self.name_to_packed_index.get(name).map(|&(_, a)| a)
    }

    pub fn Acquire(ctx: &Rc<emcore::emContext::emContext>) -> Rc<RefCell<Self>> {
        ctx.acquire::<Self>("", || {
            let theme_dir = GetThemesDirPath().unwrap_or_default();
            let mut catalog = discover_themes_from_dir(&theme_dir);
            catalog.change_generation = Rc::new(Cell::new(0));
            catalog.theme_dir_mtime = dir_mtime(&theme_dir);
            catalog.theme_dir = theme_dir;
            catalog
        })
    }

    pub fn GetChangeGeneration(&self) -> u64 {
        self.change_generation.get()
    }

    pub fn Cycle(&mut self) -> bool {
        let current_mtime = dir_mtime(&self.theme_dir);
        if current_mtime != self.theme_dir_mtime {
            self.theme_dir_mtime = current_mtime;
            let new_catalog = discover_themes_from_dir(&self.theme_dir);
            self.styles = new_catalog.styles;
            self.name_to_packed_index = new_catalog.name_to_packed_index;
            self.change_generation
                .set(self.change_generation.get() + 1);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_to_aspect_ratio_string() {
        assert_eq!(HeightToAspectRatioString(1.0), "1:1");
        assert_eq!(HeightToAspectRatioString(0.5), "2:1");
        assert_eq!(HeightToAspectRatioString(2.0), "1:2");
    }

    #[test]
    fn empty_catalog() {
        let names = emFileManThemeNames::from_themes(&[]);
        assert_eq!(names.GetThemeStyleCount(), 0);
        assert!(!names.IsExistingThemeName("anything"));
        assert!(names.GetDefaultThemeName().is_empty());
    }

    #[test]
    fn single_theme() {
        let names = emFileManThemeNames::from_themes(&[("Glass1", "Glass", "", 1.5)]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert!(names.IsExistingThemeName("Glass1"));
        assert_eq!(names.GetThemeStyleIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeAspectRatioIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
    }

    #[test]
    fn multiple_aspect_ratios_same_style() {
        let names = emFileManThemeNames::from_themes(&[
            ("Glass1", "Glass", "", 1.0),
            ("Glass2", "Glass", "", 2.0),
        ]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert_eq!(names.GetThemeAspectRatioCount(0), 2);
        // Sorted by height: Glass1 (1.0) before Glass2 (2.0)
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
        assert_eq!(names.GetThemeName(0, 1), Some("Glass2".to_string()));
    }

    #[test]
    fn default_theme_name_prefers_glass1() {
        let names = emFileManThemeNames::from_themes(&[
            ("Glass1", "Glass", "", 1.0),
            ("Other1", "Other", "", 1.0),
        ]);
        assert_eq!(names.GetDefaultThemeName(), "Glass1");
    }

    #[test]
    fn default_theme_falls_back_to_first() {
        let names = emFileManThemeNames::from_themes(&[("Metal1", "Metal", "", 1.0)]);
        assert_eq!(names.GetDefaultThemeName(), "Metal1");
    }

    #[test]
    fn style_display_name() {
        let names =
            emFileManThemeNames::from_themes(&[("Glass1", "Glass Display", "icon.png", 1.0)]);
        assert_eq!(names.GetThemeStyleDisplayName(0), Some("Glass Display"));
        assert_eq!(names.GetThemeStyleDisplayIcon(0), Some("icon.png"));
    }

    #[test]
    fn acquire_singleton() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let t1 = emFileManThemeNames::Acquire(&ctx);
        let t2 = emFileManThemeNames::Acquire(&ctx);
        assert!(Rc::ptr_eq(&t1, &t2));
    }

    #[test]
    fn discover_from_directory() {
        let dir = std::env::temp_dir().join("emcore_test_themes_disc");
        let _ = std::fs::create_dir_all(&dir);
        let content = "emFileManTheme\nDisplayName = \"TestStyle\"\nDisplayIcon = \"icon.tga\"\nHeight = 0.6\n";
        std::fs::write(dir.join("Test1.emFileManTheme"), content).expect("write");
        let names = discover_themes_from_dir(&dir);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert!(names.IsExistingThemeName("Test1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn change_generation_starts_at_zero() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let names = emFileManThemeNames::Acquire(&ctx);
        assert_eq!(names.borrow().GetChangeGeneration(), 0);
    }
}
