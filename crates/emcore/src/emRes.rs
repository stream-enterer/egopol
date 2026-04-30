use crate::emImage::emImage;
use crate::emInstallInfo::{emGetInstallPath, InstallDirType};
use crate::emResTga::load_tga;
use std::collections::HashMap;
use std::rc::Rc;

/// A cache that deduplicates resources by string key.
///
/// `purge_unused()` drops entries whose `Rc` has no external references.
pub struct ResourceCache<V> {
    entries: HashMap<String, Rc<V>>,
}

impl<V> ResourceCache<V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn GetOrInsertWith<F>(&mut self, name: &str, f: F) -> Rc<V>
    where
        F: FnOnce() -> V,
    {
        self.entries
            .entry(name.to_owned())
            .or_insert_with(|| Rc::new(f()))
            .clone()
    }

    pub fn GetRec(&self, name: &str) -> Option<Rc<V>> {
        self.entries.get(name).cloned()
    }

    pub fn remove(&mut self, name: &str) -> Option<Rc<V>> {
        self.entries.remove(name)
    }

    /// Remove entries that have no external references (strong count == 1).
    pub fn PurgeUnused(&mut self) {
        self.entries.retain(|_, v| Rc::strong_count(v) > 1);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn IsEmpty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Alias for clippy `len_without_is_empty` lint.
    pub fn is_empty(&self) -> bool {
        self.IsEmpty()
    }
}

impl<V> Default for ResourceCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Port of C++ `emGetInsResImage` (emRes.cpp). Loads a TGA from the installed
/// resource tree (`$EM_DIR/res/<prj>/<sub_path>`). Returns a blank 1×1 RGBA
/// image on any error — matches C++ graceful degradation.
pub fn emGetInsResImage(prj: &str, sub_path: &str) -> emImage {
    let path = match emGetInstallPath(InstallDirType::Res, prj, Some(sub_path)) {
        Ok(p) => p,
        Err(_) => return blank_image(),
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return blank_image(),
    };
    load_tga(&data).unwrap_or_else(|_| blank_image())
}

fn blank_image() -> emImage {
    let mut img = emImage::new(1, 1, 4);
    img.set_pixel_channel(0, 0, 3, 255);
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emGetInsResImage_returns_valid_image_when_em_dir_set() {
        let workspace = std::env::current_dir()
            .unwrap()
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists() && p.join("res").exists())
            .unwrap()
            .to_path_buf();
        std::env::set_var("EM_DIR", &workspace);
        let img = emGetInsResImage("emTest", "icons/teddy.tga");
        assert!(
            img.GetWidth() > 1 && img.GetHeight() > 1,
            "teddy must load as non-trivial image"
        );
        std::env::remove_var("EM_DIR");
    }

    #[test]
    fn emGetInsResImage_returns_blank_on_missing_file() {
        std::env::set_var("EM_DIR", "/nonexistent");
        let img = emGetInsResImage("emTest", "icons/teddy.tga");
        assert_eq!(img.GetWidth(), 1);
        assert_eq!(img.GetHeight(), 1);
        std::env::remove_var("EM_DIR");
    }
}
