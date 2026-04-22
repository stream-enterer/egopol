//! emRecFileReader — file-backed `emRecReader`.
//!
//! C++ reference: `include/emCore/emRec.h:1766-1813` (class declaration) and
//! `src/emCore/emRec.cpp:2690-2800` (implementation).
//!
//! C++ shape: `emRecFileReader : emRecReader` keeps a `FILE*` and implements
//! the protected virtuals `TryRead(char*, int)` / `TryClose` / `GetSourceName`,
//! letting the base class drive the lex.
//!
//! Rust shape (Phase 4d Task 4): the trait is per-element primitives only;
//! concrete types own their own state. `emRecFileReader` therefore COMPOSES
//! an [`emRecMemReader`] (it does not subclass it) — we slurp the file into a
//! `Vec<u8>`, hand it off via [`emRecMemReader::from_vec`], and delegate every
//! trait method. Source name is overridden to carry the file path so
//! [`RecIoError`] locations show the file, not `"rec memory buffer"`.

use std::path::Path;

use crate::emRecMemReader::emRecMemReader;
use crate::emRecReader::{emRecReader, PeekResult, RecIoError};

/// File-backed `emRecReader`. See module docs for the C++ correspondence.
pub struct emRecFileReader {
    mem: emRecMemReader,
    source_name: String,
}

impl emRecFileReader {
    /// Read the file at `path` into memory and return a ready-to-use reader.
    ///
    /// Mirrors C++ `emRecFileReader::TryStartReading` (emRec.cpp:2690-2730):
    /// the C++ version opens the file incrementally; the Rust port slurps
    /// eagerly because the lexer needs a full byte buffer. Rec files are
    /// configuration-scale (<10KB typical); this is not a hot path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, RecIoError> {
        let path = path.as_ref();
        let path_string = path.display().to_string();
        let bytes = std::fs::read(path).map_err(|e| {
            RecIoError::with_location(
                Some(path_string.clone()),
                None,
                format!("failed to read file: {}", e),
            )
        })?;
        Ok(Self {
            mem: emRecMemReader::from_vec(bytes),
            source_name: path_string,
        })
    }

    /// Path-backed source name. Mirrors `emRecFileReader::GetSourceName`
    /// (emRec.cpp:2795-2798), which returns `FilePath.Get()`.
    pub fn GetSourceName(&self) -> &str {
        &self.source_name
    }

    /// Open + validate the `#%rec:<expected_format>%` magic header.
    ///
    /// Mirrors the header-consumption branch of C++
    /// `emRecReader::TryStartReading` (emRec.cpp:2004-2042). See
    /// [`crate::emRecMemReader::with_format_header`] for the exact shape of
    /// the magic (the trailing `#` that conventionally appears after `%` is
    /// part of the lexer's comment handling, not the magic proper).
    pub fn open_with_format(
        path: impl AsRef<Path>,
        expected_format: &str,
    ) -> Result<Self, RecIoError> {
        let path = path.as_ref();
        let path_string = path.display().to_string();
        let bytes = std::fs::read(path).map_err(|e| {
            RecIoError::with_location(
                Some(path_string.clone()),
                None,
                format!("failed to read file: {}", e),
            )
        })?;
        let mem = emRecMemReader::with_format_header_vec(
            bytes,
            expected_format,
            Some(path_string.clone()),
        )?;
        Ok(Self {
            mem,
            source_name: path_string,
        })
    }
}

impl emRecReader for emRecFileReader {
    fn TryPeekNext(&mut self) -> Result<PeekResult, RecIoError> {
        self.mem.TryPeekNext().map_err(|e| self.retag(e))
    }
    fn TryReadDelimiter(&mut self) -> Result<char, RecIoError> {
        self.mem.TryReadDelimiter().map_err(|e| self.retag(e))
    }
    fn TryReadCertainDelimiter(&mut self, delimiter: char) -> Result<(), RecIoError> {
        self.mem
            .TryReadCertainDelimiter(delimiter)
            .map_err(|e| self.retag(e))
    }
    fn TryReadIdentifier(&mut self) -> Result<String, RecIoError> {
        self.mem.TryReadIdentifier().map_err(|e| self.retag(e))
    }
    fn TryReadInt(&mut self) -> Result<i32, RecIoError> {
        self.mem.TryReadInt().map_err(|e| self.retag(e))
    }
    fn TryReadDouble(&mut self) -> Result<f64, RecIoError> {
        self.mem.TryReadDouble().map_err(|e| self.retag(e))
    }
    fn TryReadQuoted(&mut self) -> Result<String, RecIoError> {
        self.mem.TryReadQuoted().map_err(|e| self.retag(e))
    }
    fn ThrowElemError(&self, text: &str) -> RecIoError {
        self.retag(self.mem.ThrowElemError(text))
    }
    fn ThrowSyntaxError(&self) -> RecIoError {
        self.retag(self.mem.ThrowSyntaxError())
    }
}

impl emRecFileReader {
    /// Replace the inner reader's `"rec memory buffer"` source name with the
    /// file path, keeping the line number intact — mirrors the effect of C++
    /// `emRecFileReader::GetSourceName` being used wherever the base class
    /// formats errors (emRec.cpp:2795-2798).
    fn retag(&self, e: RecIoError) -> RecIoError {
        RecIoError::with_location(Some(self.source_name.clone()), e.line, e.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_bytes_from_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"hello 42").unwrap();
        let path = f.path().to_path_buf();

        let mut r = emRecFileReader::new(&path).unwrap();
        assert_eq!(r.TryReadIdentifier().unwrap(), "hello");
        assert_eq!(r.TryReadInt().unwrap(), 42);
    }

    #[test]
    fn missing_file_reports_path() {
        let err = match emRecFileReader::new("/nonexistent/path/for/emrec/test") {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("/nonexistent/path/for/emrec/test"), "{msg}");
        assert!(msg.contains("failed to read file"), "{msg}");
    }

    #[test]
    fn error_carries_file_source_name() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        // Invalid: `maybe` is an identifier, not an int.
        f.write_all(b"maybe").unwrap();
        let path = f.path().to_path_buf();

        let mut r = emRecFileReader::new(&path).unwrap();
        let err = r.TryReadInt().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(&path.display().to_string()), "{msg}");
    }

    // Silence unused-field lints if retain path changes; GetSourceName is
    // part of the public API and is exercised by callers. This is a
    // sanity touch-test.
    #[test]
    fn source_name_getter_returns_path() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"").unwrap();
        let path_string = f.path().display().to_string();
        let r = emRecFileReader::new(f.path()).unwrap();
        assert_eq!(r.GetSourceName(), path_string);
    }

    // PathBuf round-trip smoke test — just verifies `AsRef<Path>` accepts
    // `PathBuf` directly.
    #[test]
    fn accepts_pathbuf() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let p: std::path::PathBuf = f.path().to_path_buf();
        assert!(emRecFileReader::new(p).is_ok());
    }
}
