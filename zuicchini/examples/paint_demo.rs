//! Paint demo derived from C++ `PaintExample.cpp` and `emTestPanel.cpp` paint section.
//!
//! A single panel exercising every major Painter drawing primitive:
//! images, rectangles, ellipses, polygons, lines, beziers, gradients,
//! textured polygons, clipping, stroke end types, and text rendering.

use std::f64::consts::PI;

use zuicchini::foundation::{Color, Image};
use zuicchini::panel::{PanelBehavior, PanelState, ViewFlags};
use zuicchini::render::{
    ImageExtension, ImageQuality, LineCap, LineJoin, Painter, Stroke, StrokeEnd, StrokeEndType,
    TextAlignment, Texture, VAlign,
};
use zuicchini::window::{App, WindowFlags};

struct PaintPanel {
    test_image: Image,
}

impl PaintPanel {
    fn new() -> Self {
        let mut img = Image::new(64, 64, 4);
        for y in 0..64u32 {
            for x in 0..64u32 {
                img.set_pixel_channel(x, y, 0, (x * 4) as u8);
                img.set_pixel_channel(x, y, 1, (y * 4) as u8);
                img.set_pixel_channel(x, y, 2, 128);
                img.set_pixel_channel(x, y, 3, 255);
            }
        }
        Self { test_image: img }
    }
}

impl PanelBehavior for PaintPanel {
    fn is_opaque(&self) -> bool {
        true
    }

    fn paint(&mut self, p: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        // Background
        p.paint_rect(0.0, 0.0, w, h, Color::WHITE, Color::TRANSPARENT);

        // ── Section 1: Image ──
        p.paint_image_scaled(
            0.05 * w,
            0.05 * h,
            0.2 * w,
            0.2 * h,
            &self.test_image,
            ImageQuality::Bilinear,
            ImageExtension::Clamp,
        );

        // ── Section 2: Filled shapes ──
        p.paint_rect(
            0.3 * w,
            0.05 * h,
            0.15 * w,
            0.15 * h,
            Color::GREEN,
            Color::TRANSPARENT,
        );

        // Ellipse (center + radius)
        p.paint_ellipse(
            0.6 * w,
            0.15 * h,
            0.08 * w,
            0.08 * h,
            Color::rgba(0x33, 0xCC, 0x88, 0xFF),
            Color::TRANSPARENT,
        );

        // Triangle
        p.paint_polygon(
            &[
                (0.05 * w, 0.35 * h),
                (0.25 * w, 0.45 * h),
                (0.02 * w, 0.55 * h),
            ],
            Color::rgba(255, 128, 0, 255),
            Color::TRANSPARENT,
        );

        // Round rect
        p.paint_round_rect(
            0.3 * w,
            0.30 * h,
            0.15 * w,
            0.15 * h,
            0.02 * w,
            Color::rgba(0x88, 0x44, 0xCC, 0xFF),
        );

        // ── Section 3: Outlines ──
        let outline = Stroke::new(Color::rgba(0x00, 0x80, 0xC0, 0xFF), 0.005 * w);
        p.paint_rect_outlined(
            0.5 * w,
            0.30 * h,
            0.15 * w,
            0.15 * h,
            &outline,
            Color::TRANSPARENT,
        );

        p.paint_ellipse_outlined(
            0.77 * w,
            0.37 * h,
            0.08 * w,
            0.08 * h,
            &outline,
            Color::TRANSPARENT,
        );

        p.paint_polygon_outlined(
            &[
                (0.05 * w, 0.60 * h),
                (0.15 * w, 0.58 * h),
                (0.20 * w, 0.68 * h),
                (0.10 * w, 0.72 * h),
            ],
            Color::rgba(255, 0, 0, 255),
            0.003 * w,
            Color::TRANSPARENT,
        );

        p.paint_round_rect_outlined(0.25 * w, 0.55 * h, 0.15 * w, 0.15 * h, 0.02 * w, &outline);

        // ── Section 4: Text ──
        p.paint_text_boxed(
            0.50 * w,
            0.05 * h,
            0.25 * w,
            0.20 * h,
            "Centered text\nin\nthe bottom-right\nof a box",
            0.017 * w,
            Color::rgba(0, 0x80, 0xC0, 0xFF),
            Color::TRANSPARENT,
            TextAlignment::Right,
            VAlign::Bottom,
            TextAlignment::Center,
            0.5,
            true,
            0.15,
        );

        p.paint_text(
            0.78 * w,
            0.05 * h,
            "paint_text()",
            0.02 * w,
            1.0,
            Color::BLACK,
            Color::TRANSPARENT,
        );

        // ── Section 5: Stroked line ──
        let mut stroke_line = Stroke::new(Color::rgba(255, 0, 0, 128), 0.015 * w);
        stroke_line.cap = LineCap::Round;
        stroke_line.join = LineJoin::Round;
        stroke_line.start_end = StrokeEnd::new(StrokeEndType::Cap);
        stroke_line.finish_end = StrokeEnd::new(StrokeEndType::Cap);
        p.paint_line_stroked(
            0.45 * w,
            0.55 * h,
            0.65 * w,
            0.72 * h,
            &stroke_line,
            Color::TRANSPARENT,
        );

        // ── Section 6: Bezier curves ──
        let bezier_pts = [
            (0.70 * w, 0.55 * h),
            (0.60 * w, 0.60 * h),
            (0.80 * w, 0.65 * h),
            (0.70 * w, 0.72 * h),
        ];
        p.paint_bezier(
            &bezier_pts,
            Color::rgba(0x00, 0xAA, 0x00, 0xFF),
            Color::TRANSPARENT,
        );

        let bezier_stroke = Stroke::new(Color::rgba(0xCC, 0x00, 0x88, 0xFF), 0.003 * w);
        p.paint_bezier_outline(&bezier_pts, &bezier_stroke, Color::TRANSPARENT);

        let mut arrow_stroke = Stroke::new(Color::rgba(0x00, 0x00, 0xFF, 0xFF), 0.004 * w);
        arrow_stroke.cap = LineCap::Round;
        arrow_stroke.join = LineJoin::Round;
        arrow_stroke.start_end =
            StrokeEnd::new(StrokeEndType::ContourTriangle).with_inner_color(Color::RED);
        arrow_stroke.finish_end = StrokeEnd::new(StrokeEndType::Arrow);
        let bezier_pts2 = [
            (0.82 * w, 0.55 * h),
            (0.92 * w, 0.60 * h),
            (0.82 * w, 0.65 * h),
            (0.92 * w, 0.72 * h),
        ];
        p.paint_bezier_line(&bezier_pts2, &arrow_stroke, Color::TRANSPARENT);

        // ── Section 7: Gradients ──
        p.paint_linear_gradient(
            0.05 * w,
            0.78 * h,
            0.15 * w,
            0.08 * h,
            Color::rgba(0, 0xFF, 0, 0x80),
            Color::rgba(0xFF, 0xFF, 0, 0xFF),
            true,
            Color::TRANSPARENT,
        );

        p.paint_radial_gradient(
            0.30 * w,
            0.82 * h,
            0.08 * w,
            0.06 * h,
            Color::rgba(0xFF, 0x88, 0, 0xFF),
            Color::rgba(0, 0x55, 0, 0xFF),
            Color::TRANSPARENT,
        );

        // ── Section 8: Textured polygons ──
        let star = make_star(0.55 * w, 0.84 * h, 0.06 * w, 0.06 * h, 5);
        p.paint_polygon_textured(
            &star,
            &Texture::LinearGradient {
                color_a: Color::rgba(0, 0xFF, 0, 0x80),
                color_b: Color::rgba(0xFF, 0xFF, 0, 0xFF),
                start: (0.49 * w, 0.78 * h),
                end: (0.61 * w, 0.90 * h),
            },
            Color::TRANSPARENT,
        );

        let star2 = make_star(0.70 * w, 0.84 * h, 0.06 * w, 0.06 * h, 5);
        p.paint_polygon_textured(
            &star2,
            &Texture::RadialGradient {
                color_inner: Color::rgba(0xCC, 0xCC, 0x33, 0xFF),
                color_outer: Color::rgba(0, 0, 0xFF, 0x60),
                center: (0.70 * w, 0.84 * h),
                radius: 0.06 * w,
            },
            Color::TRANSPARENT,
        );

        let star3 = make_star(0.85 * w, 0.84 * h, 0.06 * w, 0.06 * h, 5);
        p.paint_polygon_textured(
            &star3,
            &Texture::Image {
                image: self.test_image.clone(),
                extension: ImageExtension::Repeat,
                quality: ImageQuality::Bilinear,
            },
            Color::TRANSPARENT,
        );

        // ── Section 9: Clipping demo ──
        p.push_state();
        p.clip_rect(0.05 * w, 0.88 * h, 0.15 * w, 0.10 * h);
        // Draw a circle that extends beyond the clip rectangle
        let verts: Vec<(f64, f64)> = (0..64)
            .map(|i| {
                let a = PI * 2.0 * i as f64 / 64.0;
                (
                    a.cos() * 0.10 * w + 0.125 * w,
                    a.sin() * 0.10 * h + 0.93 * h,
                )
            })
            .collect();
        p.paint_polygon(&verts, Color::rgba(255, 255, 0, 180), Color::TRANSPARENT);
        p.pop_state();

        // ── Section 10: All 17 StrokeEndType variants ──
        let end_types = [
            StrokeEndType::Butt,
            StrokeEndType::Cap,
            StrokeEndType::Arrow,
            StrokeEndType::ContourArrow,
            StrokeEndType::LineArrow,
            StrokeEndType::Triangle,
            StrokeEndType::ContourTriangle,
            StrokeEndType::Square,
            StrokeEndType::ContourSquare,
            StrokeEndType::HalfSquare,
            StrokeEndType::Circle,
            StrokeEndType::ContourCircle,
            StrokeEndType::HalfCircle,
            StrokeEndType::Diamond,
            StrokeEndType::ContourDiamond,
            StrokeEndType::HalfDiamond,
            StrokeEndType::Stroke,
        ];
        let n = end_types.len();
        let center_x = 0.45 * w;
        let center_y = 0.92 * h;
        let inner_r = 0.01 * w;
        let outer_r = 0.04 * w;
        for (i, &et) in end_types.iter().enumerate() {
            for side in 0..2u32 {
                let idx = i * 2 + side as usize;
                let a = 2.0 * PI * idx as f64 / (2 * n) as f64;
                let mut s = Stroke::new(Color::WHITE, 0.002 * w);
                if side == 1 {
                    s.cap = LineCap::Round;
                    s.join = LineJoin::Round;
                }
                s.start_end = StrokeEnd::new(StrokeEndType::Cap);
                s.finish_end =
                    StrokeEnd::new(et).with_inner_color(Color::rgba(0xFF, 0xFF, 0xFF, 0x40));
                p.paint_line_stroked(
                    center_x + inner_r * a.cos(),
                    center_y + inner_r * a.sin(),
                    center_x + outer_r * a.cos(),
                    center_y + outer_r * a.sin(),
                    &s,
                    Color::TRANSPARENT,
                );
            }
        }
    }
}

/// Generate a star polygon with the given number of points.
fn make_star(cx: f64, cy: f64, rx: f64, ry: f64, points: usize) -> Vec<(f64, f64)> {
    let mut verts = Vec::with_capacity(points * 2);
    for i in 0..(points * 2) {
        let a = PI * i as f64 / points as f64 - PI / 2.0;
        let r = if i % 2 == 0 { 1.0 } else { 0.4 };
        verts.push((cx + a.cos() * rx * r, cy + a.sin() * ry * r));
    }
    verts
}

fn main() {
    let app = App::new(Box::new(|app, event_loop| {
        let root = app.tree.create_root("root");
        app.tree.set_behavior(root, Box::new(PaintPanel::new()));
        app.tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);

        let close_sig = app.scheduler.create_signal();
        let flags_sig = app.scheduler.create_signal();
        let win = zuicchini::emCore::emWindow::ZuiWindow::create(
            event_loop,
            app.gpu(),
            root,
            WindowFlags::AUTO_DELETE,
            close_sig,
            flags_sig,
        );
        let wid = win.winit_window.id();
        app.windows.insert(wid, win);
        app.windows.get_mut(&wid).unwrap().view_mut().flags |= ViewFlags::ROOT_SAME_TALLNESS;
    }));
    app.run();
}
