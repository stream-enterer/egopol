//! emRecFileWriter — file-backed `emRecWriter`.
//!
//! C++ reference: `include/emCore/emRec.h:1820-1845` (class declaration) and
//! `src/emCore/emRec.cpp:2802-2860` (implementation).
//!
//! C++ shape: `emRecFileWriter : emRecWriter` keeps a `FILE*` and implements
//! the protected virtuals `TryWrite(const char*, int)` / `TryClose`, letting
//! the base class drive the format.
//!
//! Rust shape (Phase 4d Task 4): COMPOSES an [`emRecMemWriter`] that buffers
//! the formatted output, and flushes to disk via [`Self::finalize`].

use std::path::{Path, PathBuf};

use crate::emRecMemWriter::emRecMemWriter;
use crate::emRecReader::RecIoError;
use crate::emRecWriter::emRecWriter;

/// File-backed `emRecWriter`. Every trait method delegates to the inner
/// [`emRecMemWriter`]; call [`Self::finalize`] to flush the buffer to disk.
pub struct emRecFileWriter {
    mem: emRecMemWriter,
    path: PathBuf,
}

impl emRecFileWriter {
    /// Mirrors C++ `emRecFileWriter::TryStartWriting` (emRec.cpp:2802-2820)
    /// except the Rust port defers the actual `fopen(…, "w")` until
    /// [`Self::finalize`]; no truncation happens at construction time.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            mem: emRecMemWriter::new(),
            path: path.into(),
        }
    }

    /// Write the buffered bytes to the destination path and consume self.
    ///
    /// Mirrors the C++ `emRecFileWriter::TryClose` finalisation step
    /// (emRec.cpp:2847-2859) plus the base class' implicit flush.
    pub fn finalize(self) -> Result<(), RecIoError> {
        let bytes = self.mem.into_bytes();
        std::fs::write(&self.path, bytes).map_err(|e| {
            RecIoError::with_location(
                Some(self.path.display().to_string()),
                None,
                format!("failed to write file: {}", e),
            )
        })
    }

    /// Path being written to. Useful for diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl emRecWriter for emRecFileWriter {
    fn TryWriteDelimiter(&mut self, c: char) -> Result<(), RecIoError> {
        self.mem.TryWriteDelimiter(c)
    }
    fn TryWriteIdentifier(&mut self, idf: &str) -> Result<(), RecIoError> {
        self.mem.TryWriteIdentifier(idf)
    }
    fn TryWriteInt(&mut self, i: i32) -> Result<(), RecIoError> {
        self.mem.TryWriteInt(i)
    }
    fn TryWriteDouble(&mut self, d: f64) -> Result<(), RecIoError> {
        self.mem.TryWriteDouble(d)
    }
    fn TryWriteQuoted(&mut self, q: &str) -> Result<(), RecIoError> {
        self.mem.TryWriteQuoted(q)
    }
    fn TryWriteSpace(&mut self) -> Result<(), RecIoError> {
        self.mem.TryWriteSpace()
    }
    fn TryWriteNewLine(&mut self) -> Result<(), RecIoError> {
        self.mem.TryWriteNewLine()
    }
    fn TryWriteIndent(&mut self) -> Result<(), RecIoError> {
        self.mem.TryWriteIndent()
    }
    fn IncIndent(&mut self) {
        self.mem.IncIndent();
    }
    fn DecIndent(&mut self) {
        self.mem.DecIndent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_buffer_on_finalize() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let path = f.path().to_path_buf();
        drop(f); // don't hold the handle — finalize will open for write.

        let mut w = emRecFileWriter::new(&path);
        w.TryWriteIdentifier("yes").unwrap();
        w.finalize().unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes, b"yes");
    }

    #[test]
    fn finalize_reports_path_on_io_error() {
        let w = emRecFileWriter::new("/nonexistent/directory/out.rec");
        let err = w.finalize().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("/nonexistent/directory/out.rec"), "{msg}");
        assert!(msg.contains("failed to write file"), "{msg}");
    }

    #[test]
    fn path_getter_returns_destination() {
        let w = emRecFileWriter::new("/tmp/some-rec-path");
        assert_eq!(w.path(), Path::new("/tmp/some-rec-path"));
    }
}
