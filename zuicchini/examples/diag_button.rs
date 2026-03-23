use zuicchini::emCore::emLook::emLook;

fn main() {
    // Load several goldens and map their layouts
    let tests = vec![
        "widget_checkbox_unchecked",
        "widget_button_normal",
        "widget_border_instrument",
    ];

    for name in &tests {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("golden/compositor/{}.compositor.golden", name));
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                println!("=== {} MISSING: {} ===\n", name, e);
                continue;
            }
        };
        let expected = &data[8..];
        let golden_w = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let golden_h = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        println!("=== {} ({}x{}) ===", name, golden_w, golden_h);

        // Sample vertical line at x=400 — find color regions
        println!("  Vertical x=400:");
        let x = 400;
        let mut prev = (0u8, 0u8, 0u8);
        let mut run_start = 0;
        for y in 0..golden_h {
            let idx = (y * golden_w + x) * 4;
            let cur = (expected[idx], expected[idx + 1], expected[idx + 2]);
            if (cur.0 as i32 - prev.0 as i32).abs() > 3
                || (cur.1 as i32 - prev.1 as i32).abs() > 3
                || (cur.2 as i32 - prev.2 as i32).abs() > 3
            {
                if y > 0 {
                    println!(
                        "    y={:3}-{:3}: ({:3},{:3},{:3}) [{} rows]",
                        run_start,
                        y - 1,
                        prev.0,
                        prev.1,
                        prev.2,
                        y - run_start
                    );
                }
                run_start = y;
            }
            prev = cur;
        }
        println!(
            "    y={:3}-{:3}: ({:3},{:3},{:3}) [{} rows]",
            run_start,
            golden_h - 1,
            prev.0,
            prev.1,
            prev.2,
            golden_h - run_start
        );

        // Sample horizontal line at y=golden_h/2 — find color regions
        let y = golden_h / 2;
        println!("  Horizontal y={}:", y);
        prev = (0, 0, 0);
        run_start = 0;
        for x in 0..golden_w {
            let idx = (y * golden_w + x) * 4;
            let cur = (expected[idx], expected[idx + 1], expected[idx + 2]);
            if (cur.0 as i32 - prev.0 as i32).abs() > 3
                || (cur.1 as i32 - prev.1 as i32).abs() > 3
                || (cur.2 as i32 - prev.2 as i32).abs() > 3
            {
                if x > 0 {
                    println!(
                        "    x={:3}-{:3}: ({:3},{:3},{:3}) [{} px]",
                        run_start,
                        x - 1,
                        prev.0,
                        prev.1,
                        prev.2,
                        x - run_start
                    );
                }
                run_start = x;
            }
            prev = cur;
        }
        println!(
            "    x={:3}-{:3}: ({:3},{:3},{:3}) [{} px]",
            run_start,
            golden_w - 1,
            prev.0,
            prev.1,
            prev.2,
            golden_w - run_start
        );

        // Find unique dominant colors
        let mut colors: std::collections::BTreeMap<(u8, u8, u8), u32> = Default::default();
        for y in 0..golden_h {
            for x in 0..golden_w {
                let idx = (y * golden_w + x) * 4;
                let key = (expected[idx], expected[idx + 1], expected[idx + 2]);
                *colors.entry(key).or_default() += 1;
            }
        }
        println!("  Dominant colors:");
        let mut sorted: Vec<_> = colors.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for &((r, g, b), count) in sorted.iter().take(10) {
            let pct = count as f64 / (golden_w * golden_h) as f64 * 100.0;
            println!("    ({:3},{:3},{:3}): {:6} ({:.1}%)", r, g, b, count, pct);
        }

        println!();
    }

    // Also print our known Rust emLook colors for comparison
    let look = emLook::new();
    println!("Rust Look colors:");
    println!(
        "  bg_color:        ({},{},{},{})",
        look.bg_color.GetRed(),
        look.bg_color.GetGreen(),
        look.bg_color.GetBlue(),
        look.bg_color.GetAlpha()
    );
    println!(
        "  fg_color:        ({},{},{},{})",
        look.fg_color.GetRed(),
        look.fg_color.GetGreen(),
        look.fg_color.GetBlue(),
        look.fg_color.GetAlpha()
    );
    println!(
        "  button_bg_color: ({},{},{},{})",
        look.button_bg_color.GetRed(),
        look.button_bg_color.GetGreen(),
        look.button_bg_color.GetBlue(),
        look.button_bg_color.GetAlpha()
    );
    println!(
        "  button_fg_color: ({},{},{},{})",
        look.button_fg_color.GetRed(),
        look.button_fg_color.GetGreen(),
        look.button_fg_color.GetBlue(),
        look.button_fg_color.GetAlpha()
    );
}
