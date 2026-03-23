#[allow(dead_code)]
mod common;

use criterion::{criterion_group, criterion_main, Criterion};
use zuicchini::emCore::emImage::emImage;
use zuicchini::emCore::emViewRendererTileCache::TileCache;

use common::{run_one_frame, setup_tree_and_view, DEFAULT_VH, DEFAULT_VW, SCENARIOS};

fn bench_interaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("interaction");

    for scenario in SCENARIOS {
        group.bench_function(scenario.name, |b| {
            let (mut tree, mut view, _) = setup_tree_and_view(DEFAULT_VW, DEFAULT_VH);
            let mut buf = emImage::new(DEFAULT_VW, DEFAULT_VH, 4);
            let mut tc = TileCache::new(DEFAULT_VW, DEFAULT_VH, 256);
            let fix_x = DEFAULT_VW as f64 / 2.0;
            let fix_y = DEFAULT_VH as f64 / 2.0;

            // Warmup
            run_one_frame(
                &mut tree, &mut view, &mut buf, &mut tc, scenario, fix_x, fix_y,
            );

            b.iter(|| {
                run_one_frame(
                    &mut tree, &mut view, &mut buf, &mut tc, scenario, fix_x, fix_y,
                );
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_interaction);
criterion_main!(benches);
