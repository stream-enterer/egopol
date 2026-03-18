#[allow(dead_code)]
mod common;

use gungraun::{library_benchmark, library_benchmark_group, main};
use zuicchini::foundation::Image;
use zuicchini::panel::{PanelTree, View};
use zuicchini::render::TileCache;

use common::{run_one_frame, setup_tree_and_view, DEFAULT_VH, DEFAULT_VW, SCENARIOS};

type ScenarioState = (PanelTree, View, Image, TileCache, f64, f64, usize);

fn setup_scenario(idx: usize) -> ScenarioState {
    let (mut tree, mut view, _) = setup_tree_and_view(DEFAULT_VW, DEFAULT_VH);
    let mut buf = Image::new(DEFAULT_VW, DEFAULT_VH, 4);
    let mut tc = TileCache::new(DEFAULT_VW, DEFAULT_VH, 256);
    let fx = DEFAULT_VW as f64 / 2.0;
    let fy = DEFAULT_VH as f64 / 2.0;
    // Warmup frame
    run_one_frame(
        &mut tree,
        &mut view,
        &mut buf,
        &mut tc,
        &SCENARIOS[idx],
        fx,
        fy,
    );
    (tree, view, buf, tc, fx, fy, idx)
}

fn run_scenario(state: ScenarioState) {
    let (mut tree, mut view, mut buf, mut tc, fx, fy, idx) = state;
    run_one_frame(
        &mut tree,
        &mut view,
        &mut buf,
        &mut tc,
        &SCENARIOS[idx],
        fx,
        fy,
    );
}

#[library_benchmark]
#[bench::run(args = (0), setup = setup_scenario)]
fn bench_static(state: ScenarioState) {
    run_scenario(state);
}

#[library_benchmark]
#[bench::run(args = (1), setup = setup_scenario)]
fn bench_pan(state: ScenarioState) {
    run_scenario(state);
}

#[library_benchmark]
#[bench::run(args = (2), setup = setup_scenario)]
fn bench_zoom_in(state: ScenarioState) {
    run_scenario(state);
}

#[library_benchmark]
#[bench::run(args = (3), setup = setup_scenario)]
fn bench_zoom_out(state: ScenarioState) {
    run_scenario(state);
}

#[library_benchmark]
#[bench::run(args = (4), setup = setup_scenario)]
fn bench_pan_zoom(state: ScenarioState) {
    run_scenario(state);
}

library_benchmark_group!(
    name = interaction,
    benchmarks = [bench_static, bench_pan, bench_zoom_in, bench_zoom_out, bench_pan_zoom]
);

fn main() {
    main!(library_benchmark_groups = interaction);
}
