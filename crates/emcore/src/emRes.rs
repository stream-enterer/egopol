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
