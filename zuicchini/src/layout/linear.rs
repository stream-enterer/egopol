use std::collections::HashMap;

use crate::foundation::Rect;
use crate::panel::{NoticeFlags, PanelBehavior, PanelCtx, PanelId};
use crate::render::Painter;

use super::{
    get_constraint, Alignment, ChildConstraint, Orientation, ResolvedOrientation, Spacing,
};

/// Linear layout: arranges children along a single axis with weighted distribution.
pub struct LinearLayout {
    pub orientation: Orientation,
    pub alignment: Alignment,
    pub spacing: Spacing,
    pub child_constraints: HashMap<PanelId, ChildConstraint>,
    pub default_constraint: ChildConstraint,
    /// Minimum number of cells (pads with empty space if fewer children).
    pub min_cell_count: usize,
}

impl LinearLayout {
    pub fn horizontal() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            alignment: Alignment::default(),
            spacing: Spacing::default(),
            child_constraints: HashMap::new(),
            default_constraint: ChildConstraint::default(),
            min_cell_count: 0,
        }
    }

    pub fn vertical() -> Self {
        Self {
            orientation: Orientation::Vertical,
            ..Self::horizontal()
        }
    }

    pub fn adaptive(tallness_threshold: f64) -> Self {
        Self {
            orientation: Orientation::Adaptive { tallness_threshold },
            ..Self::horizontal()
        }
    }

    pub fn with_spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn set_child_constraint(&mut self, child: PanelId, constraint: ChildConstraint) {
        self.child_constraints.insert(child, constraint);
    }

    fn do_layout(&mut self, ctx: &mut PanelCtx) {
        let Rect { w, h, .. } = ctx.layout_rect();
        let children = ctx.children();
        if children.is_empty() {
            return;
        }

        let resolved = self.orientation.resolve(w, h);
        let horizontal = resolved == ResolvedOrientation::Horizontal;
        let sp = &self.spacing;

        let (main_total, cross_total, mms, mme, mcs, mce, inner_main) = if horizontal {
            (
                w,
                h,
                sp.margin_left,
                sp.margin_right,
                sp.margin_top,
                sp.margin_bottom,
                sp.inner_h,
            )
        } else {
            (
                h,
                w,
                sp.margin_top,
                sp.margin_bottom,
                sp.margin_left,
                sp.margin_right,
                sp.inner_v,
            )
        };

        let cell_count = children.len().max(self.min_cell_count);
        let gap_count = cell_count.saturating_sub(1);

        // Total weight including padding for min_cell_count
        let children_weight: f64 = children
            .iter()
            .map(|c| get_constraint(&self.child_constraints, *c, &self.default_constraint).weight)
            .sum::<f64>();
        let pad_count = cell_count.saturating_sub(children.len());
        let total_weight: f64 =
            (children_weight + pad_count as f64 * self.default_constraint.weight).max(1e-100);

        // Proportional spacing: compute scale factors.
        // Content occupies total_weight proportion-units on main, 1.0 on cross.
        let denom_main = mms + inner_main * gap_count as f64 + mme + total_weight;
        let denom_cross = mcs + mce + 1.0;
        if denom_main < 1e-100 || denom_cross < 1e-100 {
            return;
        }

        let s_main = main_total / denom_main;
        let s_cross = cross_total / denom_cross;

        // Available dimensions in pixels for the content area
        let available_main = total_weight * s_main;
        let cross_px = s_cross; // 1.0 * s_cross

        // Calculate force: iterative solver in pixel space
        let force =
            self.calculate_force(&children, available_main, cross_px, horizontal, cell_count);

        // Compute each child's main size from force, clamped by tallness
        let mut main_sizes = Vec::with_capacity(children.len());
        for child in &children {
            let cc = get_constraint(&self.child_constraints, *child, &self.default_constraint);
            let mut main_size = cc.weight * force;

            if main_size > 0.0 {
                let (cw, ch) = if horizontal {
                    (main_size, cross_px)
                } else {
                    (cross_px, main_size)
                };
                let tallness = ch / cw;
                let clamped = tallness.clamp(cc.min_tallness, cc.max_tallness);
                if (clamped - tallness).abs() > 1e-10 {
                    main_size = if horizontal {
                        cross_px / clamped
                    } else {
                        cross_px * clamped
                    };
                }
            }

            main_sizes.push(main_size);
        }

        // Place children
        let actual_mm = mms * s_main;
        let actual_mc = mcs * s_cross;
        let actual_gap = inner_main * s_main;

        let mut main_pos = actual_mm;
        for (i, child) in children.iter().enumerate() {
            let main_size = main_sizes[i];
            let (x, y, cw, ch) = if horizontal {
                (main_pos, actual_mc, main_size, cross_px)
            } else {
                (actual_mc, main_pos, cross_px, main_size)
            };
            ctx.layout_child(*child, x, y, cw, ch);
            main_pos += main_size + actual_gap;
        }
    }

    /// Iterative force solver matching C++ CalculateForce.
    /// Returns force (pixels per weight unit) that distributes `total_length`
    /// among free children while respecting tallness constraints.
    fn calculate_force(
        &self,
        children: &[PanelId],
        total_length: f64,
        cross: f64,
        horizontal: bool,
        cell_count: usize,
    ) -> f64 {
        let n = children.len();
        if n == 0 || total_length <= 0.0 {
            return 0.0;
        }

        // Child state: None = free, Some(fixed_size) = constrained
        // Track compressed vs expanded separately for conflict resolution
        let constraints: Vec<&ChildConstraint> = children
            .iter()
            .map(|c| get_constraint(&self.child_constraints, *c, &self.default_constraint))
            .collect();

        #[derive(Clone, Copy, PartialEq)]
        enum State {
            Free,
            Compressed(f64),
            Expanded(f64),
        }

        let mut states = vec![State::Free; n];
        // Include min_cell_count padding in free weight
        let pad_weight = if cell_count > n {
            (cell_count - n) as f64 * self.default_constraint.weight
        } else {
            0.0
        };
        let mut free_weight: f64 = constraints.iter().map(|c| c.weight).sum::<f64>() + pad_weight;
        let mut free_length = total_length;

        for _ in 0..n + 2 {
            if free_weight <= 0.0 {
                break;
            }
            let force = free_length / free_weight;

            let mut any_changed = false;
            let mut has_compressed = false;
            let mut has_expanded = false;

            for i in 0..n {
                if states[i] != State::Free {
                    continue;
                }
                let cc = constraints[i];
                let main_size = cc.weight * force;
                if main_size <= 0.0 {
                    continue;
                }

                let (cw, ch) = if horizontal {
                    (main_size, cross)
                } else {
                    (cross, main_size)
                };
                if cw <= 0.0 {
                    continue;
                }
                let tallness = ch / cw;

                if tallness > cc.max_tallness {
                    // Too tall → child needs more main space → expanded
                    let fixed = if horizontal {
                        cross / cc.max_tallness
                    } else {
                        cross * cc.max_tallness
                    };
                    states[i] = State::Expanded(fixed);
                    free_weight -= cc.weight;
                    free_length -= fixed;
                    any_changed = true;
                    has_expanded = true;
                } else if tallness < cc.min_tallness {
                    // Too wide → child needs less main space → compressed
                    let fixed = if horizontal {
                        cross / cc.min_tallness
                    } else {
                        cross * cc.min_tallness
                    };
                    states[i] = State::Compressed(fixed);
                    free_weight -= cc.weight;
                    free_length -= fixed;
                    any_changed = true;
                    has_compressed = true;
                }
            }

            if !any_changed {
                break;
            }

            // Conflict resolution: if both compressed and expanded exist
            if has_compressed && has_expanded {
                let committed: f64 = states
                    .iter()
                    .map(|s| match s {
                        State::Compressed(f) | State::Expanded(f) => *f,
                        State::Free => 0.0,
                    })
                    .sum();

                // C++ CalculateForce names its constrained lists by
                // tallness direction, which flips between orientations.
                // In horizontal mode Expanded (tallness>max, larger main)
                // maps to C++ "compressed", while in vertical it maps to
                // C++ "expanded". C++ always keeps "compressed" on over-
                // commit, so the Rust release target differs by axis.
                let release_expanded = if horizontal {
                    committed <= total_length
                } else {
                    committed > total_length
                };
                for i in 0..n {
                    let release = match states[i] {
                        State::Expanded(_) => release_expanded,
                        State::Compressed(_) => !release_expanded,
                        State::Free => false,
                    };
                    if release {
                        states[i] = State::Free;
                        free_weight += constraints[i].weight;
                    }
                }

                free_length = total_length
                    - states
                        .iter()
                        .map(|s| match s {
                            State::Compressed(f) | State::Expanded(f) => *f,
                            State::Free => 0.0,
                        })
                        .sum::<f64>();
            }
        }

        if free_weight > 0.0 {
            free_length / free_weight
        } else {
            0.0
        }
    }
}

impl PanelBehavior for LinearLayout {
    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        self.do_layout(ctx);
    }

    fn notice(&mut self, _flags: NoticeFlags) {}
}

/// LinearGroup: a LinearLayout that also paints a border and is focusable.
pub struct LinearGroup {
    pub layout: LinearLayout,
}

impl LinearGroup {
    pub fn horizontal() -> Self {
        Self {
            layout: LinearLayout::horizontal(),
        }
    }

    pub fn vertical() -> Self {
        Self {
            layout: LinearLayout::vertical(),
        }
    }
}

impl PanelBehavior for LinearGroup {
    fn paint(&mut self, _painter: &mut Painter, _w: f64, _h: f64) {}

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        self.layout.do_layout(ctx);
    }

    fn auto_expand(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::PanelTree;

    fn setup_tree(n: usize) -> (PanelTree, PanelId, Vec<PanelId>) {
        let mut tree = PanelTree::new();
        let root = tree.create_root("root");
        tree.set_layout_rect(root, 0.0, 0.0, 400.0, 200.0);
        let mut children = Vec::new();
        for i in 0..n {
            let c = tree.create_child(root, &format!("child_{i}"));
            children.push(c);
        }
        (tree, root, children)
    }

    #[test]
    fn horizontal_equal_weight() {
        let (mut tree, root, children) = setup_tree(4);
        let mut layout = LinearLayout::horizontal();
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        // Each child should get 100px wide, 200px tall
        for (i, child) in children.iter().enumerate() {
            let r = tree.get(*child).unwrap().layout_rect;
            assert!((r.w - 100.0).abs() < 0.01, "child {i} width: {}", r.w);
            assert!((r.h - 200.0).abs() < 0.01, "child {i} height: {}", r.h);
            assert!(
                (r.x - (i as f64 * 100.0)).abs() < 0.01,
                "child {i} x: {}",
                r.x
            );
            assert!((r.y - 0.0).abs() < 0.01, "child {i} y: {}", r.y);
        }
    }

    #[test]
    fn vertical_equal_weight() {
        let (mut tree, root, children) = setup_tree(2);
        tree.set_layout_rect(root, 0.0, 0.0, 300.0, 400.0);
        let mut layout = LinearLayout::vertical();
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        for (i, child) in children.iter().enumerate() {
            let r = tree.get(*child).unwrap().layout_rect;
            assert!((r.w - 300.0).abs() < 0.01, "child {i} width: {}", r.w);
            assert!((r.h - 200.0).abs() < 0.01, "child {i} height: {}", r.h);
            assert!(
                (r.y - (i as f64 * 200.0)).abs() < 0.01,
                "child {i} y: {}",
                r.y
            );
        }
    }

    #[test]
    fn weighted_distribution() {
        let (mut tree, root, children) = setup_tree(3);
        tree.set_layout_rect(root, 0.0, 0.0, 300.0, 100.0);
        let mut layout = LinearLayout::horizontal();
        layout.set_child_constraint(
            children[0],
            ChildConstraint {
                weight: 1.0,
                ..Default::default()
            },
        );
        layout.set_child_constraint(
            children[1],
            ChildConstraint {
                weight: 2.0,
                ..Default::default()
            },
        );
        layout.set_child_constraint(
            children[2],
            ChildConstraint {
                weight: 1.0,
                ..Default::default()
            },
        );
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        let w0 = tree.get(children[0]).unwrap().layout_rect.w;
        let w1 = tree.get(children[1]).unwrap().layout_rect.w;
        let w2 = tree.get(children[2]).unwrap().layout_rect.w;
        assert!((w0 - 75.0).abs() < 0.01);
        assert!((w1 - 150.0).abs() < 0.01);
        assert!((w2 - 75.0).abs() < 0.01);
    }

    #[test]
    fn spacing() {
        // Proportional spacing: margin=0.5 each side, inner=1.0
        // 2 children (weight 1.0 each): denom_x = 0.5 + 1.0 + 0.5 + 2.0 = 4.0
        // sx = 200/4 = 50. margin = 25, gap = 50, cell = 50.
        // denom_y = 0 + 0 + 1.0 = 1.0, sy = 100.
        let (mut tree, root, children) = setup_tree(2);
        tree.set_layout_rect(root, 0.0, 0.0, 200.0, 100.0);
        let mut layout = LinearLayout::horizontal().with_spacing(Spacing {
            inner_h: 1.0,
            inner_v: 0.0,
            margin_left: 0.5,
            margin_right: 0.5,
            margin_top: 0.0,
            margin_bottom: 0.0,
        });
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        let r0 = tree.get(children[0]).unwrap().layout_rect;
        let r1 = tree.get(children[1]).unwrap().layout_rect;
        assert!((r0.x - 25.0).abs() < 0.01, "r0.x: {}", r0.x);
        assert!((r0.w - 50.0).abs() < 0.01, "r0.w: {}", r0.w);
        assert!((r1.x - 125.0).abs() < 0.01, "r1.x: {}", r1.x); // 25 + 50 + 50
        assert!((r1.w - 50.0).abs() < 0.01, "r1.w: {}", r1.w);
        assert!((r0.h - 100.0).abs() < 0.01, "r0.h: {}", r0.h);
    }

    #[test]
    fn tallness_constraints() {
        // Horizontal layout, 400x200, 2 children weight 1.0 each.
        // No spacing: denom_x = 2, sx = 200. denom_y = 1, sy = 200.
        // Each child: cw = 200, ch = 200, tallness = 1.0.
        // Child 0: max_tallness = 0.5 → cw = 200/0.5 = 400 (expanded)
        // Force solver marks child 0 as expanded with fixed=400.
        // free_length = 400-400 = 0, free_weight = 1.0, force = 0.
        // Child 1 gets 0 width. Tallness = 200/0 → skip.
        // Actually let me use a more reasonable scenario.
        //
        // 2 children in 600x100, weight [1.0, 1.0].
        // denom_x = 2, sx = 300. denom_y = 1, sy = 100.
        // available_main = 600. force = 300.
        // Child 0: cw=300, ch=100, tallness=0.333. Constraint min_tallness=0.5.
        // tallness < min → compressed. fixed = 100/0.5 = 200.
        // free_length = 600-200 = 400, free_weight=1.0, force = 400.
        // Child 1: cw=400, ch=100, tallness=0.25. No constraint → OK.
        let (mut tree, root, children) = setup_tree(2);
        tree.set_layout_rect(root, 0.0, 0.0, 600.0, 100.0);
        let mut layout = LinearLayout::horizontal();
        layout.set_child_constraint(
            children[0],
            ChildConstraint {
                weight: 1.0,
                min_tallness: 0.5,
                ..Default::default()
            },
        );
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        let r0 = tree.get(children[0]).unwrap().layout_rect;
        let r1 = tree.get(children[1]).unwrap().layout_rect;
        assert!((r0.w - 200.0).abs() < 0.01, "r0.w: {}", r0.w);
        assert!((r0.h - 100.0).abs() < 0.01, "r0.h: {}", r0.h);
        assert!((r1.w - 400.0).abs() < 0.01, "r1.w: {}", r1.w);
    }

    #[test]
    fn force_convergence() {
        // 3 children in 900x100, weights [1,1,1].
        // denom_x=3, sx=300. available_main=900. force=300.
        // Child 0: min_tallness=0.5 → tallness=100/300=0.333 < 0.5 → compressed to 200.
        // Child 1: max_tallness=0.2 → tallness=100/300=0.333 > 0.2 → expanded to 500.
        // Both compressed AND expanded → committed=700 < 900 → release compressed.
        // free: child 0 + child 2, free_weight=2, free_length=900-500=400. force=200.
        // Child 0: tallness=100/200=0.5 = min_tallness → OK.
        // Child 2: tallness=100/200=0.5 → OK (default constraints).
        // Converged. force=200.
        let (mut tree, root, children) = setup_tree(3);
        tree.set_layout_rect(root, 0.0, 0.0, 900.0, 100.0);
        let mut layout = LinearLayout::horizontal();
        layout.set_child_constraint(
            children[0],
            ChildConstraint {
                weight: 1.0,
                min_tallness: 0.5,
                ..Default::default()
            },
        );
        layout.set_child_constraint(
            children[1],
            ChildConstraint {
                weight: 1.0,
                max_tallness: 0.2,
                ..Default::default()
            },
        );
        layout.do_layout(&mut PanelCtx::new(&mut tree, root));

        let r0 = tree.get(children[0]).unwrap().layout_rect;
        let r1 = tree.get(children[1]).unwrap().layout_rect;
        let r2 = tree.get(children[2]).unwrap().layout_rect;
        assert!((r0.w - 200.0).abs() < 0.01, "r0.w: {}", r0.w);
        assert!((r1.w - 500.0).abs() < 0.01, "r1.w: {}", r1.w);
        assert!((r2.w - 200.0).abs() < 0.01, "r2.w: {}", r2.w);
    }
}
