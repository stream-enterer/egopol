//! Input-handling demo derived from C++ `InputExample.cpp`.
//!
//! Logs keyboard and mouse events to an on-screen list, demonstrating
//! modifier matching, key press/release tracking, and mouse position.

use zuicchini::emCore::emColor::Color;
use zuicchini::input::{InputEvent, InputKey, InputState, InputVariant};
use zuicchini::panel::{PanelBehavior, PanelState, ViewFlags};
use zuicchini::render::{Painter, TextAlignment, VAlign};
use zuicchini::window::{App, WindowFlags};

const MAX_LOG: usize = 15;

struct InputPanel {
    x_key_down: bool,
    button_down: bool,
    last_mx: f64,
    last_my: f64,
    log: Vec<String>,
}

impl InputPanel {
    fn new() -> Self {
        Self {
            x_key_down: false,
            button_down: false,
            last_mx: 0.0,
            last_my: 0.0,
            log: Vec::new(),
        }
    }

    fn push_log(&mut self, msg: String) {
        if self.log.len() >= MAX_LOG {
            self.log.remove(0);
        }
        self.log.push(msg);
    }
}

impl PanelBehavior for InputPanel {
    fn is_opaque(&self) -> bool {
        true
    }

    fn input(&mut self, event: &InputEvent, _state: &PanelState, input_state: &InputState) -> bool {
        // E with no modifiers
        if event.key == InputKey::Key('e')
            && event.variant == InputVariant::Press
            && !event.shift
            && !event.ctrl
            && !event.alt
        {
            self.push_log("E pressed (no modifiers)".into());
            return true;
        }

        // Shift+Alt+G
        if event.key == InputKey::Key('g')
            && event.variant == InputVariant::Press
            && event.shift
            && event.alt
            && !event.ctrl
        {
            self.push_log("Shift+Alt+G".into());
            return true;
        }

        // Ctrl+V
        if event.key == InputKey::Key('v')
            && event.variant == InputVariant::Press
            && event.ctrl
            && !event.shift
            && !event.alt
        {
            self.push_log("Ctrl+V hotkey".into());
            return true;
        }

        // Dollar sign character
        if event.variant == InputVariant::Press && event.chars == "$" {
            self.push_log("Dollar sign ($) pressed".into());
            return true;
        }

        // X key press/release tracking
        if event.key == InputKey::Key('x') && event.variant == InputVariant::Press {
            self.x_key_down = true;
            self.push_log("X key pressed".into());
            return true;
        }
        if self.x_key_down && !input_state.is_pressed(InputKey::Key('x')) {
            self.x_key_down = false;
            self.push_log("X key released".into());
        }

        // Left mouse button
        if event.key == InputKey::MouseLeft && event.variant == InputVariant::Press {
            self.button_down = true;
            self.last_mx = event.mouse_x;
            self.last_my = event.mouse_y;
            self.push_log(format!(
                "Click at ({:.0}, {:.0})",
                event.mouse_x, event.mouse_y
            ));
            // Don't eat — let panel system handle focus.
            return false;
        }
        if self.button_down && !input_state.is_pressed(InputKey::MouseLeft) {
            self.button_down = false;
            self.push_log("Left button released".into());
        }

        // Mouse drag tracking
        if self.button_down
            && event.variant == InputVariant::Move
            && (self.last_mx != event.mouse_x || self.last_my != event.mouse_y)
        {
            self.last_mx = event.mouse_x;
            self.last_my = event.mouse_y;
            self.push_log(format!(
                "Dragged to ({:.0}, {:.0})",
                event.mouse_x, event.mouse_y
            ));
        }

        false
    }

    fn paint(&mut self, p: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        p.paint_rect(0.0, 0.0, w, h, Color::WHITE, Color::TRANSPARENT);

        // Title
        p.paint_text_boxed(
            0.0,
            0.0,
            w,
            h * 0.08,
            "Input Demo — press keys, click mouse",
            h * 0.05,
            Color::BLACK,
            Color::TRANSPARENT,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            0.5,
            true,
            0.15,
        );

        // Event log
        for (i, entry) in self.log.iter().enumerate() {
            p.paint_text(
                0.02 * w,
                (0.10 + i as f64 * 0.04) * h,
                entry,
                h * 0.03,
                1.0,
                Color::BLACK,
                Color::TRANSPARENT,
            );
        }

        // Mouse position indicator
        if self.button_down {
            let sz = 0.01 * w;
            p.paint_rect(
                self.last_mx - sz,
                self.last_my - sz,
                sz * 2.0,
                sz * 2.0,
                Color::rgba(255, 0, 0, 180),
                Color::TRANSPARENT,
            );
        }
    }
}

fn main() {
    let app = App::new(Box::new(|app, event_loop| {
        let root = app.tree.create_root("root");
        app.tree.set_behavior(root, Box::new(InputPanel::new()));
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
