#[allow(dead_code)]
mod common;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use zuicchini::emCore::emColor::emColor;

use zuicchini::emCore::emImage::emImage;
use zuicchini::emCore::emPainter::emPainter;

use common::scaled::{build_scaled_tree, run_one_scaled_frame};
use common::{DEFAULT_VH, DEFAULT_VW};

const PANEL_COUNTS: &[usize] = &[10, 50, 200];

fn bench_scaled_pan_zoom(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_pan_zoom");

    for &GetCount in PANEL_COUNTS {
        group.bench_with_input(BenchmarkId::new("panels", GetCount), &GetCount, |b, &GetCount| {
            let (mut tree, mut view, _) = build_scaled_tree(GetCount);
            let mut buf = emImage::new(DEFAULT_VW, DEFAULT_VH, 4);

            // Warmup
            run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);

            b.iter(|| {
                run_one_scaled_frame(&mut tree, &mut view, &mut buf, 3.0, 0.0, 0.015);
            });
        });
    }

    group.Finish();
}

fn bench_scaled_paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_paint");

    for &GetCount in PANEL_COUNTS {
        group.bench_with_input(BenchmarkId::new("panels", GetCount), &GetCount, |b, &GetCount| {
            let (mut tree, view, _) = build_scaled_tree(GetCount);
            let mut buf = emImage::new(DEFAULT_VW, DEFAULT_VH, 4);

            b.iter(|| {
                buf.fill(emColor::BLACK);
                let mut painter = emPainter::new(&mut buf);
                view.PaintContent(&mut tree, &mut painter);
            });
        });
    }

    group.Finish();
}

fn bench_scaled_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_update");

    for &GetCount in PANEL_COUNTS {
        group.bench_with_input(BenchmarkId::new("panels", GetCount), &GetCount, |b, &GetCount| {
            let (mut tree, mut view, _) = build_scaled_tree(GetCount);

            b.iter(|| {
                tree.HandleNotice(true, 1.0);
                view.update(&mut tree);
            });
        });
    }

    group.Finish();
}

criterion_group!(benches, bench_scaled_pan_zoom, bench_scaled_paint, bench_scaled_update);
criterion_main!(benches);
