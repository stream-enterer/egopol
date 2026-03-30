//! emTmpFile — temporary file path holder with RAII deletion.
//!
//! C++ emTmpFile.h provides emTmpFile (RAII path holder) and
//! emTmpFileMaster (IPC-based singleton for crash-resilient cleanup).
//!
//! DIVERGED: emTmpFileMaster deferred. The IPC-based singleton requires
//! deep integration with emModel/emContext lifecycle. emTmpFile works
//! standalone with explicit paths. emTmpFileMaster will be ported when
//! emTmpConv (the only outside consumer) is ported, if RAII cleanup
//! proves insufficient.

use std::path::{Path, PathBuf};

/// Temporary file/directory path holder. Deletes the file or directory
/// tree on drop. Matches C++ `emTmpFile`.
pub struct emTmpFile {
    path: PathBuf,
}

impl emTmpFile {
    /// Construct with empty path (no file to delete). C++ `emTmpFile()`.
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }

    /// Construct with an explicit path. C++ `emTmpFile(const emString&)`.
    pub fn from_custom_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Set a custom path. Calls Discard() first. C++ `SetupCustomPath`.
    pub fn SetupCustomPath(&mut self, path: PathBuf) {
        self.Discard();
        self.path = path;
    }

    /// Get the current path. C++ `GetPath`.
    pub fn GetPath(&self) -> &Path {
        &self.path
    }

    /// Delete the file/directory and clear the path. C++ `Discard`.
    pub fn Discard(&mut self) {
        if !self.path.as_os_str().is_empty() {
            if self.path.is_dir() {
                let _ = std::fs::remove_dir_all(&self.path);
            } else if self.path.exists() {
                let _ = std::fs::remove_file(&self.path);
            }
            self.path = PathBuf::new();
        }
    }
}

impl Default for emTmpFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for emTmpFile {
    fn drop(&mut self) {
        self.Discard();
    }
}
