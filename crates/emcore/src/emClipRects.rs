use std::fmt;

/// A disjoint set of non-overlapping rectangles supporting set operations
/// (unite, intersect, subtract). Used for dirty-region tracking.
///
/// Rust port of C++ `emClipRects<f64>`.
///
/// Coordinates use x1/y1 (top-left, inclusive) and x2/y2 (bottom-right,
/// exclusive) convention.
#[derive(Clone)]
pub struct ClipRects {
    rects: Vec<ClipRect>,
}

/// A single rectangle in a [`ClipRects`] set.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ClipRect {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl ClipRect {
    fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }
}

impl ClipRects {
    /// Create an empty set.
    pub fn new() -> Self {
        Self { rects: Vec::new() }
    }

    /// Whether the set is empty.
    pub fn IsEmpty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Number of rectangles.
    pub fn GetCount(&self) -> usize {
        self.rects.len()
    }

    /// Iterate over rectangles.
    pub fn iter(&self) -> impl Iterator<Item = &ClipRect> {
        self.rects.iter()
    }

    /// Add a rectangle to the set (OR / union operation).
    ///
    /// Matching C++ `emClipRects::Unite(x1, y1, x2, y2)`.
    pub fn unite_rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if x1 >= x2 || y1 >= y2 {
            return;
        }
        Self::priv_unite(&mut self.rects, x1, y1, x2, y2);
    }

    /// Subtract a single rectangle from the set.
    ///
    /// Matching C++ `emClipRects::Subtract(x1, y1, x2, y2)`.
    fn subtract_rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if self.rects.is_empty() || x1 >= x2 || y1 >= y2 {
            return;
        }

        let mut i = 0;
        while i < self.rects.len() {
            let rx1 = self.rects[i].x1;
            let ry1 = self.rects[i].y1;
            let rx2 = self.rects[i].x2;
            let ry2 = self.rects[i].y2;

            if rx1 >= x2 || rx2 <= x1 || ry1 >= y2 || ry2 <= y1 {
                i += 1;
                continue;
            }

            // There is overlap — remove the original rect
            self.rects.swap_remove(i);

            let sy1 = ry1.max(y1);
            let sy2 = ry2.min(y2);

            // Top strip
            if ry1 < y1 {
                self.rects.push(ClipRect::new(rx1, ry1, rx2, y1));
            }
            // Left strip
            if rx1 < x1 {
                self.rects.push(ClipRect::new(rx1, sy1, x1, sy2));
            }
            // Right strip
            if rx2 > x2 {
                self.rects.push(ClipRect::new(x2, sy1, rx2, sy2));
            }
            // Bottom strip
            if ry2 > y2 {
                self.rects.push(ClipRect::new(rx1, y2, rx2, ry2));
            }
            // Don't increment — swap_remove may have placed a new element at i
        }
    }

    /// Compute the bounding box of all rectangles.
    /// Returns `(0, 0, 0, 0)` if empty.
    pub fn GetMinMax(&self) -> (f64, f64, f64, f64) {
        if self.rects.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let mut x1 = self.rects[0].x1;
        let mut y1 = self.rects[0].y1;
        let mut x2 = self.rects[0].x2;
        let mut y2 = self.rects[0].y2;
        for r in &self.rects[1..] {
            if r.x1 < x1 {
                x1 = r.x1;
            }
            if r.y1 < y1 {
                y1 = r.y1;
            }
            if r.x2 > x2 {
                x2 = r.x2;
            }
            if r.y2 > y2 {
                y2 = r.y2;
            }
        }
        (x1, y1, x2, y2)
    }

    /// Private unite helper — direct translation of C++ `PrivUnite`.
    fn priv_unite(list: &mut Vec<ClipRect>, mut x1: f64, mut y1: f64, mut x2: f64, mut y2: f64) {
        let mut i = 0;
        while i < list.len() {
            let rx1 = list[i].x1;
            let ry1 = list[i].y1;
            let rx2 = list[i].x2;
            let ry2 = list[i].y2;

            if ry1 > y2 || ry2 < y1 || rx1 > x2 || rx2 < x1 {
                i += 1;
            } else if rx1 <= x1 && rx2 >= x2 && ry1 <= y1 && ry2 >= y2 {
                // Existing rect fully contains the new one
                return;
            } else if rx1 >= x1 && rx2 <= x2 && ry1 >= y1 && ry2 <= y2 {
                // New rect fully contains existing
                list.swap_remove(i);
            } else if rx1 == x1 && rx2 == x2 {
                // Same x-extent — merge vertically
                if y1 > ry1 {
                    y1 = ry1;
                }
                if y2 < ry2 {
                    y2 = ry2;
                }
                list.swap_remove(i);
            } else if ry1 < y2 && ry2 > y1 {
                // Partial overlap — split existing and widen new rect
                if ry2 > y2 {
                    list[i].y1 = y2;
                    if ry1 < y1 {
                        list.push(ClipRect::new(rx1, ry1, rx2, y1));
                    }
                } else if ry1 < y1 {
                    list[i].y2 = y1;
                } else {
                    list.swap_remove(i);
                }
                if y1 < ry1 {
                    Self::priv_unite(list, x1, y1, x2, ry1);
                    y1 = ry1;
                }
                if y2 > ry2 {
                    Self::priv_unite(list, x1, ry2, x2, y2);
                    y2 = ry2;
                }
                if x1 > rx1 {
                    x1 = rx1;
                }
                if x2 < rx2 {
                    x2 = rx2;
                }
                continue;
            } else {
                i += 1;
            }
        }
        list.push(ClipRect::new(x1, y1, x2, y2));
    }
}

impl Default for ClipRects {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ClipRects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipRects")
            .field("count", &self.rects.len())
            .field("rects", &self.rects)
            .finish()
    }
}

// --- Additional set operations ---

impl ClipRects {
    /// Intersect with a single rectangle. Rectangles outside are removed,
    /// those partially inside are clipped.
    pub fn intersect_rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if x1 >= x2 || y1 >= y2 {
            self.rects.clear();
            return;
        }
        self.rects.retain_mut(|r| {
            if r.x1 < x1 {
                r.x1 = x1;
            }
            if r.x2 > x2 {
                r.x2 = x2;
            }
            if r.x1 >= r.x2 {
                return false;
            }
            if r.y1 < y1 {
                r.y1 = y1;
            }
            if r.y2 > y2 {
                r.y2 = y2;
            }
            r.y1 < r.y2
        });
    }

    /// Create a set containing a single rectangle.
    pub fn from_rect(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut s = Self::new();
        if x1 < x2 && y1 < y2 {
            s.rects.push(ClipRect { x1, y1, x2, y2 });
        }
        s
    }

    /// Clear all rectangles.
    pub fn Clear(&mut self) {
        self.rects.clear();
    }

    /// Set to a single rectangle, replacing all existing contents.
    pub fn Set(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.rects.clear();
        if x1 < x2 && y1 < y2 {
            self.rects.push(ClipRect { x1, y1, x2, y2 });
        }
    }

    /// Replace contents with the bounding box of the current set.
    pub fn SetToMinMax(&mut self) {
        if self.rects.len() <= 1 {
            return;
        }
        let (x1, y1, x2, y2) = self.GetMinMax();
        self.Set(x1, y1, x2, y2);
    }

    /// Set this set to the bounding box of another set.
    ///
    /// Port of C++ `emClipRects::SetToMinMaxOf`.
    pub fn SetToMinMaxOf(&mut self, other: &ClipRects) {
        if other.rects.len() <= 1 {
            self.rects.clone_from(&other.rects);
        } else {
            let (x1, y1, x2, y2) = other.GetMinMax();
            self.Set(x1, y1, x2, y2);
        }
    }

    /// Check whether this set contains all of another set.
    ///
    /// Port of C++ `emClipRects::IsSupersetOf(const emClipRects&)`.
    pub fn IsSupersetOf(&self, other: &ClipRects) -> bool {
        other.IsSubsetOf(self)
    }

    /// Add all rectangles from another set (union).
    pub fn Unite(&mut self, other: &ClipRects) {
        for r in &other.rects {
            Self::priv_unite(&mut self.rects, r.x1, r.y1, r.x2, r.y2);
        }
    }

    /// Intersect with another set of clip rects.
    pub fn Intersect(&mut self, other: &ClipRects) {
        if self.rects.is_empty() {
            return;
        }
        if other.rects.is_empty() {
            self.Clear();
            return;
        }
        if other.rects.len() == 1 {
            let r = &other.rects[0];
            self.intersect_rect(r.x1, r.y1, r.x2, r.y2);
            return;
        }
        if self.rects.len() == 1 {
            let r = self.rects[0];
            let mut cr = other.clone();
            cr.intersect_rect(r.x1, r.y1, r.x2, r.y2);
            *self = cr;
            return;
        }
        let mut complement = Self::new();
        let (x1, y1, x2, y2) = self.GetMinMax();
        complement.Set(x1, y1, x2, y2);
        complement.Subtract(other);
        self.Subtract(&complement);
    }

    /// Subtract all rectangles of another set.
    pub fn Subtract(&mut self, other: &ClipRects) {
        for r in &other.rects {
            if self.rects.is_empty() {
                break;
            }
            self.subtract_rect(r.x1, r.y1, r.x2, r.y2);
        }
    }

    /// Sort rectangles by Y1 then X1.
    pub fn Sort(&mut self) {
        self.rects.sort_by(|a, b| {
            a.y1.partial_cmp(&b.y1)
                .unwrap()
                .then(a.x1.partial_cmp(&b.x1).unwrap())
        });
    }

    /// Check whether all rectangles fit inside the given bounds.
    pub fn is_subset_of_rect(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
        for r in &self.rects {
            if r.x1 < x1 || r.y1 < y1 || r.x2 > x2 || r.y2 > y2 {
                return false;
            }
        }
        true
    }

    /// Check whether this set is a subset of another set.
    pub fn IsSubsetOf(&self, other: &ClipRects) -> bool {
        if self.rects.is_empty() {
            return true;
        }
        if other.rects.is_empty() {
            return false;
        }
        if other.rects.len() == 1 {
            let r = &other.rects[0];
            return self.is_subset_of_rect(r.x1, r.y1, r.x2, r.y2);
        }
        let mut diff = self.clone();
        diff.Subtract(other);
        diff.IsEmpty()
    }

    /// Check whether this set contains the given rectangle.
    pub fn is_superset_of_rect(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
        ClipRects::from_rect(x1, y1, x2, y2).IsSubsetOf(self)
    }
}

impl PartialEq for ClipRects {
    fn eq(&self, other: &Self) -> bool {
        self.IsSubsetOf(other) && other.IsSubsetOf(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set() {
        let cr = ClipRects::new();
        assert!(cr.IsEmpty());
        assert_eq!(cr.GetCount(), 0);
        assert_eq!(cr.GetMinMax(), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn single_rect() {
        let cr = ClipRects::from_rect(10.0, 20.0, 30.0, 40.0);
        assert_eq!(cr.GetCount(), 1);
        assert_eq!(cr.GetMinMax(), (10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn unite_non_overlapping() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.unite_rect(20.0, 20.0, 30.0, 30.0);
        assert_eq!(cr.GetCount(), 2);
        assert_eq!(cr.GetMinMax(), (0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn unite_overlapping_merges() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.unite_rect(5.0, 0.0, 15.0, 10.0);
        let (x1, y1, x2, y2) = cr.GetMinMax();
        assert!(x1 <= 0.0);
        assert!(y1 <= 0.0);
        assert!(x2 >= 15.0);
        assert!(y2 >= 10.0);
    }

    #[test]
    fn unite_containing() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.unite_rect(2.0, 2.0, 8.0, 8.0);
        assert_eq!(cr.GetCount(), 1);
    }

    #[test]
    fn unite_contained_by_existing() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.unite_rect(-1.0, -1.0, 11.0, 11.0);
        assert_eq!(cr.GetCount(), 1);
        let r = cr.iter().next().unwrap();
        assert_eq!((r.x1, r.y1, r.x2, r.y2), (-1.0, -1.0, 11.0, 11.0));
    }

    #[test]
    fn unite_same_x_merges_vertically() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 5.0);
        cr.unite_rect(0.0, 5.0, 10.0, 10.0);
        assert_eq!(cr.GetCount(), 1);
        let r = cr.iter().next().unwrap();
        assert_eq!((r.x1, r.y1, r.x2, r.y2), (0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn intersect_rect_clips() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.intersect_rect(5.0, 5.0, 15.0, 15.0);
        assert_eq!(cr.GetCount(), 1);
        let r = cr.iter().next().unwrap();
        assert_eq!((r.x1, r.y1, r.x2, r.y2), (5.0, 5.0, 10.0, 10.0));
    }

    #[test]
    fn intersect_rect_no_overlap() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 5.0, 5.0);
        cr.intersect_rect(10.0, 10.0, 20.0, 20.0);
        assert!(cr.IsEmpty());
    }

    #[test]
    fn subtract_rect_splits() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.subtract_rect(3.0, 3.0, 7.0, 7.0);
        assert!(!cr.IsEmpty());
        assert!(!cr.is_superset_of_rect(3.0, 3.0, 7.0, 7.0));
        assert!(cr.is_superset_of_rect(0.0, 0.0, 3.0, 3.0));
    }

    #[test]
    fn subtract_rect_full_removal() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.subtract_rect(-1.0, -1.0, 11.0, 11.0);
        assert!(cr.IsEmpty());
    }

    #[test]
    fn subtract_no_overlap() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 5.0, 5.0);
        cr.subtract_rect(10.0, 10.0, 20.0, 20.0);
        assert_eq!(cr.GetCount(), 1);
    }

    #[test]
    fn intersect_two_sets() {
        let mut a = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        let b = ClipRects::from_rect(5.0, 5.0, 15.0, 15.0);
        a.Intersect(&b);
        assert!(a.is_subset_of_rect(5.0, 5.0, 10.0, 10.0));
    }

    #[test]
    fn set_to_min_max_collapses() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 5.0, 5.0);
        cr.unite_rect(10.0, 10.0, 15.0, 15.0);
        assert_eq!(cr.GetCount(), 2);
        cr.SetToMinMax();
        assert_eq!(cr.GetCount(), 1);
        let r = cr.iter().next().unwrap();
        assert_eq!((r.x1, r.y1, r.x2, r.y2), (0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn sort_orders_by_y_then_x() {
        let mut cr = ClipRects::new();
        cr.unite_rect(10.0, 10.0, 20.0, 20.0);
        cr.unite_rect(0.0, 0.0, 5.0, 5.0);
        cr.unite_rect(5.0, 0.0, 8.0, 5.0);
        cr.Sort();
        let rects: Vec<_> = cr.iter().collect();
        for i in 1..rects.len() {
            assert!(
                (rects[i].y1, rects[i].x1) >= (rects[i - 1].y1, rects[i - 1].x1),
                "sort order violated"
            );
        }
    }

    #[test]
    fn equality() {
        let a = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        let mut b = ClipRects::new();
        b.unite_rect(0.0, 0.0, 10.0, 5.0);
        b.unite_rect(0.0, 5.0, 10.0, 10.0);
        assert_eq!(a, b);
    }

    #[test]
    fn unite_empty_rect_noop() {
        let mut cr = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        cr.unite_rect(5.0, 5.0, 5.0, 10.0);
        assert_eq!(cr.GetCount(), 1);
    }

    #[test]
    fn subtract_from_empty_noop() {
        let mut cr = ClipRects::new();
        cr.subtract_rect(0.0, 0.0, 10.0, 10.0);
        assert!(cr.IsEmpty());
    }

    #[test]
    fn is_subset_of_check() {
        let a = ClipRects::from_rect(2.0, 2.0, 8.0, 8.0);
        let b = ClipRects::from_rect(0.0, 0.0, 10.0, 10.0);
        assert!(a.IsSubsetOf(&b));
        assert!(!b.IsSubsetOf(&a));
    }
}


#[cfg(kani)]
mod kani_private_proofs {
    use super::*;

    #[kani::proof]
    fn kani_private_ClipRect_new() {
        let mut p_x1: f64 = kani::any::<f64>();
        kani::assume(p_x1.is_finite());
        let mut p_y1: f64 = kani::any::<f64>();
        kani::assume(p_y1.is_finite());
        let mut p_x2: f64 = kani::any::<f64>();
        kani::assume(p_x2.is_finite());
        let mut p_y2: f64 = kani::any::<f64>();
        kani::assume(p_y2.is_finite());
        let _r = ClipRect::new(p_x1, p_y1, p_x2, p_y2);
    }
}
