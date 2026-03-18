#[allow(unused)]
pub mod scaled;

use std::f64::consts::PI;

use zuicchini::foundation::{Color, Image};
use zuicchini::panel::{PanelBehavior, PanelId, PanelState, PanelTree, View, ViewFlags};
use zuicchini::render::{
    ImageExtension, ImageQuality, LineCap, LineJoin, Painter, Stroke, StrokeEnd, StrokeEndType,
    Texture, TileCache, TILE_SIZE,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DEFAULT_VW: u32 = 1920;
pub const DEFAULT_VH: u32 = 1080;

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

pub struct Scenario {
    pub name: &'static str,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "Static",
        dx: 0.0,
        dy: 0.0,
        dz: 0.0,
    },
    Scenario {
        name: "Pan",
        dx: 5.0,
        dy: 0.0,
        dz: 0.0,
    },
    Scenario {
        name: "Zoom In",
        dx: 0.0,
        dy: 0.0,
        dz: 0.02,
    },
    Scenario {
        name: "Zoom Out",
        dx: 0.0,
        dy: 0.0,
        dz: -0.02,
    },
    Scenario {
        name: "Pan+Zoom",
        dx: 3.0,
        dy: 0.0,
        dz: 0.015,
    },
];

// ---------------------------------------------------------------------------
// TestPanel (verbatim from examples/bench_interaction.rs)
// ---------------------------------------------------------------------------

pub struct TestPanel {
    test_image: Image,
}

impl TestPanel {
    pub fn new() -> Self {
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

impl PanelBehavior for TestPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        if state.viewed_rect.w < 25.0 {
            return;
        }

        painter.push_state();
        painter.scale(w, w);
        let h = h / w;

        let fg = Color::grey(136);
        let bg = Color::rgba(0x00, 0x1C, 0x38, 0xFF);

        painter.paint_rect(0.0, 0.0, 1.0, h, bg, Color::TRANSPARENT);
        painter.paint_rect_outlined(
            0.01,
            0.01,
            1.0 - 0.02,
            h - 0.02,
            &Stroke::new(fg, 0.02),
            Color::TRANSPARENT,
        );

        let _state_str = format!(
            "State: InFocusedPath ViewFocused Pri={:.3} MemLim={}",
            state.priority, state.memory_limit,
        );
        painter.paint_rect(
            0.25,
            0.8,
            0.05,
            0.05,
            Color::rgba(255, 0, 0, 32),
            Color::TRANSPARENT,
        );

        painter.paint_polygon(
            &[(0.7, 0.6), (0.6, 0.7), (0.8, 0.8)],
            fg,
            Color::TRANSPARENT,
        );
        painter.paint_polygon_even_odd(
            &[
                (0.90, 0.90),
                (0.94, 0.90),
                (0.94, 0.94),
                (0.90, 0.94),
                (0.90, 0.90),
                (0.91, 0.91),
                (0.93, 0.91),
                (0.93, 0.93),
                (0.91, 0.93),
                (0.91, 0.91),
            ],
            Color::rgba(255, 255, 255, 128),
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[
                (0.80, 0.90),
                (0.84, 0.90),
                (0.84, 0.94),
                (0.80, 0.94),
                (0.80, 0.90),
                (0.81, 0.91),
                (0.81, 0.93),
                (0.83, 0.93),
                (0.83, 0.91),
                (0.81, 0.91),
            ],
            Color::WHITE,
            Color::TRANSPARENT,
        );

        let circle: Vec<_> = (0..64)
            .map(|i| {
                let a = PI * i as f64 / 32.0;
                (a.sin() * 0.05 + 0.65, a.cos() * 0.05 + 0.85)
            })
            .collect();
        painter.paint_polygon(&circle, Color::rgba(255, 255, 0, 255), Color::TRANSPARENT);

        let clipped: Vec<_> = (0..64)
            .map(|i| {
                let a = PI * i as f64 / 32.0;
                (a.sin() * 0.05 + 0.55, a.cos() * 0.05 + 0.85)
            })
            .collect();
        painter.push_state();
        painter.clip_rect(0.51, 0.81, 0.08, 0.08);
        painter.paint_polygon(&clipped, Color::rgba(0, 255, 0, 255), Color::TRANSPARENT);
        painter.pop_state();

        let ellipse: Vec<_> = (0..64)
            .map(|i| {
                let a = PI * i as f64 / 32.0;
                (a.sin() * 0.06 + 0.6, a.cos() * 0.04 + 0.86)
            })
            .collect();
        painter.paint_polygon(&ellipse, Color::rgba(255, 0, 0, 92), Color::TRANSPARENT);

        painter.paint_polygon(
            &[(0.6, 0.9), (0.5, 0.92), (0.65, 0.95)],
            Color::rgba(187, 255, 255, 255),
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[(0.6, 0.96), (0.5, 0.92), (0.65, 0.95)],
            Color::RED,
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[(0.45, 0.9), (0.35, 0.92), (0.5, 0.95)],
            Color::rgba(187, 255, 255, 255),
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[(0.45, 0.96), (0.35, 0.92), (0.5, 0.95)],
            Color::RED,
            Color::TRANSPARENT,
        );

        painter.paint_polygon(
            &[(0.6, 0.6), (0.602, 0.6), (0.502, 0.7)],
            Color::rgba(187, 136, 255, 192),
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[(0.7, 0.55), (0.702, 0.55), (0.802, 0.9), (0.8, 0.9)],
            Color::rgba(136, 187, 255, 192),
            Color::TRANSPARENT,
        );
        painter.paint_polygon(
            &[(0.8, 0.55), (0.9, 0.55), (0.8, 0.8), (0.9, 0.8)],
            Color::rgba(136, 187, 255, 192),
            Color::TRANSPARENT,
        );

        painter.paint_ellipse(0.055, 0.805, 0.005, 0.005, Color::WHITE, Color::TRANSPARENT);
        painter.paint_ellipse(0.07, 0.805, 0.01, 0.005, Color::WHITE, Color::TRANSPARENT);
        painter.paint_ellipse(
            0.0925,
            0.805,
            0.0025,
            0.005,
            Color::WHITE,
            Color::TRANSPARENT,
        );

        let deg = PI / 180.0;
        painter.paint_ellipse_sector(
            0.105, 0.805, 0.005, 0.005, 45.0, 305.0, Color::WHITE, Color::TRANSPARENT,
        );
        painter.paint_ellipse_sector(
            0.12, 0.805, 0.01, 0.005, -350.0, 395.0, Color::WHITE, Color::TRANSPARENT,
        );
        painter.paint_ellipse_sector(
            0.1325, 0.805, 0.0025, 0.005, 245.0, 50.0, Color::WHITE, Color::TRANSPARENT,
        );
        painter.paint_ellipse_sector(
            0.145, 0.805, 0.005, 0.005, 195.0, 50.0, Color::WHITE, Color::TRANSPARENT,
        );

        painter.paint_rect_outlined(
            0.05,
            0.82,
            0.01,
            0.01,
            &Stroke::new(Color::WHITE, 0.001),
            Color::TRANSPARENT,
        );
        let mut sd = Stroke::new(Color::WHITE, 0.001);
        sd.dash_pattern = vec![0.002, 0.001];
        painter.paint_rect_outlined(0.07, 0.82, 0.02, 0.01, &sd, Color::TRANSPARENT);
        painter.paint_rect_outlined(
            0.10,
            0.82,
            0.01,
            0.01,
            &Stroke::new(Color::WHITE, 0.008),
            Color::TRANSPARENT,
        );
        painter.paint_rect_outlined(
            0.13,
            0.82,
            0.01,
            0.01,
            &Stroke::new(Color::WHITE, 0.011),
            Color::TRANSPARENT,
        );

        painter.paint_round_rect(0.05, 0.84, 0.01, 0.01, 0.001, Color::WHITE);
        painter.paint_round_rect(0.07, 0.84, 0.02, 0.01, 0.002, Color::WHITE);
        painter.paint_round_rect(0.10, 0.84, 0.01, 0.01, 0.003, Color::WHITE);
        painter.paint_round_rect(0.13, 0.84, 0.01, 0.01, 0.006, Color::WHITE);
        painter.paint_round_rect(0.15, 0.84, 0.01, 0.01, 0.0, Color::WHITE);

        painter.paint_ellipse_outlined(
            0.055,
            0.865,
            0.005,
            0.005,
            &Stroke::new(Color::WHITE, 0.003),
            Color::TRANSPARENT,
        );
        painter.paint_ellipse_outlined(
            0.075,
            0.865,
            0.01,
            0.005,
            &Stroke::new(Color::WHITE, 0.001),
            Color::TRANSPARENT,
        );
        let mut dot_s = Stroke::new(Color::WHITE, 0.00025);
        dot_s.join = LineJoin::Round;
        dot_s.cap = LineCap::Round;
        dot_s.dash_pattern = vec![0.0001, 0.0005];
        painter.paint_ellipse_outlined(0.0925, 0.865, 0.0025, 0.005, &dot_s, Color::TRANSPARENT);

        painter.paint_ellipse_arc(
            0.105,
            0.865,
            0.005,
            0.005,
            90.0 * deg,
            225.0 * deg,
            &Stroke::new(Color::WHITE, 0.001),
            Color::TRANSPARENT,
        );
        painter.paint_ellipse_sector_outlined(
            0.12,
            0.865,
            0.01,
            0.005,
            45.0,
            -365.0,
            &Stroke::new(Color::WHITE, 0.0001),
            Color::TRANSPARENT,
        );
        painter.paint_ellipse_arc(
            0.1325,
            0.865,
            0.0025,
            0.005,
            245.0 * deg,
            295.0 * deg,
            &Stroke::new(Color::WHITE, 0.001),
            Color::TRANSPARENT,
        );
        painter.paint_ellipse_arc(
            0.145,
            0.865,
            0.005,
            0.005,
            195.0 * deg,
            245.0 * deg,
            &Stroke::new(Color::WHITE, 0.001),
            Color::TRANSPARENT,
        );
        let mut rs = Stroke::new(Color::WHITE, 0.0001);
        rs.join = LineJoin::Round;
        rs.cap = LineCap::Round;
        rs.start_end = StrokeEnd::new(StrokeEndType::Cap);
        rs.finish_end = StrokeEnd::new(StrokeEndType::LineArrow);
        painter.paint_ellipse_arc(
            0.155,
            0.865,
            0.005,
            0.005,
            0.0,
            -145.0 * deg,
            &rs,
            Color::TRANSPARENT,
        );

        painter.paint_round_rect_outlined(
            0.05,
            0.88,
            0.01,
            0.01,
            0.001,
            &Stroke::new(Color::WHITE, 0.001),
        );
        painter.paint_round_rect_outlined(
            0.07,
            0.88,
            0.02,
            0.01,
            0.002,
            &Stroke::new(Color::WHITE, 0.001),
        );
        painter.paint_round_rect_outlined(
            0.10,
            0.88,
            0.01,
            0.01,
            0.003,
            &Stroke::new(Color::WHITE, 0.003),
        );
        painter.paint_round_rect_outlined(
            0.12,
            0.88,
            0.01,
            0.01,
            0.006,
            &Stroke::new(Color::WHITE, 0.0001),
        );
        let mut dds = Stroke::new(Color::WHITE, 0.00002);
        dds.dash_pattern = vec![0.0001, 0.00005, 0.00003, 0.00005];
        painter.paint_round_rect_outlined(0.135, 0.88, 0.01, 0.01, 0.001, &dds);
        painter.paint_round_rect_outlined(
            0.15,
            0.88,
            0.01,
            0.01,
            0.0,
            &Stroke::new(Color::WHITE, 0.001),
        );

        painter.paint_bezier(
            &[(0.05, 0.90), (0.06, 0.90), (0.05, 0.91)],
            Color::WHITE,
            Color::TRANSPARENT,
        );
        painter.paint_bezier(
            &[
                (0.065, 0.91),
                (0.05, 0.902),
                (0.058, 0.89),
                (0.065, 0.900),
                (0.072, 0.89),
                (0.08, 0.902),
            ],
            Color::WHITE,
            Color::TRANSPARENT,
        );
        let mut rd = Stroke::new(Color::WHITE, 0.0002);
        rd.join = LineJoin::Round;
        rd.cap = LineCap::Round;
        rd.dash_pattern = vec![0.001, 0.0005];
        painter.paint_bezier_outline(
            &[
                (0.085, 0.91),
                (0.07, 0.902),
                (0.078, 0.89),
                (0.085, 0.900),
                (0.092, 0.89),
                (0.10, 0.902),
            ],
            &rd,
            Color::TRANSPARENT,
        );
        let mut bls = Stroke::new(Color::WHITE, 0.0002);
        bls.join = LineJoin::Round;
        bls.cap = LineCap::Round;
        bls.dash_pattern = vec![0.001, 0.0005];
        bls.start_end = StrokeEnd::new(StrokeEndType::ContourTriangle).with_inner_color(Color::RED);
        bls.finish_end = StrokeEnd::new(StrokeEndType::Arrow);
        painter.paint_bezier_line(
            &[(0.105, 0.91), (0.09, 0.902), (0.098, 0.89), (0.105, 0.900)],
            &bls,
            Color::TRANSPARENT,
        );

        let n = 17usize;
        for i in 0..(2 * n) {
            let a = 2.0 * PI * i as f64 / (2 * n) as f64;
            let mut ls = Stroke::new(Color::WHITE, 0.0001);
            if i & 1 != 0 {
                ls.join = LineJoin::Round;
                ls.cap = LineCap::Round;
            }
            ls.start_end = StrokeEnd::new(StrokeEndType::Cap);
            let end_type = match i / 2 {
                0 => StrokeEndType::Butt,
                1 => StrokeEndType::Cap,
                2 => StrokeEndType::Arrow,
                3 => StrokeEndType::ContourArrow,
                4 => StrokeEndType::LineArrow,
                5 => StrokeEndType::Triangle,
                6 => StrokeEndType::ContourTriangle,
                7 => StrokeEndType::Square,
                8 => StrokeEndType::ContourSquare,
                9 => StrokeEndType::HalfSquare,
                10 => StrokeEndType::Circle,
                11 => StrokeEndType::ContourCircle,
                12 => StrokeEndType::HalfCircle,
                13 => StrokeEndType::Diamond,
                14 => StrokeEndType::ContourDiamond,
                15 => StrokeEndType::HalfDiamond,
                _ => StrokeEndType::Stroke,
            };
            ls.finish_end =
                StrokeEnd::new(end_type).with_inner_color(Color::rgba(0xFF, 0xFF, 0xFF, 0x40));
            painter.paint_line_stroked(
                0.117 + 0.002 * a.cos(),
                0.903 + 0.002 * a.sin(),
                0.117 + 0.0075 * a.cos(),
                0.903 + 0.0075 * a.sin(),
                &ls,
                Color::TRANSPARENT,
            );
        }

        let mut ps = Stroke::new(Color::WHITE, 0.0005);
        ps.join = LineJoin::Round;
        ps.cap = LineCap::Round;
        ps.start_end =
            StrokeEnd::new(StrokeEndType::ContourArrow).with_inner_color(Color::TRANSPARENT);
        ps.finish_end = StrokeEnd::new(StrokeEndType::Cap);
        painter.paint_solid_polyline(
            &[(0.13, 0.897), (0.14, 0.902), (0.13, 0.906), (0.137, 0.909)],
            &ps,
            false,
            Color::TRANSPARENT,
        );

        painter.paint_polygon_outlined(
            &[(0.06, 0.80), (0.10, 0.85), (0.08, 0.91)],
            Color::RED,
            0.0002,
            Color::TRANSPARENT,
        );

        let star = |ox: f64| -> Vec<(f64, f64)> {
            vec![
                (ox, 0.905),
                (ox + 0.015, 0.912),
                (ox + 0.030, 0.900),
                (ox + 0.022, 0.915),
                (ox + 0.030, 0.930),
                (ox + 0.020, 0.922),
                (ox + 0.005, 0.935),
                (ox + 0.012, 0.920),
            ]
        };
        painter.paint_polygon_textured(
            &star(0.200),
            &Texture::LinearGradient {
                color_a: Color::rgba(0, 255, 0, 128),
                color_b: Color::rgba(255, 255, 0, 255),
                start: (0.23, 0.9),
                end: (0.2, 0.93),
            },
            Color::TRANSPARENT,
        );
        painter.paint_polygon_textured(
            &star(0.220),
            &Texture::RadialGradient {
                color_inner: Color::rgba(0xCC, 0xCC, 0x33, 0xFF),
                color_outer: Color::rgba(0, 0, 0xFF, 0x60),
                center: (0.235, 0.918),
                radius: 0.04,
            },
            Color::TRANSPARENT,
        );
        painter.paint_polygon_textured(
            &star(0.240),
            &Texture::Image {
                image: self.test_image.clone(),
                extension: ImageExtension::Clamp,
                quality: ImageQuality::Bilinear,
            },
            Color::TRANSPARENT,
        );

        painter.paint_linear_gradient(
            0.2,
            0.94,
            0.02,
            0.01,
            Color::rgba(0, 0, 0, 128),
            Color::rgba(128, 128, 128, 128),
            true,
            Color::TRANSPARENT,
        );
        painter.paint_radial_gradient(
            0.225,
            0.945,
            0.004,
            0.005,
            Color::rgba(255, 136, 0, 255),
            Color::rgba(0, 85, 0, 255),
            Color::TRANSPARENT,
        );

        let eg: Vec<_> = (0..64)
            .map(|i| {
                let a = 2.0 * PI * i as f64 / 64.0;
                (0.24 + 0.01 * a.cos(), 0.945 + 0.005 * a.sin())
            })
            .collect();
        painter.paint_polygon_textured(
            &eg,
            &Texture::RadialGradient {
                color_inner: Color::TRANSPARENT,
                color_outer: Color::rgba(0, 204, 136, 255),
                center: (0.24, 0.945),
                radius: 0.01,
            },
            Color::TRANSPARENT,
        );

        painter.paint_image_scaled(
            0.26,
            0.94,
            0.02,
            0.01,
            &self.test_image,
            ImageQuality::Bilinear,
            ImageExtension::Clamp,
        );
        painter.paint_image_scaled(
            0.275,
            0.907,
            0.002,
            0.002,
            &self.test_image,
            ImageQuality::Bilinear,
            ImageExtension::Repeat,
        );
        painter.paint_image_scaled(
            0.275,
            0.910,
            0.002,
            0.002,
            &self.test_image,
            ImageQuality::Bilinear,
            ImageExtension::Clamp,
        );
        painter.paint_image_scaled(
            0.275,
            0.913,
            0.002,
            0.002,
            &self.test_image,
            ImageQuality::Bilinear,
            ImageExtension::Zero,
        );

        painter.pop_state();
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

pub fn setup_tree_and_view(vw: u32, vh: u32) -> (PanelTree, View, PanelId) {
    let mut tree = PanelTree::new();
    let root = tree.create_root("bench_root");
    tree.set_behavior(root, Box::new(TestPanel::new()));
    let tallness = vh as f64 / vw as f64;
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, tallness);
    tree.set_focusable(root, true);

    let mut view = View::new(root, vw as f64, vh as f64);
    view.flags |= ViewFlags::ROOT_SAME_TALLNESS;
    tree.deliver_notices(true, 1.0);
    view.update(&mut tree);

    (tree, view, root)
}

/// Execute one complete frame cycle without timing instrumentation.
pub fn run_one_frame(
    tree: &mut PanelTree,
    view: &mut View,
    viewport_buf: &mut Image,
    tile_cache: &mut TileCache,
    scenario: &Scenario,
    fix_x: f64,
    fix_y: f64,
) {
    let (cols, rows) = tile_cache.grid_size();

    // 1. Scroll/zoom
    view.raw_scroll_and_zoom(tree, fix_x, fix_y, scenario.dx, scenario.dy, scenario.dz);

    // 2. Notices
    tree.deliver_notices(true, 1.0);

    // 3. View update
    view.update(tree);

    // 4. Paint
    viewport_buf.fill(Color::BLACK);
    {
        let mut painter = Painter::new(viewport_buf);
        view.paint(tree, &mut painter);
    }

    // 5. Tile copy
    for row in 0..rows {
        for col in 0..cols {
            let tile = tile_cache.get_or_create(col, row);
            tile.image
                .copy_from_rect(0, 0, viewport_buf, (col * TILE_SIZE, row * TILE_SIZE, TILE_SIZE, TILE_SIZE));
        }
    }

    // 6. Frame cleanup
    view.clear_viewport_changed();
}
