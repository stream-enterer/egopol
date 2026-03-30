// Selection subsystem and command tree of emFileManModel.

use std::path::{Path, PathBuf};

use emcore::emColor::emColor;
use emcore::emImage::emImage;
use emcore::emImageFile::load_image_from_file;
use emcore::emLook::emLook;
use emcore::emStd2::emCalcHashCode;

// ---------------------------------------------------------------------------
// Command tree
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandType {
    Command,
    Group,
    Separator,
}

#[derive(Clone, Debug)]
pub struct CommandNode {
    pub cmd_path: String,
    pub command_type: CommandType,
    pub order: f64,
    pub interpreter: String,
    pub dir: String,
    pub default_for: String,
    pub caption: String,
    pub description: String,
    pub icon: Option<emImage>,
    pub look: emLook,
    pub hotkey: String,
    pub border_scaling: f64,
    pub pref_child_tallness: f64,
    pub children: Vec<CommandNode>,
    pub dir_crc: u64,
}

impl Default for CommandNode {
    fn default() -> Self {
        Self {
            cmd_path: String::new(),
            command_type: CommandType::Command,
            order: 0.0,
            interpreter: String::new(),
            dir: String::new(),
            default_for: String::new(),
            caption: String::new(),
            description: String::new(),
            icon: None,
            look: emLook::default(),
            hotkey: String::new(),
            border_scaling: 0.0,
            pref_child_tallness: 0.0,
            children: Vec::new(),
            dir_crc: 0,
        }
    }
}

/// Parse `# [[BEGIN PROPERTIES]]` ... `# [[END PROPERTIES]]` blocks from
/// command file content. Each property line has the form `# Key = Value`.
pub fn parse_command_properties(content: &str, cmd_path: &str) -> Result<CommandNode, String> {
    let mut node = CommandNode {
        cmd_path: cmd_path.to_string(),
        ..CommandNode::default()
    };

    let mut in_block = false;
    let mut found_type = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "# [[BEGIN PROPERTIES]]" {
            in_block = true;
            continue;
        }
        if trimmed == "# [[END PROPERTIES]]" {
            break;
        }
        if !in_block {
            continue;
        }

        // Strip leading "# " and parse "Key = Value"
        let stripped = if let Some(s) = trimmed.strip_prefix("# ") {
            s
        } else {
            continue;
        };

        let Some((key, value)) = stripped.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "Type" => {
                node.command_type = match value {
                    "Command" => CommandType::Command,
                    "Group" => CommandType::Group,
                    "Separator" => CommandType::Separator,
                    other => return Err(format!("Unknown command type: {other}")),
                };
                found_type = true;
            }
            "Order" => {
                node.order = value
                    .parse::<f64>()
                    .map_err(|e| format!("Bad Order value: {e}"))?;
            }
            "Interpreter" => {
                node.interpreter = value.to_string();
            }
            "Directory" | "Dir" => {
                // Resolve relative to command file's parent directory
                let parent = Path::new(cmd_path)
                    .parent()
                    .unwrap_or_else(|| Path::new(""));
                let resolved = parent.join(value);
                node.dir = resolved.to_string_lossy().to_string();
            }
            "DefaultFor" => {
                node.default_for = value.to_string();
            }
            "Caption" => {
                if node.caption.is_empty() {
                    node.caption = value.to_string();
                } else {
                    node.caption.push('\n');
                    node.caption.push_str(value);
                }
            }
            "Description" | "Descr" => {
                if node.description.is_empty() {
                    node.description = value.to_string();
                } else {
                    node.description.push('\n');
                    node.description.push_str(value);
                }
            }
            "Hotkey" => {
                node.hotkey = value.to_string();
            }
            "BorderScaling" => {
                node.border_scaling = value
                    .parse::<f64>()
                    .map_err(|e| format!("Bad BorderScaling value: {e}"))?;
            }
            "PrefChildTallness" => {
                node.pref_child_tallness = value
                    .parse::<f64>()
                    .map_err(|e| format!("Bad PrefChildTallness value: {e}"))?;
            }
            "BgColor" => {
                if let Some(color) = emColor::TryParse(value) {
                    node.look.bg_color = color;
                }
            }
            "FgColor" => {
                if let Some(color) = emColor::TryParse(value) {
                    node.look.fg_color = color;
                }
            }
            "ButtonBgColor" => {
                if let Some(color) = emColor::TryParse(value) {
                    node.look.button_bg_color = color;
                }
            }
            "ButtonFgColor" => {
                if let Some(color) = emColor::TryParse(value) {
                    node.look.button_fg_color = color;
                }
            }
            "Icon" => {
                let icon_path = if Path::new(value).is_absolute() {
                    PathBuf::from(value)
                } else {
                    Path::new(cmd_path)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(value)
                };
                node.icon = load_image_from_file(&icon_path);
            }
            _ => {}
        }
    }

    if !found_type {
        return Err("Missing Type property".to_string());
    }

    Ok(node)
}

/// Returns true if the filename has an allowed command file extension.
/// Allowed (case-insensitive): `.js`, `.pl`, `.props`, `.py`, `.sh`
pub fn check_command_file_ending(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".js")
        || lower.ends_with(".pl")
        || lower.ends_with(".props")
        || lower.ends_with(".py")
        || lower.ends_with(".sh")
}

/// Check whether `cmd` is a default command for the given file path.
/// Returns a priority value (higher = better match), or 0 if no match.
///
/// For extension matching this does NOT check the filesystem — it only
/// compares the suffix. For the keywords "file" and "directory" it uses
/// `Path::is_file()` / `Path::is_dir()`.
#[allow(non_snake_case)]
pub fn CheckDefaultCommand(cmd: &CommandNode, file_path: &str) -> i32 {
    if cmd.command_type != CommandType::Command {
        return 0;
    }
    if cmd.default_for.is_empty() {
        return 0;
    }
    if cmd.default_for == "file" {
        return if Path::new(file_path).is_file() { 1 } else { 0 };
    }
    if cmd.default_for == "directory" {
        return if Path::new(file_path).is_dir() { 1 } else { 0 };
    }

    let path_len = file_path.len();
    let path_lower = file_path.to_ascii_lowercase();
    let mut best_len: usize = 0;

    for ext in cmd.default_for.split(':') {
        let ext_len = ext.len();
        if ext_len > best_len && ext_len <= path_len {
            let ext_lower = ext.to_ascii_lowercase();
            if path_lower.ends_with(&ext_lower) {
                best_len = ext_len;
            }
        }
    }

    if best_len > 0 {
        (best_len + 1) as i32
    } else {
        0
    }
}

/// Depth-first search for the best default command for a file path.
/// Returns the `CommandNode` with the highest priority match, or `None`.
#[allow(non_snake_case)]
pub fn SearchDefaultCommandFor<'a>(
    root: &'a CommandNode,
    file_path: &str,
) -> Option<&'a CommandNode> {
    let mut best_cmd: Option<&'a CommandNode> = None;
    let mut best_pri: i32 = 0;

    search_default_recursive(root, file_path, &mut best_cmd, &mut best_pri);
    best_cmd
}

fn search_default_recursive<'a>(
    parent: &'a CommandNode,
    file_path: &str,
    best_cmd: &mut Option<&'a CommandNode>,
    best_pri: &mut i32,
) {
    // Check CT_COMMAND children
    for child in &parent.children {
        if child.command_type == CommandType::Command {
            let pri = CheckDefaultCommand(child, file_path);
            if pri > *best_pri {
                *best_pri = pri;
                *best_cmd = Some(child);
            }
        }
    }
    // Recurse into CT_GROUP children
    for child in &parent.children {
        if child.command_type == CommandType::Group {
            search_default_recursive(child, file_path, best_cmd, best_pri);
        }
    }
}

struct SelEntry {
    hash_code: i32,
    path: String,
}

pub struct SelectionManager {
    sel: [Vec<SelEntry>; 2], // 0=source, 1=target
    #[allow(dead_code)]
    shift_tgt_sel_path: String,
    sel_cmd_counter: u32,
}

/// Binary search over a sorted `Vec<SelEntry>`, ordered by `(hash_code, path)`.
/// Returns `Ok(index)` if found, `Err(insert_pos)` if not.
fn search_selection(sel: &[SelEntry], hash_code: i32, path: &str) -> Result<usize, usize> {
    let mut lo: usize = 0;
    let mut hi: usize = sel.len();
    while lo < hi {
        let mid = (lo + hi) >> 1;
        let entry = &sel[mid];
        if entry.hash_code > hash_code {
            hi = mid;
        } else if entry.hash_code < hash_code {
            lo = mid + 1;
        } else {
            match entry.path.as_str().cmp(path) {
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Equal => return Ok(mid),
            }
        }
    }
    Err(hi)
}

impl SelectionManager {
    pub fn new() -> Self {
        Self {
            sel: [Vec::new(), Vec::new()],
            shift_tgt_sel_path: String::new(),
            sel_cmd_counter: 0,
        }
    }

    // --- Source selection ---

    pub fn GetSourceSelectionCount(&self) -> usize {
        self.sel[0].len()
    }

    pub fn GetSourceSelection(&self, index: usize) -> &str {
        &self.sel[0][index].path
    }

    pub fn IsSelectedAsSource(&self, path: &str) -> bool {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        search_selection(&self.sel[0], hash_code, path).is_ok()
    }

    pub fn SelectAsSource(&mut self, path: &str) {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        if let Err(pos) = search_selection(&self.sel[0], hash_code, path) {
            self.sel[0].insert(
                pos,
                SelEntry {
                    hash_code,
                    path: path.to_string(),
                },
            );
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    pub fn DeselectAsSource(&mut self, path: &str) {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        if let Ok(pos) = search_selection(&self.sel[0], hash_code, path) {
            self.sel[0].remove(pos);
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    pub fn ClearSourceSelection(&mut self) {
        if !self.sel[0].is_empty() {
            self.sel[0].clear();
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    // --- Target selection ---

    pub fn GetTargetSelectionCount(&self) -> usize {
        self.sel[1].len()
    }

    pub fn GetTargetSelection(&self, index: usize) -> &str {
        &self.sel[1][index].path
    }

    pub fn IsSelectedAsTarget(&self, path: &str) -> bool {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        search_selection(&self.sel[1], hash_code, path).is_ok()
    }

    pub fn SelectAsTarget(&mut self, path: &str) {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        if let Err(pos) = search_selection(&self.sel[1], hash_code, path) {
            self.sel[1].insert(
                pos,
                SelEntry {
                    hash_code,
                    path: path.to_string(),
                },
            );
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    pub fn DeselectAsTarget(&mut self, path: &str) {
        let hash_code = emCalcHashCode(path.as_bytes(), 0);
        if let Ok(pos) = search_selection(&self.sel[1], hash_code, path) {
            self.sel[1].remove(pos);
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    pub fn ClearTargetSelection(&mut self) {
        if !self.sel[1].is_empty() {
            self.sel[1].clear();
            self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
        }
    }

    // --- Cross-selection operations ---

    pub fn SwapSelection(&mut self) {
        self.sel.swap(0, 1);
        self.sel_cmd_counter = self.sel_cmd_counter.wrapping_add(1);
    }

    /// Returns true if any selected path (source or target) is within the
    /// given directory tree. A path is "in" the dir tree if it starts with
    /// `dir_path` followed by `'/'`, or equals `dir_path` exactly.
    pub fn IsAnySelectionInDirTree(&self, dir_path: &str) -> bool {
        for arr in &self.sel {
            for entry in arr {
                if entry.path == dir_path
                    || (entry.path.starts_with(dir_path)
                        && entry.path.as_bytes().get(dir_path.len()) == Some(&b'/'))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Remove entries whose paths no longer exist on the filesystem.
    pub fn UpdateSelection(&mut self) {
        for arr in &mut self.sel {
            arr.retain(|entry| Path::new(&entry.path).exists());
        }
    }

    pub fn GetCommandRunId(&self) -> String {
        format!("{}", self.sel_cmd_counter)
    }

    pub fn handle_ipc_message(&mut self, args: &[&str]) {
        if args.len() == 1 && args[0] == "update" {
            return;
        }
        if args.len() >= 2 {
            let matches_id = self.GetCommandRunId() == args[1];
            match args[0] {
                "select" if matches_id => {
                    self.SwapSelection();
                    self.ClearTargetSelection();
                    for path in &args[2..] {
                        self.DeselectAsSource(path);
                        self.SelectAsTarget(path);
                    }
                }
                "selectks" if matches_id => {
                    self.ClearTargetSelection();
                    for path in &args[2..] {
                        self.DeselectAsSource(path);
                        self.SelectAsTarget(path);
                    }
                }
                "selectcs" if matches_id => {
                    self.ClearSourceSelection();
                    self.ClearTargetSelection();
                    for path in &args[2..] {
                        self.SelectAsTarget(path);
                    }
                }
                _ => {
                    log::warn!("emFileManModel: Illegal MiniIpc request: {:?}", args);
                }
            }
        }
    }
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_selections() {
        let m = SelectionManager::new();
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert_eq!(m.GetTargetSelectionCount(), 0);
        assert!(!m.IsSelectedAsSource("/foo"));
        assert!(!m.IsSelectedAsTarget("/foo"));
    }

    #[test]
    fn select_and_deselect_source() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/foo");
        assert!(m.IsSelectedAsSource("/foo"));
        assert_eq!(m.GetSourceSelectionCount(), 1);

        m.DeselectAsSource("/foo");
        assert!(!m.IsSelectedAsSource("/foo"));
        assert_eq!(m.GetSourceSelectionCount(), 0);
    }

    #[test]
    fn select_and_deselect_target() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/bar");
        assert!(m.IsSelectedAsTarget("/bar"));
        assert_eq!(m.GetTargetSelectionCount(), 1);

        m.DeselectAsTarget("/bar");
        assert!(!m.IsSelectedAsTarget("/bar"));
    }

    #[test]
    fn duplicate_select_is_idempotent() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/foo");
        m.SelectAsSource("/foo");
        assert_eq!(m.GetSourceSelectionCount(), 1);
    }

    #[test]
    fn swap_selection() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/tgt1");
        m.SwapSelection();
        assert!(m.IsSelectedAsTarget("/src1"));
        assert!(m.IsSelectedAsSource("/tgt1"));
    }

    #[test]
    fn clear_selections() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/s1");
        m.SelectAsSource("/s2");
        m.SelectAsTarget("/t1");
        m.ClearSourceSelection();
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert_eq!(m.GetTargetSelectionCount(), 1);
        m.ClearTargetSelection();
        assert_eq!(m.GetTargetSelectionCount(), 0);
    }

    #[test]
    fn hash_binary_search_ordering() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/z/last");
        m.SelectAsTarget("/a/first");
        m.SelectAsTarget("/m/middle");
        assert_eq!(m.GetTargetSelectionCount(), 3);
        assert!(m.IsSelectedAsTarget("/a/first"));
        assert!(m.IsSelectedAsTarget("/m/middle"));
        assert!(m.IsSelectedAsTarget("/z/last"));
    }

    #[test]
    fn get_selection_by_index() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/b");
        m.SelectAsSource("/a");
        assert_eq!(m.GetSourceSelectionCount(), 2);
        let s0 = m.GetSourceSelection(0);
        let s1 = m.GetSourceSelection(1);
        assert!(s0 == "/a" || s0 == "/b");
        assert!(s1 == "/a" || s1 == "/b");
        assert_ne!(s0, s1);
    }

    #[test]
    fn is_any_selection_in_dir_tree() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/home/user/docs/file.txt");
        assert!(m.IsAnySelectionInDirTree("/home/user/docs"));
        assert!(m.IsAnySelectionInDirTree("/home/user"));
        assert!(m.IsAnySelectionInDirTree("/home"));
        assert!(!m.IsAnySelectionInDirTree("/tmp"));
    }

    #[test]
    fn update_selection_removes_nonexistent() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/dev/null"); // exists
        m.SelectAsTarget("/nonexistent_emfileman_test"); // doesn't exist
        assert_eq!(m.GetTargetSelectionCount(), 2);
        m.UpdateSelection();
        assert_eq!(m.GetTargetSelectionCount(), 1);
        assert!(m.IsSelectedAsTarget("/dev/null"));
    }

    #[test]
    fn command_run_id_changes() {
        let mut m = SelectionManager::new();
        let id1 = m.GetCommandRunId();
        m.SelectAsSource("/foo");
        let id2 = m.GetCommandRunId();
        assert_ne!(id1, id2);
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn parse_command_properties() {
        let content = "#!/bin/bash\n\
            # [[BEGIN PROPERTIES]]\n\
            # Type = Command\n\
            # Order = 1.5\n\
            # Interpreter = bash\n\
            # Caption = Test Command\n\
            # Description = A test command\n\
            # DefaultFor = .txt:.rs\n\
            # [[END PROPERTIES]]\n\
            echo \"hello\"\n";
        let cmd = super::parse_command_properties(content, "/test/cmd.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Command);
        assert!((cmd.order - 1.5).abs() < f64::EPSILON);
        assert_eq!(cmd.interpreter, "bash");
        assert_eq!(cmd.caption, "Test Command");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.default_for, ".txt:.rs");
    }

    #[test]
    fn parse_group_properties() {
        let content = "#!/bin/bash\n\
            # [[BEGIN PROPERTIES]]\n\
            # Type = Group\n\
            # Order = 2.0\n\
            # Directory = subdir\n\
            # Caption = My Group\n\
            # [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test/group.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Group);
        assert_eq!(cmd.caption, "My Group");
    }

    #[test]
    fn parse_separator() {
        let content = "# [[BEGIN PROPERTIES]]\n# Type = Separator\n# [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test/sep.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Separator);
    }

    #[test]
    fn check_default_command_for_extension() {
        let cmd = CommandNode {
            default_for: ".txt:.rs".to_string(),
            command_type: CommandType::Command,
            ..CommandNode::default()
        };
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.txt"), 5); // ".txt".len() + 1
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.rs"), 4); // ".rs".len() + 1
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.py"), 0);
    }

    #[test]
    fn check_command_file_ending() {
        assert!(super::check_command_file_ending("test.sh"));
        assert!(super::check_command_file_ending("test.py"));
        assert!(super::check_command_file_ending("test.pl"));
        assert!(super::check_command_file_ending("test.js"));
        assert!(super::check_command_file_ending("test.props"));
        assert!(!super::check_command_file_ending("test.exe"));
        assert!(!super::check_command_file_ending("test.txt"));
    }

    #[test]
    fn search_default_command_for_priority() {
        let child1 = CommandNode {
            default_for: ".txt".to_string(),
            command_type: CommandType::Command,
            caption: "Simple".to_string(),
            ..CommandNode::default()
        };
        let child2 = CommandNode {
            default_for: ".tar.gz".to_string(),
            command_type: CommandType::Command,
            caption: "Archive".to_string(),
            ..CommandNode::default()
        };
        let root = CommandNode {
            command_type: CommandType::Group,
            children: vec![child1, child2],
            ..CommandNode::default()
        };
        // .tar.gz is longer match for "foo.tar.gz"
        let result = SearchDefaultCommandFor(&root, "/foo.tar.gz");
        assert!(result.is_some());
        assert_eq!(result.unwrap().caption, "Archive");
    }

    #[test]
    fn multi_line_caption() {
        let content = "# [[BEGIN PROPERTIES]]\n\
            # Type = Command\n\
            # Caption = Line 1\n\
            # Caption = Line 2\n\
            # [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test/cmd.sh").unwrap();
        assert_eq!(cmd.caption, "Line 1\nLine 2");
    }

    #[test]
    fn command_node_has_icon_and_look_fields() {
        let node = CommandNode::default();
        assert!(node.icon.is_none());
        assert_eq!(node.look, emcore::emLook::emLook::default());
    }

    #[test]
    fn parse_command_properties_with_colors() {
        let content = "#!/bin/bash\n\
            # [[BEGIN PROPERTIES]]\n\
            # Type = Command\n\
            # Caption = Test\n\
            # BgColor = #FF0000FF\n\
            # FgColor = #00FF00FF\n\
            # ButtonBgColor = #0000FFFF\n\
            # ButtonFgColor = #FFFFFFFF\n\
            # [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test.sh").unwrap();
        assert_eq!(cmd.look.bg_color, emcore::emColor::emColor::rgba(0xFF, 0x00, 0x00, 0xFF));
        assert_eq!(cmd.look.fg_color, emcore::emColor::emColor::rgba(0x00, 0xFF, 0x00, 0xFF));
        assert_eq!(
            cmd.look.button_bg_color,
            emcore::emColor::emColor::rgba(0x00, 0x00, 0xFF, 0xFF)
        );
        assert_eq!(
            cmd.look.button_fg_color,
            emcore::emColor::emColor::rgba(0xFF, 0xFF, 0xFF, 0xFF)
        );
    }

    #[test]
    fn parse_command_properties_with_6_digit_color() {
        let content = "# [[BEGIN PROPERTIES]]\n\
            # Type = Command\n\
            # BgColor = #FF0000\n\
            # [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test.sh").unwrap();
        assert_eq!(cmd.look.bg_color, emcore::emColor::emColor::rgba(0xFF, 0x00, 0x00, 0xFF));
    }

    #[test]
    fn parse_command_properties_icon_nonexistent() {
        let content = "# [[BEGIN PROPERTIES]]\n\
            # Type = Command\n\
            # Icon = nonexistent.tga\n\
            # [[END PROPERTIES]]\n";
        let cmd = super::parse_command_properties(content, "/test/cmd.sh").unwrap();
        // Icon file doesn't exist, so icon should be None
        assert!(cmd.icon.is_none());
    }
}

#[cfg(test)]
mod ipc_tests {
    use super::*;

    #[test]
    fn ipc_select_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["select", &run_id, "/new_target"]);

        // "select" swaps src→tgt, clears tgt, then deselects from src and selects as tgt
        // After swap: old source "/src1" becomes target, then clear target removes it
        // Then: deselect "/new_target" from source (noop), select as target
        assert!(m.IsSelectedAsTarget("/new_target"));
    }

    #[test]
    fn ipc_selectks_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/old_tgt");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["selectks", &run_id, "/new_target"]);

        // "selectks" keeps source, clears tgt, deselects from src, selects as tgt
        assert!(m.IsSelectedAsTarget("/new_target"));
        assert!(!m.IsSelectedAsTarget("/old_tgt"));
        assert!(m.IsSelectedAsSource("/src1")); // source kept (not deselected since different path)
    }

    #[test]
    fn ipc_selectcs_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/tgt1");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["selectcs", &run_id, "/new"]);

        // "selectcs" clears both, selects paths as target
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert!(m.IsSelectedAsTarget("/new"));
    }

    #[test]
    fn ipc_stale_run_id_ignored() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/existing");

        m.handle_ipc_message(&["select", "wrong_id", "/new"]);

        // Stale ID: selection unchanged
        assert!(m.IsSelectedAsTarget("/existing"));
        assert!(!m.IsSelectedAsTarget("/new"));
    }

    #[test]
    fn ipc_update_message() {
        let mut m = SelectionManager::new();
        // "update" is a no-op on SelectionManager (caller handles the signal)
        m.handle_ipc_message(&["update"]);
        // Just verify it doesn't crash
    }
}
