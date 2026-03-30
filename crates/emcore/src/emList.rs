// emList.rs — COW doubly-linked list, ported from emList.h
//
// DIVERGED: C++ emList uses intrusive doubly-linked list with O(1) splice.
// Rust uses Vec<T> backing for cache locality. Navigation uses index (not
// pointer). Move operations between lists are O(n) copies, not O(1) splices.
// C++ pointer-based API (GetNext(const OBJ*)) becomes index-based
// (GetNext(usize) -> Option<(usize, &T)>).
//
// DIVERGED: C++ overloaded methods are split into distinct names
// (InsertAtBeg_one, InsertAtEnd_one, Add_one, etc.) because Rust has no
// overloading.
//
// DIVERGED: Iterator inner class renamed to ListCursor with index-based
// tracking (same pattern as emArray::Cursor), since Rust Iterator is a
// standard trait with different semantics.
//
// DIVERGED: Move operations (MoveToBeg, MoveToEnd, MoveBefore, MoveAfter)
// are O(n) Vec operations, not O(1) pointer relinks.
//
// DIVERGED: GetSubList, Extract(range) — omitted from initial port. No
// emCore consumer exists.
//
// DIVERGED: Sort takes no comparator function pointer. Uses Ord trait.
// Separate impl block bounded on T: Ord.
//
// DIVERGED: GetIndexOf searches by value equality (PartialEq) rather than
// by pointer identity, since elements are stored in a Vec, not as
// individually-allocated nodes.

use std::rc::Rc;

/// Stable cursor for emList. Tracks position by index.
///
/// DIVERGED: C++ emList::Iterator auto-adjusts when elements are
/// inserted/removed and tracks pointer identity through COW copies.
/// This Rust cursor uses a plain index like emArray::Cursor.
pub struct ListCursor {
    index: Option<usize>,
}

impl ListCursor {
    pub fn IsValid<T: Clone>(&self, list: &emList<T>) -> bool {
        match self.index {
            Some(i) => i < list.GetCount(),
            None => false,
        }
    }

    pub fn Get<'a, T: Clone>(&self, list: &'a emList<T>) -> Option<&'a T> {
        self.index.and_then(|i| list.data.get(i))
    }

    pub fn SetNext<T: Clone>(&mut self, list: &emList<T>) {
        match self.index {
            Some(i) if i + 1 < list.GetCount() => self.index = Some(i + 1),
            Some(_) => self.index = None,
            None => {}
        }
    }

    pub fn SetPrev<T: Clone>(&mut self, _list: &emList<T>) {
        match self.index {
            Some(0) => self.index = None,
            Some(i) => self.index = Some(i - 1),
            None => {}
        }
    }

    pub fn SetIndex(&mut self, index: usize) {
        self.index = Some(index);
    }

    pub fn Detach(&mut self) {
        self.index = None;
    }
}

/// COW doubly-linked list backed by `Rc<Vec<T>>`.
///
/// Clone is O(1) shallow (Rc::clone). Mutation triggers deep copy
/// if the Rc is shared (Rc::make_mut).
pub struct emList<T: Clone> {
    data: Rc<Vec<T>>,
}

impl<T: Clone> Clone for emList<T> {
    fn clone(&self) -> Self {
        emList {
            data: Rc::clone(&self.data),
        }
    }
}

impl<T: Clone> Default for emList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> emList<T> {
    /// Construct an empty list.
    pub fn new() -> Self {
        emList {
            data: Rc::new(Vec::new()),
        }
    }

    /// Construct a list with one element.
    pub fn from_element(obj: T) -> Self {
        emList {
            data: Rc::new(vec![obj]),
        }
    }

    // --- Navigation ---

    /// Get a reference to the first element, or None if empty.
    pub fn GetFirst(&self) -> Option<&T> {
        self.data.first()
    }

    /// Get a reference to the last element, or None if empty.
    pub fn GetLast(&self) -> Option<&T> {
        self.data.last()
    }

    /// Get the next element after the given index. Returns the new index and
    /// a reference, or None if at the end.
    pub fn GetNext(&self, index: usize) -> Option<(usize, &T)> {
        let next = index + 1;
        self.data.get(next).map(|v| (next, v))
    }

    /// Get the previous element before the given index. Returns the new index
    /// and a reference, or None if at the beginning.
    pub fn GetPrev(&self, index: usize) -> Option<(usize, &T)> {
        if index == 0 {
            return None;
        }
        let prev = index - 1;
        self.data.get(prev).map(|v| (prev, v))
    }

    /// Get element at the given index, or None if out of range.
    pub fn GetAtIndex(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Search for an element by value and return its index.
    ///
    /// DIVERGED: C++ searches by pointer identity within the linked list.
    /// Rust searches by value equality since elements are in a Vec.
    pub fn GetIndexOf(&self, elem: &T) -> Option<usize>
    where
        T: PartialEq,
    {
        self.data.iter().position(|e| e == elem)
    }

    // --- Writable access ---

    /// Get a mutable reference to the element at the given index.
    /// Triggers COW deep copy if shared.
    pub fn GetWritable(&mut self, index: usize) -> &mut T {
        &mut Rc::make_mut(&mut self.data)[index]
    }

    /// Get a mutable reference to the first element.
    pub fn GetFirstWritable(&mut self) -> Option<&mut T> {
        if self.data.is_empty() {
            return None;
        }
        Some(&mut Rc::make_mut(&mut self.data)[0])
    }

    /// Get a mutable reference to the last element.
    pub fn GetLastWritable(&mut self) -> Option<&mut T> {
        let len = self.data.len();
        if len == 0 {
            return None;
        }
        let v = Rc::make_mut(&mut self.data);
        Some(&mut v[len - 1])
    }

    /// Replace an element at the given index.
    pub fn Set(&mut self, index: usize, obj: T) {
        Rc::make_mut(&mut self.data)[index] = obj;
    }

    // --- Insert ---

    /// Insert one element at the beginning.
    pub fn InsertAtBeg_one(&mut self, obj: T) {
        Rc::make_mut(&mut self.data).insert(0, obj);
    }

    /// Insert one element at the end.
    pub fn InsertAtEnd_one(&mut self, obj: T) {
        Rc::make_mut(&mut self.data).push(obj);
    }

    /// Insert one element before the given index.
    pub fn InsertBefore(&mut self, index: usize, obj: T) {
        Rc::make_mut(&mut self.data).insert(index, obj);
    }

    /// Insert one element after the given index.
    pub fn InsertAfter(&mut self, index: usize, obj: T) {
        Rc::make_mut(&mut self.data).insert(index + 1, obj);
    }

    /// Alias for InsertAtEnd_one.
    pub fn Add_one(&mut self, obj: T) {
        self.InsertAtEnd_one(obj);
    }

    // --- Remove ---

    /// Remove the first element.
    pub fn RemoveFirst(&mut self) {
        if !self.data.is_empty() {
            Rc::make_mut(&mut self.data).remove(0);
        }
    }

    /// Remove the last element.
    pub fn RemoveLast(&mut self) {
        if !self.data.is_empty() {
            Rc::make_mut(&mut self.data).pop();
        }
    }

    /// Remove the element at the given index.
    pub fn Remove(&mut self, index: usize) {
        Rc::make_mut(&mut self.data).remove(index);
    }

    /// Remove all elements.
    pub fn Clear(&mut self) {
        if !self.data.is_empty() {
            Rc::make_mut(&mut self.data).clear();
        }
    }

    // --- Extract ---

    /// Remove and return the first element.
    pub fn ExtractFirst(&mut self) -> Option<T> {
        if self.data.is_empty() {
            None
        } else {
            Some(Rc::make_mut(&mut self.data).remove(0))
        }
    }

    /// Remove and return the last element.
    pub fn ExtractLast(&mut self) -> Option<T> {
        if self.data.is_empty() {
            None
        } else {
            Rc::make_mut(&mut self.data).pop()
        }
    }

    // --- Query ---

    /// Whether the list has no elements.
    pub fn IsEmpty(&self) -> bool {
        self.data.is_empty()
    }

    /// Compute the number of elements.
    pub fn GetCount(&self) -> usize {
        self.data.len()
    }

    /// Get number of references to the data behind this list.
    pub fn GetDataRefCount(&self) -> usize {
        Rc::strong_count(&self.data)
    }

    /// Ensure this list has its own unique copy of the data.
    pub fn MakeNonShared(&mut self) {
        Rc::make_mut(&mut self.data);
    }

    // --- Cursor factories ---

    /// Create a cursor pointing to the first element.
    pub fn cursor_first(&self) -> ListCursor {
        ListCursor {
            index: if self.data.is_empty() { None } else { Some(0) },
        }
    }

    /// Create a cursor pointing to the last element.
    pub fn cursor_last(&self) -> ListCursor {
        ListCursor {
            index: if self.data.is_empty() {
                None
            } else {
                Some(self.data.len() - 1)
            },
        }
    }

    /// Create a cursor pointing to the given index.
    pub fn cursor_at(&self, index: usize) -> ListCursor {
        ListCursor {
            index: if index < self.data.len() {
                Some(index)
            } else {
                None
            },
        }
    }
}

impl<T: Clone + Ord> emList<T> {
    /// Sort this list. The order of equal elements is preserved (stable sort).
    /// Returns true if the order changed.
    pub fn Sort(&mut self) -> bool {
        let len = self.data.len();
        if len <= 1 {
            return false;
        }
        // Check if already sorted to avoid unnecessary COW clone.
        if self.data.windows(2).all(|w| w[0] <= w[1]) {
            return false;
        }
        Rc::make_mut(&mut self.data).sort();
        true
    }
}
