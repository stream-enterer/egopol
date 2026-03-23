use zuicchini::emCore::emColor::emColor;
use zuicchini::emCore::emImage::emImage;
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emStroke::emStroke;

#[test]
fn paint_rect_fills_correct_pixels() {
    let mut img = emImage::new(10, 10, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.PaintRect(2.0, 3.0, 4.0, 2.0, emColor::RED, emColor::TRANSPARENT);
    }
    // Pixel inside the rect
    assert_eq!(img.GetPixel(3, 3), &[255, 0, 0, 255]);
    assert_eq!(img.GetPixel(5, 4), &[255, 0, 0, 255]);
    // Pixel outside the rect should be canvas GetColor
    assert_eq!(img.GetPixel(0, 0), &[0, 0, 0, 255]);
    assert_eq!(img.GetPixel(7, 7), &[0, 0, 0, 255]);
}

#[test]
fn canvas_blend_works_in_painter() {
    let mut img = emImage::new(4, 4, 4);
    img.fill(emColor::rgb(100, 100, 100));

    {
        let mut p = emPainter::new(&mut img);
        // Canvas = rgb(50,50,50), source = rgb(150,150,150), alpha = 128
        // target += (150 - 50) * 128 / 255 = target + 50 = 150
        p.SetAlpha(128);
        p.PaintRect(
            0.0,
            0.0,
            4.0,
            4.0,
            emColor::rgb(150, 150, 150),
            emColor::rgb(50, 50, 50),
        );
    }
    let px = img.GetPixel(0, 0);
    assert_eq!(px[0], 150);
    assert_eq!(px[1], 150);
    assert_eq!(px[2], 150);
}

#[test]
fn clip_rect_respected() {
    let mut img = emImage::new(10, 10, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.SetClipping(2.0, 2.0, 4.0, 4.0);
        // Paint a rect that extends beyond the clip
        p.PaintRect(0.0, 0.0, 10.0, 10.0, emColor::GREEN, emColor::TRANSPARENT);
    }
    // Inside clip: should be painted
    assert_eq!(img.GetPixel(3, 3), &[0, 255, 0, 255]);
    // Outside clip: should be canvas GetColor (untouched)
    assert_eq!(img.GetPixel(0, 0), &[0, 0, 0, 255]);
    assert_eq!(img.GetPixel(7, 7), &[0, 0, 0, 255]);
}

#[test]
fn coordinate_transforms() {
    let mut img = emImage::new(20, 20, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.translate(5.0, 5.0);
        p.PaintRect(0.0, 0.0, 2.0, 2.0, emColor::BLUE, emColor::TRANSPARENT);
    }
    // Translated rect should appear at (5,5)
    assert_eq!(img.GetPixel(5, 5), &[0, 0, 255, 255]);
    assert_eq!(img.GetPixel(6, 6), &[0, 0, 255, 255]);
    // Origin should be canvas GetColor
    assert_eq!(img.GetPixel(0, 0), &[0, 0, 0, 255]);
}

#[test]
fn push_pop_state() {
    let mut img = emImage::new(20, 20, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.push_state();
        p.translate(10.0, 10.0);
        p.PaintRect(0.0, 0.0, 2.0, 2.0, emColor::RED, emColor::TRANSPARENT);
        p.pop_state();
        // After pop, translation is restored
        p.PaintRect(0.0, 0.0, 2.0, 2.0, emColor::GREEN, emColor::TRANSPARENT);
    }
    // Red at translated GetPos
    assert_eq!(img.GetPixel(10, 10), &[255, 0, 0, 255]);
    // Green at origin (painted after pop) — canvas blended on top of red remnant
    assert_eq!(img.GetPixel(0, 0)[1], 255); // green channel
}

#[test]
fn paint_ellipse_basic() {
    let mut img = emImage::new(20, 20, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.PaintEllipse(10.0, 10.0, 5.0, 5.0, emColor::RED, emColor::TRANSPARENT);
    }
    // Center should be filled
    let px = img.GetPixel(10, 10);
    assert_eq!(px[0], 255); // red
                            // Far corner should be canvas GetColor
    assert_eq!(img.GetPixel(0, 0), &[0, 0, 0, 255]);
}

#[test]
fn paint_line_basic() {
    let mut img = emImage::new(10, 10, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        p.PaintLine(0.0, 0.0, 9.0, 0.0, emColor::WHITE, emColor::TRANSPARENT);
    }
    // Horizontal line at y=0
    assert_eq!(img.GetPixel(0, 0), &[255, 255, 255, 255]);
    assert_eq!(img.GetPixel(5, 0), &[255, 255, 255, 255]);
    // Below the line should be canvas GetColor
    assert_eq!(img.GetPixel(0, 5), &[0, 0, 0, 255]);
}

#[test]
fn PaintRectOutline() {
    let mut img = emImage::new(20, 20, 4);
    img.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut img);
        p.SetCanvasColor(emColor::BLACK);
        let stroke = emStroke::new(emColor::WHITE, 2.0);
        p.PaintRectOutline(5.0, 5.0, 10.0, 10.0, &stroke, emColor::TRANSPARENT);
    }
    // Top edge interior pixel (fully inside the stroke ring).
    // emStroke centered on boundary: outer=(4,4), inner=(6,6).
    // Pixel (8,5) is fully within the top stroke band.
    assert_eq!(img.GetPixel(8, 5), &[255, 255, 255, 255]);
    // Center should be canvas GetColor (only outline)
    assert_eq!(img.GetPixel(10, 10), &[0, 0, 0, 255]);
}

#[test]
fn paint_image_colored_basic() {
    // Create a 2x2 greyscale GetImage
    let mut alpha_img = emImage::new(2, 2, 1);
    alpha_img.SetPixel(0, 0)[0] = 255;
    alpha_img.SetPixel(1, 0)[0] = 128;
    alpha_img.SetPixel(0, 1)[0] = 0;
    alpha_img.SetPixel(1, 1)[0] = 64;

    let mut target = emImage::new(4, 4, 4);
    target.fill(emColor::BLACK);

    {
        let mut p = emPainter::new(&mut target);
        p.SetCanvasColor(emColor::BLACK);
        p.PaintImageColored(
            0.0,
            0.0,
            2.0,
            2.0,
            &alpha_img,
            0,
            0,
            2,
            2,
            emColor::TRANSPARENT,
            emColor::RED,
            emColor::TRANSPARENT,
            zuicchini::emCore::emTexture::ImageExtension::EdgeOrZero,
        );
    }
    // Top-left pixel: full red (alpha=255 from mask)
    let px = target.GetPixel(0, 0);
    assert_eq!(px[0], 255); // red
                            // Bottom-left pixel: no PaintContent (alpha=0 from mask)
    let px2 = target.GetPixel(0, 1);
    assert_eq!(px2[0], 0); // still black
}
