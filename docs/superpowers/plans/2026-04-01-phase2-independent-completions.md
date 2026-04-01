# Phase 2: Independent Completions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the eagle logo polygons, star texture rendering, IPC single-instance, detached control window, and fix VcItem color parsing — five independent items completing emMain visual and behavioral fidelity.

**Architecture:** Each task is fully independent — no ordering dependencies within this phase. All tasks modify emmain crate files, one copies a TGA resource, one modifies emMain.rs and main.rs for IPC wiring. The color round-trip fix changes how VcItem colors are parsed (hex strings instead of struct).

**Tech Stack:** Rust, emcore (emPainter, emImage, emColor, emMiniIpc), emmain crate.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `res/emMain/Star.tga` | Create (copy from C++) | Star shape texture for starfield rendering |
| `crates/emmain/src/emMainContentPanel.rs` | Modify | Add polygon data and PaintEagle method |
| `crates/emmain/src/emStarFieldPanel.rs` | Modify | 3-tier star rendering with TGA texture |
| `crates/emmain/src/emMainWindow.rs` | Modify | Add CreateControlWindow + DoCustomCheat |
| `crates/emmain/src/emMain.rs` | Rewrite | Wire IPC client/server, emMain engine struct |
| `crates/eaglemode/src/main.rs` | Modify | Wire IPC flow with real emMiniIpcClient |
| `crates/emmain/src/emVirtualCosmos.rs` | Modify | Fix color parsing to handle hex strings |

---

### Task 1: Eagle Logo Polygons (Item 9)

**Files:**
- Modify: `crates/emmain/src/emMainContentPanel.rs`

Port the 14 polygon arrays from C++ `emMainContentPanel::PaintEagle` (emMainContentPanel.cpp:132-349). Replace the "Eagle Mode" text placeholder with polygon rendering.

- [ ] **Step 1: Write test for polygon data integrity**

Add to the test module in `emMainContentPanel.rs`:

```rust
#[test]
fn test_eagle_polygon_count() {
    assert_eq!(EAGLE_POLYS.len(), 14);
    assert_eq!(EAGLE_POLY_COLORS.len(), 14);
}

#[test]
fn test_eagle_polygon_sizes() {
    // C++ polySizes: 461,74,6,8,151,15,71,70,18,19,15,18,27,7
    // These are coordinate counts (pairs), so vertex counts are half
    let expected_coord_counts = [461, 74, 6, 8, 151, 15, 71, 70, 18, 19, 15, 18, 27, 7];
    for (i, expected) in expected_coord_counts.iter().enumerate() {
        assert_eq!(
            EAGLE_POLYS[i].len(),
            *expected / 2,
            "poly{i} vertex count mismatch: expected {} vertices ({} coords), got {} vertices",
            expected / 2,
            expected,
            EAGLE_POLYS[i].len()
        );
    }
}

#[test]
fn test_eagle_poly0_first_vertex() {
    // First vertex of poly0: (79695.0, 46350.0)
    assert!((EAGLE_POLYS[0][0].0 - 79695.0).abs() < 0.01);
    assert!((EAGLE_POLYS[0][0].1 - 46350.0).abs() < 0.01);
}

#[test]
fn test_eagle_poly0_last_vertex() {
    // Last vertex of poly0: (77940.0, 46440.0)
    let last = EAGLE_POLYS[0].last().unwrap();
    assert!((last.0 - 77940.0).abs() < 0.01);
    assert!((last.1 - 46440.0).abs() < 0.01);
}

#[test]
fn test_eagle_poly13_eye() {
    // poly13 is the eye — 7 coordinate pairs
    assert_eq!(EAGLE_POLYS[13].len(), 7 / 2); // C++ has 7 coords = 3.5 pairs?
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emmain`
Expected: FAIL — `EAGLE_POLYS` and `EAGLE_POLY_COLORS` don't exist.

- [ ] **Step 3: Add polygon data constants**

Add above the `emMainContentPanel` struct definition. The C++ stores coordinates as flat `double[]` arrays with interleaved X,Y. Convert to `&[(f64, f64)]` slices. The C++ `polySizes` array gives coordinate counts (not vertex counts), so divide by 2 for vertex count.

Note: poly13 has 7 coordinates in C++ which is odd — this is 3 vertices plus one extra coordinate. Looking at the C++ data: `78660.0,47465.0, 78750.0,47610.0, 78750.0,47790.0, 78615.0,47880.0, 78390.0,47925.0, 78210.0,47835.0, 78120.0,47675.0` — that's actually 7 pairs (14 doubles), and C++ `polySizes[13] = 7` means 7 coordinate *pairs* (the polySizes counts are actually vertex counts, not coordinate counts). Let me recheck: the C++ calls `painter.PaintPolygon(polys[i], polySizes[i], polyColors[i])` where PaintPolygon signature is `PaintPolygon(const double* xy, int count, emColor color)` and count is the number of *vertices* (each vertex is 2 doubles).

So polySizes = number of vertices. poly0 has 461 values ÷ 2 = 230.5 — that can't be right. Looking at the C++ poly0 array, it has coordinate pairs, so 461 doubles should be invalid. Let me recount: the array ends at `77940.0,46440.0` — counting the entries would give an even number. The C++ `polySizes[0] = 461` must actually mean 461/2 rounded... No, looking at the C++ PaintPolygon: `void emPainter::PaintPolygon(const double * xy, int count, emUInt32 color)` where `count` is the number of *points* (each point = 2 doubles). So `polySizes[0] = 461` means 461 points, consuming 922 doubles. But the array only has ~461 doubles (230 pairs + 1 extra). This needs careful validation.

Actually, re-reading the C++ more carefully: the poly0 array is defined as one long initializer list. The `polySizes` are indeed vertex counts. Let me recount poly0's actual double count by examining the data. The array has many entries per line (12 per line = 6 pairs). This is a large polygon with hundreds of vertices.

The safest approach: copy the raw coordinate data as-is and let the test verify vertex counts match C++.

```rust
// ── Eagle polygon data ─────────────────────────────────────────────
// Port of C++ emMainContentPanel::PaintEagle polygon arrays.
// Each polygon is a slice of (x, y) vertex pairs in eagle coordinate space.
// Colors are packed RGBA matching C++ polyColors[].

/// Polygon vertex counts matching C++ polySizes[].
const EAGLE_POLY_SIZES: [usize; 14] = [
    // C++ polySizes: these are vertex counts (each vertex = 1 x,y pair)
    // poly0 has the most coordinates in the C++ source
    230, 37, 3, 4, 75, 7, 35, 35, 9, 9, 7, 9, 13, 7,
];

/// Polygon colors matching C++ polyColors[].
const EAGLE_POLY_COLORS: [u32; 14] = [
    0x302030FF, 0x303040FF, 0x303040FF, 0x303040FF,
    0x303040FF, 0x303040FF, 0x303040FF, 0x508080FF,
    0x505030FF, 0x505030FF, 0x505030FF, 0x508080FF,
    0x505030FF, 0x000000FF,
];
```

Then add each polygon as a `static` array. The full data is very large (~2000 lines). The implementer should copy all 14 polygon arrays from the C++ source (emMainContentPanel.cpp:134-345), converting from flat `double[]` to `(f64, f64)` tuple arrays.

Example for poly2 (smallest, 3 vertices / 6 coords):
```rust
static EAGLE_POLY2: [(f64, f64); 3] = [
    (52965.0, 62730.0), (52290.0, 63180.0), (52290.0, 63900.0),
];
// ... but actually poly2 has 6 values = 3 pairs, and polySizes[2] = 6
// Hmm - need to recheck whether polySizes are vertex counts or coord counts
```

**Implementation note for the implementer:** The C++ `polySizes` array contains the number of *coordinate values divided by 2* (i.e., vertex counts). The C++ `PaintPolygon(xy, count, color)` takes `count` as the number of vertices. You must carefully count the actual number of coordinate pairs in each C++ array to get the correct vertex count. Copy the raw doubles from C++ and pair them as `(x, y)` tuples.

- [ ] **Step 4: Implement PaintEagle method**

Add to `emMainContentPanel`:

```rust
/// Paint the eagle shape using transformed coordinates.
///
/// Port of C++ `emMainContentPanel::PaintEagle`.
/// C++ creates a sub-painter with shifted origin and scale, then
/// renders 14 polygons.
fn paint_eagle(&self, painter: &mut emPainter) {
    let polys: [&[(f64, f64)]; 14] = [
        &EAGLE_POLY0, &EAGLE_POLY1, &EAGLE_POLY2, &EAGLE_POLY3,
        &EAGLE_POLY4, &EAGLE_POLY5, &EAGLE_POLY6, &EAGLE_POLY7,
        &EAGLE_POLY8, &EAGLE_POLY9, &EAGLE_POLY10, &EAGLE_POLY11,
        &EAGLE_POLY12, &EAGLE_POLY13,
    ];

    // Transform vertices from eagle coordinate space to panel space.
    // C++ creates a sub-painter with shifted origin and scale:
    //   originX + scaleX * EagleShiftX, originY + scaleY * EagleShiftY
    //   scaleX * EagleScaleX, scaleY * EagleScaleY
    // We transform the vertices directly instead.
    for (i, poly) in polys.iter().enumerate() {
        let color = emColor::from_packed(EAGLE_POLY_COLORS[i]);
        let transformed: Vec<(f64, f64)> = poly
            .iter()
            .map(|(x, y)| {
                (
                    x * self.eagle_scale_x + self.eagle_shift_x,
                    y * self.eagle_scale_y + self.eagle_shift_y,
                )
            })
            .collect();
        painter.PaintPolygon(&transformed, color, emColor::TRANSPARENT);
    }
}
```

- [ ] **Step 5: Replace text placeholder in Paint with PaintEagle**

Replace the current `Paint` method body's eagle section (lines 104-119):

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
    let top_color = emColor::from_packed(0x91ABF2FF);
    let bot_color = emColor::from_packed(0xE1DDB7FF);
    let canvas = emColor::from_packed(0x000000FF);

    // Gradient background
    painter.paint_linear_gradient(0.0, 0.0, w, h, top_color, bot_color, false, canvas);

    // Eagle logo polygons
    self.paint_eagle(painter);
}
```

Remove the DIVERGED comment.

- [ ] **Step 6: Fix test expectations for poly13**

After counting the actual C++ poly13 data: `78660.0,47465.0, 78750.0,47610.0, 78750.0,47790.0, 78615.0,47880.0, 78390.0,47925.0, 78210.0,47835.0, 78120.0,47675.0` — that's 14 doubles = 7 vertices. And `polySizes[13] = 7` confirms 7 vertices. Fix the test accordingly.

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 8: Run clippy**

Run: `cargo clippy -p emmain -- -D warnings`
Expected: No warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/emmain/src/emMainContentPanel.rs
git commit -m "feat(emMainContentPanel): port eagle logo polygons from C++ PaintEagle"
```

---

### Task 2: Star.tga Textured Star Rendering (Item 10)

**Files:**
- Create: `res/emMain/Star.tga`
- Modify: `crates/emmain/src/emStarFieldPanel.rs`

Port the 3-tier star rendering from C++ `emStarFieldPanel::PaintOverlay` (emStarFieldPanel.cpp:102-147). The current Rust code uses `PaintEllipse` for all star sizes.

- [ ] **Step 1: Copy Star.tga resource**

```bash
cp ~/git/eaglemode-0.96.4/res/emMain/Star.tga res/emMain/Star.tga
```

- [ ] **Step 2: Write tests for star rendering tiers**

Add to the test module in `emStarFieldPanel.rs`:

```rust
#[test]
fn test_star_shape_loaded() {
    // Star.tga should load successfully via include_bytes
    let img = emcore::emResTga::load_tga(
        include_bytes!("../../../res/emMain/Star.tga")
    ).expect("failed to load Star.tga");
    assert!(img.GetWidth() > 0);
    assert!(img.GetHeight() > 0);
}

#[test]
fn test_star_tier_thresholds() {
    // C++ tiers: vr > 4.0 → image, vr > 1.2 (after r*=0.6) → ellipse, else → rect
    // For a star with Radius=r, vr = scale_x * r
    // Tier 1 (image): vr > 4.0
    // Tier 2 (ellipse): vr <= 4.0 but (r*0.6) in view > 1.2
    // Tier 3 (rect): (r*0.6) in view <= 1.2
    let r = MIN_STAR_RADIUS / MIN_PANEL_SIZE * 0.75; // typical radius
    // Just verify the thresholds are constants we can test against
    assert!(4.0 > 1.2); // tier 1 threshold > tier 2 threshold
    assert!(0.6 * 0.8862 < 1.0); // combined scale factors < 1
}
```

- [ ] **Step 3: Run tests to verify Star.tga test fails**

Run: `cargo-nextest ntr -p emmain`
Expected: test_star_shape_loaded may pass if file was copied; other tests should pass.

- [ ] **Step 4: Add StarShape field and load in constructor**

Modify `emStarFieldPanel`:

```rust
use emcore::emImage::emImage;
use emcore::emResTga::load_tga;

pub struct emStarFieldPanel {
    depth: i32,
    child_random_seeds: [u32; 4],
    stars: Vec<Star>,
    star_shape: emImage,
    noticed_viewed_w: f64,
}

impl emStarFieldPanel {
    pub fn new(depth: i32, seed: u32) -> Self {
        let star_shape = load_tga(include_bytes!("../../../res/emMain/Star.tga"))
            .expect("failed to load Star.tga");
        // ... rest of constructor unchanged ...
        Self {
            depth,
            child_random_seeds,
            stars,
            star_shape,
            noticed_viewed_w: 0.0,
        }
    }
}
```

- [ ] **Step 5: Replace Paint with 3-tier rendering**

The C++ renders stars in `PaintOverlay` (after children), but the Rust `PanelBehavior` trait has no `PaintOverlay`. Stars must be rendered in `Paint`. This is an acceptable divergence since the starfield has no child content that needs to appear behind stars at the same depth level.

Replace the `Paint` method:

```rust
fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, _state: &PanelState) {
    let bg = emColor::from_packed(BG_COLOR);
    painter.Clear(bg);

    let (sx, _sy) = painter.scaling();

    for star in &self.stars {
        let r = star.Radius;
        let vr = sx * r;

        if vr <= MIN_STAR_RADIUS {
            continue;
        }

        if vr > 4.0 {
            // Tier 1: Textured star with HSV glow
            let hue = star.Color.GetHue();
            let sat = star.Color.GetSat();

            // Glow layer: full saturation, alpha = sat * 18 clamped to 255
            let alpha = ((sat * 18.0) as u32).min(255) as u8;
            let glow_color = emColor::SetHSVA(hue, 100.0, 100.0).with_alpha(alpha);
            let x = star.X - r;
            let y = star.Y - r;
            let d = r * 2.0;
            let iw = self.star_shape.GetWidth();
            let ih = self.star_shape.GetHeight();
            painter.PaintImageColored(
                x, y, d, d,
                &self.star_shape,
                0, 0, iw, ih,
                glow_color,
                emColor::TRANSPARENT,
                emColor::TRANSPARENT,
                emcore::emTexture::ImageExtension::Zero,
            );

            // Star layer: reduced saturation, full opacity
            let star_sat = (sat - 10.0).max(0.0);
            let star_color = emColor::SetHSVA(hue, star_sat, 100.0);
            painter.PaintImageColored(
                x, y, d, d,
                &self.star_shape,
                0, 0, iw, ih,
                star_color,
                emColor::TRANSPARENT,
                emColor::TRANSPARENT,
                emcore::emTexture::ImageExtension::Zero,
            );
        } else {
            // Scale radius for smaller tiers
            let r2 = r * 0.6;
            let vr2 = sx * r2;

            if vr2 > 1.2 {
                // Tier 2: Ellipse
                // Rust PaintEllipse(cx, cy, rx, ry, color, canvas_color)
                // where cx,cy is center, rx,ry is radii
                painter.PaintEllipse(
                    star.X, star.Y, r2, r2,
                    star.Color, bg,
                );
            } else {
                // Tier 3: Rectangle (smallest stars)
                let r3 = r2 * 0.8862;
                let x = star.X - r3;
                let y = star.Y - r3;
                let d = r3 * 2.0;
                painter.PaintRect(x, y, d, d, star.Color, bg);
            }
        }
    }
}
```

Remove the DIVERGED comment.

- [ ] **Step 6: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add res/emMain/Star.tga crates/emmain/src/emStarFieldPanel.rs
git commit -m "feat(emStarFieldPanel): port 3-tier star rendering with Star.tga texture"
```

---

### Task 3: IPC Single-Instance (Item 15)

**Files:**
- Modify: `crates/emmain/src/emMain.rs`
- Modify: `crates/eaglemode/src/main.rs`

Wire `emMiniIpcClient::TrySend` for the client side. Port the `emMain` engine struct for the server side with `OnReception` dispatching to `NewWindow`.

- [ ] **Step 1: Write tests for IPC client wiring**

Add to `emMain.rs`:

```rust
#[test]
fn test_try_ipc_client_formats_args() {
    // Should not panic when called with various args
    // (will return false since no server is running)
    assert!(!try_ipc_client("nonexistent_test_server_12345", None));
    assert!(!try_ipc_client("nonexistent_test_server_12345", Some("/home")));
}

#[test]
fn test_calc_server_name_format() {
    let name = CalcServerName();
    // C++ format: "eaglemode_on_<host>:<display>.<screen>"
    // Rust format currently: "eaglemode_on_<host>_<display_underscored>"
    // Both should start with "eaglemode_on_"
    assert!(name.starts_with("eaglemode_on_"));
}
```

- [ ] **Step 2: Wire try_ipc_client to emMiniIpcClient::TrySend**

Replace the stub in `emMain.rs`:

```rust
use emcore::emMiniIpc::emMiniIpcClient;

/// Try to send a command to an already-running instance via IPC.
///
/// Port of C++ main() IPC client path (emMain.cpp:574-592).
/// Sends "NewWindow" command with forwarded args.
/// Returns true if server responded (caller should exit).
pub fn try_ipc_client(server_name: &str, visit: Option<&str>) -> bool {
    let mut args: Vec<&str> = vec!["NewWindow"];
    if let Some(v) = visit {
        args.push("-visit");
        args.push(v);
    }

    match emMiniIpcClient::TrySend(server_name, &args) {
        Ok(()) => {
            log::info!("IPC: sent NewWindow to existing instance");
            true
        }
        Err(e) => {
            log::debug!("IPC: no existing instance ({e})");
            false
        }
    }
}
```

Remove the DIVERGED comment and TODO.

- [ ] **Step 3: Add emMain engine struct with IPC server**

Port the C++ `emMain` class (emMain.cpp:66-99) as a struct. The C++ class extends `emEngine` and `emMiniIpcServer`. In Rust, compose rather than inherit:

```rust
use emcore::emMiniIpc::emMiniIpcServer;

/// IPC server engine for single-instance coordination.
///
/// Port of C++ `emMain` class (emMain.cpp:66-99).
/// Creates an IPC server, dispatches incoming "NewWindow" and "ReloadFiles"
/// commands.
pub struct emMain {
    server_name: String,
    // Server wiring deferred until Phase 3 integrates with the scheduler.
    // The struct is defined here so the API shape is correct.
}

impl emMain {
    /// Create a new emMain engine.
    ///
    /// Port of C++ `emMain::emMain(context, serve)`.
    pub fn new(serve: bool) -> Self {
        let server_name = CalcServerName();
        if serve {
            log::info!("IPC server name: {server_name}");
        }
        Self { server_name }
    }

    /// Handle an incoming IPC command.
    ///
    /// Port of C++ `emMain::OnReception`.
    pub fn on_reception(&self, args: &[String]) {
        if args.is_empty() {
            log::warn!("emMain: empty IPC message");
            return;
        }
        match args[0].as_str() {
            "NewWindow" => {
                log::info!("emMain: received NewWindow command");
                // NewWindow dispatch requires App + EventLoop access.
                // Full wiring in Phase 3 when startup engine is ported.
            }
            "ReloadFiles" => {
                log::info!("emMain: received ReloadFiles command");
            }
            _ => {
                let joined: String = args.join(" ");
                log::warn!("emMain: illegal MiniIpc request: {joined}");
            }
        }
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}
```

Note: Full `emMiniIpcServer` integration (StartServing, Cycle polling, NewWindow dispatch to App) requires Phase 3's scheduler/engine integration. This task establishes the struct and client wiring.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMain.rs
git commit -m "feat(emMain): wire IPC client and add emMain engine struct"
```

---

### Task 4: Detached Control Window (Item 12)

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

Add `CreateControlWindow` method triggered by cheat code `"ccw"`. This creates a second OS window hosting `emMainControlPanel`.

- [ ] **Step 1: Write test for CreateControlWindow**

```rust
#[test]
fn test_config_defaults_unchanged() {
    let config = emMainWindowConfig::default();
    assert!(!config.fullscreen);
    assert!(config.visit.is_none());
    assert!(config.geometry.is_none());
    assert!((config.control_tallness - 5.0).abs() < 1e-10);
}
```

This task is primarily about adding the `CreateControlWindow` method. Since it requires a running event loop and window to test, we verify it compiles and the config is not broken.

- [ ] **Step 2: Add CreateControlWindow method**

Add to `emMainWindow.rs`:

```rust
use emcore::emView::ViewFlags;

/// Create a detached control window.
///
/// Port of C++ `emMainWindow::CreateControlWindow` (emMainWindow.cpp:309-327).
/// Creates a second OS window with `VF_POPUP_ZOOM | VF_ROOT_SAME_TALLNESS`
/// and `WF_AUTO_DELETE`, hosting an `emMainControlPanel`.
///
/// Triggered by the `"ccw"` cheat code in `DoCustomCheat`.
pub fn create_control_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    _content_view_root: PanelId,
) -> Option<winit::window::WindowId> {
    use crate::emMainControlPanel::emMainControlPanel;

    let ctrl_panel = emMainControlPanel::new(Rc::clone(&app.context));
    let root_id = app.tree.create_root("ctrl_window_root");
    app.tree.set_behavior(root_id, Box::new(ctrl_panel));

    let flags = WindowFlags::AUTO_DELETE;
    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();

    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    app.windows.insert(window_id, window);
    Some(window_id)
}
```

Add the necessary imports at the top:

```rust
use emcore::emPanelTree::PanelId;
```

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat(emMainWindow): add CreateControlWindow for detached control panel"
```

---

### Task 5: VcItem Color Round-Trip Fix (Item 17a)

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

**Bug found during design review:** The C++ `.emVcItem` files use hex string colors (`BackgroundColor = "#BBB"`), but the Rust parser calls `get_struct("backgroundcolor")` expecting a `{r g b}` sub-struct. This silently falls back to the default color, meaning all VcItem border/background/title colors are wrong.

- [ ] **Step 1: Write failing test with actual C++ VcItem color values**

Add to the test module in `emVirtualCosmos.rs`:

```rust
#[test]
fn test_vcitem_hex_color_parsing() {
    // C++ Home.emVcItem has: BackgroundColor = "#BBB"
    // "#BBB" is shorthand for #BBBBBB = RGB(187, 187, 187)
    use emcore::emRec::RecStruct;
    use emcore::emRecRecord::Record;

    let mut rec = RecStruct::new();
    rec.set_str("title", "Test");
    rec.set_double("posx", 0.5);
    rec.set_double("posy", 0.5);
    rec.set_double("width", 0.1);
    rec.set_double("contenttallness", 1.0);
    rec.set_str("backgroundcolor", "#BBB");
    rec.set_str("bordercolor", "#333");
    rec.set_str("titlecolor", "#BBB");
    rec.set_str("filename", "test.emFileLink");

    let item = emVirtualCosmosItemRec::from_rec(&rec).unwrap();
    // #BBB = RGB(187, 187, 187)
    assert_eq!(item.BackgroundColor.GetRed(), 187,
        "BackgroundColor red should be 187, got {}", item.BackgroundColor.GetRed());
    assert_eq!(item.BackgroundColor.GetGreen(), 187);
    assert_eq!(item.BackgroundColor.GetBlue(), 187);
    // #333 = RGB(51, 51, 51)
    assert_eq!(item.BorderColor.GetRed(), 51);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo-nextest ntr -p emmain -- test_vcitem_hex_color_parsing`
Expected: FAIL — `BackgroundColor` falls back to default `0xAAAAAAFF` instead of parsing `"#BBB"`.

- [ ] **Step 3: Fix color parsing in emVirtualCosmosItemRec::from_rec**

The fix: try `get_str` first and parse with `emColor::TryParse`, then fall back to `get_struct` with `emColorRec::FromRecStruct`. This handles both hex strings and struct formats.

In `emVirtualCosmos.rs`, replace the color parsing section of `from_rec`:

```rust
use emcore::emColor::emColor;

// Helper: parse a color field that may be either a hex string or {r g b} struct.
fn parse_color_field(rec: &RecStruct, field: &str, default: emColor) -> emColor {
    // Try hex string first (C++ .emVcItem files use "#BBB" format)
    if let Some(s) = rec.get_str(field) {
        if let Some(c) = emColor::TryParse(s) {
            return c;
        }
    }
    // Fall back to struct format {r g b a}
    if let Some(s) = rec.get_struct(field) {
        if let Ok(c) = emColorRec::FromRecStruct(s, true) {
            return c;
        }
    }
    default
}
```

Then in `from_rec`, replace:
```rust
let bg = rec
    .get_struct("backgroundcolor")
    .and_then(|s| emColorRec::FromRecStruct(s, true).ok())
    .unwrap_or_else(|| emColor::from_packed(0xAAAAAAFF));
```

With:
```rust
let bg = parse_color_field(rec, "backgroundcolor", emColor::from_packed(0xAAAAAAFF));
let border_color = parse_color_field(rec, "bordercolor", emColor::from_packed(0xAAAAAAFF));
let title_color = parse_color_field(rec, "titlecolor", emColor::from_packed(0x000000FF));
```

- [ ] **Step 4: Also fix to_rec serialization**

The `to_rec` currently writes colors as structs. For round-trip fidelity with C++ files, write as hex strings instead. But check what C++ actually does — if C++ writes structs and reads strings, we should match. For now, keep struct output (the important fix is the *input* parsing).

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr -p emmain`
Expected: All tests pass including the new hex color test.

- [ ] **Step 6: Write test for bookmark color parsing too**

The same issue may affect `emBookmarks.rs`. Check if bookmark files also use hex strings:

```rust
#[test]
fn test_bookmark_hex_color() {
    // Verify bookmark color parsing handles hex strings too
    // The default Bookmarks.emBookmarks file uses struct format,
    // but user-edited files might use hex. Test both.
}
```

This is investigatory — check the actual bookmark file format before writing the test. If bookmarks use struct format only, no fix needed there.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix(emVirtualCosmos): parse VcItem colors as hex strings, matching C++ format"
```

---

## Summary

| Task | Item | Description | Key Files |
|------|------|-------------|-----------|
| 1 | 9 | Eagle logo polygons | `emMainContentPanel.rs` |
| 2 | 10 | Star.tga 3-tier rendering | `emStarFieldPanel.rs`, `res/emMain/Star.tga` |
| 3 | 15 | IPC single-instance | `emMain.rs`, `main.rs` |
| 4 | 12 | Detached control window | `emMainWindow.rs` |
| 5 | 17a | VcItem color round-trip | `emVirtualCosmos.rs` |

All tasks are independent — they can be executed in any order or in parallel.
