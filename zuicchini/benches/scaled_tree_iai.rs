#[allow(dead_code)]
mod common;

use gungraun::{library_benchmark, library_benchmark_group, main};
use zuicchini::foundation::{Color, Image};
use zuicchini::panel::{PanelTree, View};
use zuicchini::render::Painter;

use common::scaled::{build_scaled_tree, run_one_scaled_frame};
use common::{DEFAULT_VH, DEFAULT_VW};

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

type FrameState = (PanelTree, View, Image);

fn setup_pan_zoom(count: usize) -> FrameState {
    let (mut tree, mut view, _) = build_scaled_tree(count);
    let mut buf = Image::new(DEFAULT_VW, DEFAULT_VH, 4);
    // Warmup
    run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);
    (tree, view, buf)
}

fn setup_paint(count: usize) -> FrameState {
    let (tree, view, _) = build_scaled_tree(count);
    let buf = Image::new(DEFAULT_VW, DEFAULT_VH, 4);
    (tree, view, buf)
}

fn setup_update(count: usize) -> (PanelTree, View) {
    let (tree, view, _) = build_scaled_tree(count);
    (tree, view)
}

// ---------------------------------------------------------------------------
// Pan+Zoom benchmarks
// ---------------------------------------------------------------------------

#[library_benchmark]
#[bench::run(args = (10), setup = setup_pan_zoom)]
fn pan_zoom_10(state: FrameState) {
    let (mut tree, mut view, mut buf) = state;
    run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);
}

#[library_benchmark]
#[bench::run(args = (50), setup = setup_pan_zoom)]
fn pan_zoom_50(state: FrameState) {
    let (mut tree, mut view, mut buf) = state;
    run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);
}

#[library_benchmark]
#[bench::run(args = (200), setup = setup_pan_zoom)]
fn pan_zoom_200(state: FrameState) {
    let (mut tree, mut view, mut buf) = state;
    run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);
}

// ---------------------------------------------------------------------------
// Paint benchmarks
// ---------------------------------------------------------------------------

#[library_benchmark]
#[bench::run(args = (10), setup = setup_paint)]
fn paint_10(state: FrameState) {
    let (mut tree, view, mut buf) = state;
    buf.fill(Color::BLACK);
    let mut painter = Painter::new(&mut buf);
    view.paint(&mut tree, &mut painter);
}

#[library_benchmark]
#[bench::run(args = (50), setup = setup_paint)]
fn paint_50(state: FrameState) {
    let (mut tree, view, mut buf) = state;
    buf.fill(Color::BLACK);
    let mut painter = Painter::new(&mut buf);
    view.paint(&mut tree, &mut painter);
}

#[library_benchmark]
#[bench::run(args = (200), setup = setup_paint)]
fn paint_200(state: FrameState) {
    let (mut tree, view, mut buf) = state;
    buf.fill(Color::BLACK);
    let mut painter = Painter::new(&mut buf);
    view.paint(&mut tree, &mut painter);
}

// ---------------------------------------------------------------------------
// Update benchmarks
// ---------------------------------------------------------------------------

#[library_benchmark]
#[bench::run(args = (10), setup = setup_update)]
fn update_10(state: (PanelTree, View)) {
    let (mut tree, mut view) = state;
    tree.deliver_notices(true, 1.0);
    view.update(&mut tree);
}

#[library_benchmark]
#[bench::run(args = (50), setup = setup_update)]
fn update_50(state: (PanelTree, View)) {
    let (mut tree, mut view) = state;
    tree.deliver_notices(true, 1.0);
    view.update(&mut tree);
}

#[library_benchmark]
#[bench::run(args = (200), setup = setup_update)]
fn update_200(state: (PanelTree, View)) {
    let (mut tree, mut view) = state;
    tree.deliver_notices(true, 1.0);
    view.update(&mut tree);
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

library_benchmark_group!(
    name = scaled_pan_zoom,
    benchmarks = [pan_zoom_10, pan_zoom_50, pan_zoom_200]
);

library_benchmark_group!(
    name = scaled_paint,
    benchmarks = [paint_10, paint_50, paint_200]
);

library_benchmark_group!(
    name = scaled_update,
    benchmarks = [update_10, update_50, update_200]
);

fn main() {
    main!(library_benchmark_groups = [scaled_pan_zoom, scaled_paint, scaled_update]);
}
