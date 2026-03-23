use crate::emCore::emColor::emColor;
use crate::emCore::emImage::emImage;

use super::emPainter::{TextAlignment, VAlign};
use crate::emCore::emStroke::emStroke;
use super::emTexture::{ImageExtension, ImageQuality};

/// A recorded drawing operation for parallel tile rendering.
///
/// During the recording phase, a single-threaded tree walk captures all
/// draw operations into a `DrawList`. During the replay phase, multiple
/// threads independently replay the list into their own tile buffers.
pub(crate) enum DrawOp {
    // State management
    PushState,
    PopState,
    SetOffset(f64, f64),
    ClipRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    SetCanvasColor(emColor),
    SetAlpha(u8),

    // Shapes
    PaintRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: emColor,
        canvas_color: emColor,
    },
    PaintRoundRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        color: emColor,
    },
    PaintRectOutlined {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        stroke: emStroke,
        canvas_color: emColor,
    },
    PaintRoundRectOutlined {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        stroke: emStroke,
    },
    PaintEllipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        color: emColor,
        canvas_color: emColor,
    },
    PaintPolygon {
        vertices: Vec<(f64, f64)>,
        color: emColor,
        canvas_color: emColor,
    },
    PaintSolidPolyline {
        vertices: Vec<(f64, f64)>,
        stroke: emStroke,
        closed: bool,
        canvas_color: emColor,
    },

    // Images — raw pointers to images owned by panel behaviors.
    PaintImageFull {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        image_ptr: *const emImage,
        alpha: u8,
        canvas_color: emColor,
    },
    PaintImageColored {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        image_ptr: *const emImage,
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        color1: emColor,
        color2: emColor,
        canvas_color: emColor,
        extension: ImageExtension,
    },
    PaintImageScaled {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        image_ptr: *const emImage,
        quality: ImageQuality,
        extension: ImageExtension,
    },
    PaintBorderImage {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        l: f64,
        t: f64,
        r: f64,
        b: f64,
        image_ptr: *const emImage,
        src_l: i32,
        src_t: i32,
        src_r: i32,
        src_b: i32,
        alpha: u8,
        canvas_color: emColor,
        which_sub_rects: u16,
    },

    // Text
    PaintText {
        x: f64,
        y: f64,
        text: String,
        char_height: f64,
        width_scale: f64,
        color: emColor,
        canvas_color: emColor,
    },
    PaintTextBoxed {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        text: String,
        max_char_height: f64,
        color: emColor,
        canvas_color: emColor,
        box_h_align: TextAlignment,
        box_v_align: VAlign,
        text_alignment: TextAlignment,
        min_width_scale: f64,
        formatted: bool,
        rel_line_space: f64,
    },
}

// SAFETY: DrawOp contains *const emImage raw pointers to images owned by
// panel behaviors in the PanelTree. These pointers remain valid during
// the parallel replay phase because:
// 1. Images are owned by behaviors stored in PanelTree
// 2. The tree is not modified between recording and replay
// 3. std::thread::scope ensures all replay threads complete before returning
// All other fields are owned values (f64, emColor, String, Vec, emStroke).
unsafe impl Send for DrawOp {}
unsafe impl Sync for DrawOp {}

/// A list of recorded drawing operations for a frame.
pub(crate) struct DrawList {
    ops: Vec<DrawOp>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            ops: Vec::with_capacity(256),
        }
    }

    pub fn ops_mut(&mut self) -> &mut Vec<DrawOp> {
        &mut self.ops
    }

    /// Replay all recorded operations into the given painter.
    ///
    /// `tile_offset` is subtracted from all absolute `SetOffset` calls to
    /// convert viewport coordinates to tile-local coordinates.
    pub fn replay(&self, painter: &mut super::emPainter::emPainter, tile_offset: (f64, f64)) {
        // Initialize the painter's offset to account for the tile position.
        // Draw operations recorded at viewport coordinates are shifted so that
        // viewport pixels in the tile's region map to tile-local coordinates.
        painter.set_offset(-tile_offset.0, -tile_offset.1);

        for op in &self.ops {
            match op {
                DrawOp::PushState => painter.push_state(),
                DrawOp::PopState => painter.pop_state(),
                DrawOp::SetOffset(x, y) => {
                    painter.set_offset(x - tile_offset.0, y - tile_offset.1);
                }
                DrawOp::ClipRect { x, y, w, h } => painter.clip_rect(*x, *y, *w, *h),
                DrawOp::SetCanvasColor(c) => painter.set_canvas_color(*c),
                DrawOp::SetAlpha(a) => painter.set_alpha(*a),

                DrawOp::PaintRect {
                    x,
                    y,
                    w,
                    h,
                    color,
                    canvas_color,
                } => painter.paint_rect(*x, *y, *w, *h, *color, *canvas_color),

                DrawOp::PaintRoundRect {
                    x,
                    y,
                    w,
                    h,
                    radius,
                    color,
                } => painter.paint_round_rect(*x, *y, *w, *h, *radius, *color),

                DrawOp::PaintRectOutlined {
                    x,
                    y,
                    w,
                    h,
                    stroke,
                    canvas_color,
                } => painter.paint_rect_outlined(*x, *y, *w, *h, stroke, *canvas_color),

                DrawOp::PaintRoundRectOutlined {
                    x,
                    y,
                    w,
                    h,
                    radius,
                    stroke,
                } => painter.paint_round_rect_outlined(*x, *y, *w, *h, *radius, stroke),

                DrawOp::PaintEllipse {
                    cx,
                    cy,
                    rx,
                    ry,
                    color,
                    canvas_color,
                } => painter.paint_ellipse(*cx, *cy, *rx, *ry, *color, *canvas_color),

                DrawOp::PaintPolygon {
                    vertices,
                    color,
                    canvas_color,
                } => painter.paint_polygon(vertices, *color, *canvas_color),

                DrawOp::PaintSolidPolyline {
                    vertices,
                    stroke,
                    closed,
                    canvas_color,
                } => painter.paint_solid_polyline(vertices, stroke, *closed, *canvas_color),

                DrawOp::PaintImageFull {
                    x,
                    y,
                    w,
                    h,
                    image_ptr,
                    alpha,
                    canvas_color,
                } => {
                    // SAFETY: see DrawOp Send/Sync impl
                    let image = unsafe { &**image_ptr };
                    painter.paint_image_full(*x, *y, *w, *h, image, *alpha, *canvas_color);
                }

                DrawOp::PaintImageColored {
                    x,
                    y,
                    w,
                    h,
                    image_ptr,
                    src_x,
                    src_y,
                    src_w,
                    src_h,
                    color1,
                    color2,
                    canvas_color,
                    extension,
                } => {
                    let image = unsafe { &**image_ptr };
                    painter.paint_image_colored(
                        *x,
                        *y,
                        *w,
                        *h,
                        image,
                        *src_x,
                        *src_y,
                        *src_w,
                        *src_h,
                        *color1,
                        *color2,
                        *canvas_color,
                        *extension,
                    );
                }

                DrawOp::PaintImageScaled {
                    x,
                    y,
                    w,
                    h,
                    image_ptr,
                    quality,
                    extension,
                } => {
                    let image = unsafe { &**image_ptr };
                    painter.paint_image_scaled(*x, *y, *w, *h, image, *quality, *extension);
                }

                DrawOp::PaintBorderImage {
                    x,
                    y,
                    w,
                    h,
                    l,
                    t,
                    r,
                    b,
                    image_ptr,
                    src_l,
                    src_t,
                    src_r,
                    src_b,
                    alpha,
                    canvas_color,
                    which_sub_rects,
                } => {
                    let image = unsafe { &**image_ptr };
                    painter.paint_border_image(
                        *x,
                        *y,
                        *w,
                        *h,
                        *l,
                        *t,
                        *r,
                        *b,
                        image,
                        *src_l,
                        *src_t,
                        *src_r,
                        *src_b,
                        *alpha,
                        *canvas_color,
                        *which_sub_rects,
                    );
                }

                DrawOp::PaintText {
                    x,
                    y,
                    text,
                    char_height,
                    width_scale,
                    color,
                    canvas_color,
                } => painter.paint_text(
                    *x,
                    *y,
                    text,
                    *char_height,
                    *width_scale,
                    *color,
                    *canvas_color,
                ),

                DrawOp::PaintTextBoxed {
                    x,
                    y,
                    w,
                    h,
                    text,
                    max_char_height,
                    color,
                    canvas_color,
                    box_h_align,
                    box_v_align,
                    text_alignment,
                    min_width_scale,
                    formatted,
                    rel_line_space,
                } => painter.paint_text_boxed(
                    *x,
                    *y,
                    *w,
                    *h,
                    text,
                    *max_char_height,
                    *color,
                    *canvas_color,
                    *box_h_align,
                    *box_v_align,
                    *text_alignment,
                    *min_width_scale,
                    *formatted,
                    *rel_line_space,
                ),
            }
        }
    }
}
