use std::path::{Path, PathBuf};

use crate::emCore::emRec::{parse_rec, write_rec, RecError};
use crate::emCore::emSignal::SignalId;

use crate::emCore::emRecRecord::Record;

/// A configuration record backed by a file path with emRec serialization.
///
/// Tracks a dirty flag for unsaved changes. `load()` reads from disk,
/// `save()` writes to disk. `load_or_install()` handles first-run by
/// creating a default config file if none exists.
pub struct emConfigModel<T: Record> {
    value: T,
    path: PathBuf,
    change_signal: SignalId,
    dirty: bool,
}

impl<T: Record> emConfigModel<T> {
    pub fn new(value: T, path: PathBuf, signal_id: SignalId) -> Self {
        Self {
            value,
            path,
            change_signal: signal_id,
            dirty: false,
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    /// Replace the value. Returns `true` if dirty flag was set (always, since
    /// Record types don't require PartialEq).
    pub fn set(&mut self, new_value: T) -> bool {
        self.value = new_value;
        self.dirty = true;
        true
    }

    /// Modify the value in place. Returns `true` (marks dirty).
    pub fn modify<F: FnOnce(&mut T)>(&mut self, f: F) -> bool {
        f(&mut self.value);
        self.dirty = true;
        true
    }

    pub fn change_signal(&self) -> SignalId {
        self.change_signal
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Reset the value to its default. Returns `true` if dirty flag was set.
    pub fn reset_to_default(&mut self) -> bool {
        self.value.set_to_default();
        self.dirty = true;
        true
    }

    /// Load the configuration from disk. Parses emRec and deserializes.
    pub fn load(&mut self) -> Result<(), RecError> {
        let contents = std::fs::read_to_string(&self.path).map_err(RecError::Io)?;
        let rec = parse_rec(&contents)?;
        self.value = T::from_rec(&rec)?;
        self.dirty = false;
        Ok(())
    }

    /// Save the configuration to disk as emRec.
    pub fn save(&mut self) -> Result<(), RecError> {
        let rec = self.value.to_rec();
        let contents = write_rec(&rec);

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(RecError::Io)?;
        }

        std::fs::write(&self.path, contents).map_err(RecError::Io)?;
        self.dirty = false;
        Ok(())
    }

    /// Load from disk, or create a default config file if none exists.
    pub fn load_or_install(&mut self) -> Result<(), RecError> {
        if self.path.exists() {
            self.load()
        } else {
            self.value.set_to_default();
            self.dirty = true;
            self.save()
        }
    }
}
