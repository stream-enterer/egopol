use std::path::{Path, PathBuf};

use crate::panel::{PanelBehavior, PanelState};
use crate::render::Painter;

use super::border::{Border, InnerBorderType, OuterBorderType};
use super::look::Look;

/// Data associated with each file entry in the listing.
#[derive(Clone, Debug)]
pub struct FileItemData {
    pub is_directory: bool,
    pub is_readable: bool,
    pub is_hidden: bool,
}

/// A file selection box widget for browsing and selecting files.
///
/// Port of C++ `emFileSelectionBox`. Provides a file browser with:
/// - A text field showing the current directory path
/// - A checkbox to toggle showing hidden files
/// - A list of files/directories in the current directory
/// - A text field for entering/editing the file name
/// - A filter list for file type filtering
pub struct FileSelectionBox {
    border: Border,
    multi_selection_enabled: bool,
    parent_dir: PathBuf,
    selected_names: Vec<String>,
    filters: Vec<String>,
    selected_filter_index: i32,
    hidden_files_shown: bool,
    triggered_file_name: String,
    parent_dir_field_hidden: bool,
    hidden_check_box_hidden: bool,
    name_field_hidden: bool,
    filter_hidden: bool,
    listing_invalid: bool,
    listing: Vec<(String, FileItemData)>,
}

impl FileSelectionBox {
    pub fn new(caption: &str) -> Self {
        let parent_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            border: Border::new(OuterBorderType::Group)
                .with_inner(InnerBorderType::Group)
                .with_caption(caption),
            multi_selection_enabled: false,
            parent_dir,
            selected_names: Vec::new(),
            filters: Vec::new(),
            selected_filter_index: -1,
            hidden_files_shown: false,
            triggered_file_name: String::new(),
            parent_dir_field_hidden: false,
            hidden_check_box_hidden: false,
            name_field_hidden: false,
            filter_hidden: false,
            listing_invalid: true,
            listing: Vec::new(),
        }
    }

    pub fn is_multi_selection_enabled(&self) -> bool {
        self.multi_selection_enabled
    }

    pub fn set_multi_selection_enabled(&mut self, enabled: bool) {
        if self.multi_selection_enabled != enabled {
            if !enabled && self.selected_names.len() > 1 {
                let first = self.selected_names[0].clone();
                self.set_selected_name(&first);
            }
            self.multi_selection_enabled = enabled;
        }
    }

    pub fn parent_directory(&self) -> &Path {
        &self.parent_dir
    }

    pub fn set_parent_directory(&mut self, parent_directory: &Path) {
        let abs_path = if parent_directory.is_absolute() {
            parent_directory.to_path_buf()
        } else {
            std::fs::canonicalize(parent_directory)
                .unwrap_or_else(|_| parent_directory.to_path_buf())
        };

        if self.parent_dir != abs_path {
            self.parent_dir = abs_path;
            self.triggered_file_name.clear();
            self.invalidate_listing();
        }
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.selected_names.first().map(|s| s.as_str())
    }

    pub fn selected_names(&self) -> &[String] {
        &self.selected_names
    }

    pub fn set_selected_name(&mut self, name: &str) {
        if name.is_empty() {
            if !self.selected_names.is_empty() {
                self.selected_names.clear();
            }
        } else if self.selected_names.len() != 1 || self.selected_names[0] != name {
            self.selected_names = vec![name.to_string()];
        }
    }

    pub fn set_selected_names(&mut self, names: &[String]) {
        let mut sorted = names.to_vec();
        sorted.sort();

        if sorted != self.selected_names {
            self.selected_names = sorted;
        }
    }

    pub fn clear_selection(&mut self) {
        self.set_selected_name("");
    }

    pub fn selected_path(&self) -> PathBuf {
        if let Some(name) = self.selected_names.first() {
            self.parent_dir.join(name)
        } else {
            self.parent_dir.clone()
        }
    }

    pub fn set_selected_path(&mut self, selected_path: &Path) {
        let abs_path = if selected_path.is_absolute() {
            selected_path.to_path_buf()
        } else {
            std::fs::canonicalize(selected_path).unwrap_or_else(|_| selected_path.to_path_buf())
        };

        if abs_path.is_dir() {
            self.set_parent_directory(&abs_path);
            self.clear_selection();
        } else {
            if let Some(parent) = abs_path.parent() {
                self.set_parent_directory(parent);
            }
            if let Some(name) = abs_path.file_name() {
                self.set_selected_name(&name.to_string_lossy());
            }
        }
    }

    pub fn filters(&self) -> &[String] {
        &self.filters
    }

    pub fn set_filters(&mut self, filters: &[String]) {
        if self.filters == filters {
            return;
        }

        self.filters = filters.to_vec();
        let count = self.filters.len() as i32;
        if self.selected_filter_index >= count {
            self.selected_filter_index = count - 1;
        } else if self.selected_filter_index < 0 && count > 0 {
            self.selected_filter_index = 0;
        }
        self.invalidate_listing();
    }

    pub fn selected_filter_index(&self) -> i32 {
        self.selected_filter_index
    }

    pub fn set_selected_filter_index(&mut self, index: i32) {
        let clamped = if index < 0 || index >= self.filters.len() as i32 {
            -1
        } else {
            index
        };
        if self.selected_filter_index != clamped {
            self.selected_filter_index = clamped;
            self.invalidate_listing();
        }
    }

    pub fn are_hidden_files_shown(&self) -> bool {
        self.hidden_files_shown
    }

    pub fn set_hidden_files_shown(&mut self, shown: bool) {
        if self.hidden_files_shown != shown {
            self.hidden_files_shown = shown;
            self.invalidate_listing();
        }
    }

    pub fn triggered_file_name(&self) -> &str {
        &self.triggered_file_name
    }

    pub fn trigger_file(&mut self, name: &str) {
        self.triggered_file_name = name.to_string();
    }

    /// Enter a sub-directory by name.
    pub fn enter_sub_dir(&mut self, name: &str) {
        let path = self.parent_dir.join(name);
        if name == ".." {
            self.set_parent_directory(&path);
            self.clear_selection();
        } else if path.is_dir() {
            // Check readability by attempting to read the directory.
            if std::fs::read_dir(&path).is_ok() {
                self.set_parent_directory(&path);
                self.clear_selection();
            }
        }
    }

    pub fn is_parent_dir_field_hidden(&self) -> bool {
        self.parent_dir_field_hidden
    }

    pub fn set_parent_dir_field_hidden(&mut self, hidden: bool) {
        self.parent_dir_field_hidden = hidden;
    }

    pub fn is_hidden_check_box_hidden(&self) -> bool {
        self.hidden_check_box_hidden
    }

    pub fn set_hidden_check_box_hidden(&mut self, hidden: bool) {
        self.hidden_check_box_hidden = hidden;
    }

    pub fn is_name_field_hidden(&self) -> bool {
        self.name_field_hidden
    }

    pub fn set_name_field_hidden(&mut self, hidden: bool) {
        self.name_field_hidden = hidden;
    }

    pub fn is_filter_hidden(&self) -> bool {
        self.filter_hidden
    }

    pub fn set_filter_hidden(&mut self, hidden: bool) {
        self.filter_hidden = hidden;
    }

    /// Reload the directory listing, applying filters and hidden-file settings.
    pub fn reload_listing(&mut self) {
        let mut entries = Vec::new();

        let dir_entries = match std::fs::read_dir(&self.parent_dir) {
            Ok(rd) => rd,
            Err(_) => {
                self.listing = entries;
                self.listing_invalid = false;
                return;
            }
        };

        for entry in dir_entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let is_directory = path.is_dir();
            let is_readable =
                std::fs::read_dir(&path).is_ok() || std::fs::File::open(&path).is_ok();
            let is_hidden = name.starts_with('.');

            let data = FileItemData {
                is_directory,
                is_readable,
                is_hidden,
            };

            // Filter hidden files.
            if !self.hidden_files_shown && is_hidden {
                continue;
            }

            // Apply file type filter (directories pass through).
            if self.selected_filter_index >= 0
                && (self.selected_filter_index as usize) < self.filters.len()
                && !is_directory
                && !match_file_name_filter(
                    &name,
                    &self.filters[self.selected_filter_index as usize],
                )
            {
                continue;
            }

            entries.push((name, data));
        }

        // Sort: directories first, then by name (locale-insensitive).
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Add ".." entry at the beginning if not at root.
        if self.parent_dir != Path::new("/") {
            entries.insert(
                0,
                (
                    "..".to_string(),
                    FileItemData {
                        is_directory: true,
                        is_readable: true,
                        is_hidden: false,
                    },
                ),
            );
        }

        self.listing = entries;
        self.listing_invalid = false;
    }

    /// Get the current directory listing.
    pub fn listing(&self) -> &[(String, FileItemData)] {
        &self.listing
    }

    /// Whether the listing needs to be reloaded.
    pub fn is_listing_invalid(&self) -> bool {
        self.listing_invalid
    }

    fn invalidate_listing(&mut self) {
        self.listing_invalid = true;
    }

    pub fn border(&self) -> &Border {
        &self.border
    }

    pub fn border_mut(&mut self) -> &mut Border {
        &mut self.border
    }
}

/// Match a filename against a filter string of the form `Description (*.ext1 *.ext2)`.
///
/// Port of C++ `emFileSelectionBox::MatchFileNameFilter`.
pub(crate) fn match_file_name_filter(file_name: &str, filter: &str) -> bool {
    // Find the patterns between the last '(' and last ')'.
    let pattern_range = match (filter.rfind('('), filter.rfind(')')) {
        (Some(start), Some(end)) if start < end => &filter[start + 1..end],
        _ => filter,
    };

    // Split patterns by whitespace, semicolons, commas, or pipes.
    for pattern in
        pattern_range.split(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == '|')
    {
        let pattern = pattern.trim();
        if !pattern.is_empty() && match_file_name_pattern(file_name, pattern) {
            return true;
        }
    }
    false
}

/// Match a filename against a glob-like pattern with `*` wildcards.
/// Matching is case-insensitive.
///
/// Port of C++ `emFileSelectionBox::MatchFileNamePattern`.
fn match_file_name_pattern(file_name: &str, pattern: &str) -> bool {
    let fname_bytes = file_name.as_bytes();
    let pat_bytes = pattern.as_bytes();
    match_pattern_recursive(fname_bytes, pat_bytes)
}

fn match_pattern_recursive(fname: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() {
        return fname.is_empty();
    }
    if pattern[0] == b'*' {
        // Try matching the rest of the pattern at each position.
        for i in 0..=fname.len() {
            if match_pattern_recursive(&fname[i..], &pattern[1..]) {
                return true;
            }
        }
        return false;
    }
    if fname.is_empty() {
        return pattern.is_empty();
    }
    if !fname[0].eq_ignore_ascii_case(&pattern[0]) {
        return false;
    }
    match_pattern_recursive(&fname[1..], &pattern[1..])
}

impl PanelBehavior for FileSelectionBox {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        let look = Look::new();
        self.border
            .paint_border(painter, w, h, &look, state.enabled, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matching_all_files() {
        assert!(match_file_name_filter("anything.txt", "All files (*)"));
        assert!(match_file_name_filter("", "All files (*)"));
    }

    #[test]
    fn filter_matching_extension() {
        assert!(match_file_name_filter("image.tga", "Targa files (*.tga)"));
        assert!(!match_file_name_filter("image.png", "Targa files (*.tga)"));
    }

    #[test]
    fn filter_matching_case_insensitive() {
        assert!(match_file_name_filter("FILE.TGA", "Targa files (*.tga)"));
        assert!(match_file_name_filter("file.Tga", "Targa files (*.tga)"));
    }

    #[test]
    fn filter_matching_multiple_patterns() {
        assert!(match_file_name_filter(
            "page.htm",
            "HTML files (*.htm *.html)"
        ));
        assert!(match_file_name_filter(
            "page.html",
            "HTML files (*.htm *.html)"
        ));
        assert!(!match_file_name_filter(
            "page.txt",
            "HTML files (*.htm *.html)"
        ));
    }

    #[test]
    fn new_file_selection_box() {
        let fsb = FileSelectionBox::new("Files");
        assert!(!fsb.is_multi_selection_enabled());
        assert!(fsb.selected_names().is_empty());
        assert_eq!(fsb.selected_filter_index(), -1);
        assert!(!fsb.are_hidden_files_shown());
    }

    #[test]
    fn set_selected_name() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_selected_name("test.txt");
        assert_eq!(fsb.selected_name(), Some("test.txt"));

        fsb.set_selected_name("");
        assert_eq!(fsb.selected_name(), None);
    }

    #[test]
    fn set_filters() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_filters(&[
            "All files (*)".to_string(),
            "Images (*.png *.jpg)".to_string(),
        ]);
        assert_eq!(fsb.filters().len(), 2);
        assert_eq!(fsb.selected_filter_index(), 0);
    }

    #[test]
    fn enter_parent_dir() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_parent_directory(Path::new("/tmp"));
        fsb.set_selected_name("foo");
        fsb.enter_sub_dir("..");
        assert!(fsb.selected_names().is_empty());
    }

    #[test]
    fn visibility_toggles() {
        let mut fsb = FileSelectionBox::new("Files");
        assert!(!fsb.is_parent_dir_field_hidden());
        fsb.set_parent_dir_field_hidden(true);
        assert!(fsb.is_parent_dir_field_hidden());

        assert!(!fsb.is_name_field_hidden());
        fsb.set_name_field_hidden(true);
        assert!(fsb.is_name_field_hidden());

        assert!(!fsb.is_filter_hidden());
        fsb.set_filter_hidden(true);
        assert!(fsb.is_filter_hidden());
    }
}
