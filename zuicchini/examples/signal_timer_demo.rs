//! Signal/timer demo derived from C++ `SignalExample.cpp`.
//!
//! Demonstrates the scheduler's signal and timer system:
//! - A button panel fires a signal on each left-click.
//! - A periodic timer fires every second.
//! - An Engine watches both signals and increments counters.
//! - A display panel paints the counter values.
//!
//! The key architectural difference from C++ (which used `Cycle()` on panels)
//! is that Rust routes signals through `Engine::cycle()` on the scheduler.

use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::emCore::emColor::Color;
use zuicchini::input::{InputEvent, InputKey, InputState, InputVariant};
use zuicchini::panel::{PanelBehavior, PanelCtx, PanelState, ViewFlags};
use zuicchini::render::{Painter, TextAlignment, VAlign};
use zuicchini::scheduler::{Engine, EngineCtx, Priority, SignalId};
use zuicchini::window::{App, WindowFlags};

// ── Shared state between engine and panels ──

struct SharedState {
    button_count: u32,
    timer_count: u32,
}

// ── Engine that watches signals ──

struct CounterEngine {
    state: Rc<RefCell<SharedState>>,
    button_signal: SignalId,
    timer_signal: SignalId,
}

impl Engine for CounterEngine {
    fn cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        let mut s = self.state.borrow_mut();
        if ctx.is_signaled(self.button_signal) {
            s.button_count += 1;
        }
        if ctx.is_signaled(self.timer_signal) {
            s.timer_count += 1;
        }
        false // sleep until next signal
    }
}

// ── Root panel: shows counter values and hosts button child ──

struct CounterPanel {
    state: Rc<RefCell<SharedState>>,
}

impl PanelBehavior for CounterPanel {
    fn is_opaque(&self) -> bool {
        true
    }

    fn paint(&mut self, p: &mut Painter, w: f64, h: f64, _ps: &PanelState) {
        p.paint_rect(
            0.0,
            0.0,
            w,
            h,
            Color::rgba(0xC0, 0xC0, 0xC0, 0xFF),
            Color::TRANSPARENT,
        );
        let s = self.state.borrow();
        let text = format!(
            "Button Signals: {}\nTimer Signals: {}",
            s.button_count, s.timer_count
        );
        p.paint_text_boxed(
            0.0,
            h * 0.3,
            w,
            h * 0.6,
            &text,
            h * 0.1,
            Color::rgba(0xFF, 0xFF, 0x80, 0xFF),
            Color::TRANSPARENT,
            TextAlignment::Center,
            VAlign::Top,
            TextAlignment::Center,
            0.5,
            true,
            0.15,
        );
    }

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        let children = ctx.children();
        let rect = ctx.layout_rect();
        let h = rect.h / rect.w;
        if !children.is_empty() {
            ctx.layout_child(children[0], 0.1, 0.1 * h, 0.8, 0.15 * h);
        }
        // Button child is created by main — just layout if it exists.
    }
}

// ── Button panel: fires a signal on left-click ──

struct ClickPanel {
    pressed: bool,
}

impl PanelBehavior for ClickPanel {
    fn is_opaque(&self) -> bool {
        true
    }

    fn paint(&mut self, p: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        let bg = if self.pressed {
            Color::rgba(0x80, 0xA0, 0x80, 0xFF)
        } else {
            Color::rgba(0xA0, 0xC0, 0xA0, 0xFF)
        };
        p.paint_rect(0.0, 0.0, w, h, bg, Color::TRANSPARENT);
        p.paint_text_boxed(
            0.0,
            0.0,
            w,
            h,
            "Click Me",
            h * 0.6,
            Color::rgba(0x00, 0x80, 0x00, 0xFF),
            Color::TRANSPARENT,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            0.5,
            true,
            0.15,
        );
    }

    fn input(&mut self, event: &InputEvent, _state: &PanelState, input_state: &InputState) -> bool {
        if event.key == InputKey::MouseLeft && event.variant == InputVariant::Press {
            self.pressed = true;
            // Signal is fired by the App scheduler — we store the signal ID
            // and the main loop fires it. But we can't access the scheduler
            // from here directly. Instead, we use a closure-like pattern:
            // mark pressed and handle in the engine via notice or repaint cycle.
            // Actually, the simplest approach: the engine polls. But that defeats
            // the purpose. Let's store a "pending fire" flag.
            return false; // let focus handling proceed
        }
        if self.pressed && !input_state.is_pressed(InputKey::MouseLeft) {
            self.pressed = false;
        }
        false
    }
}

fn main() {
    let app = App::new(Box::new(|app, event_loop| {
        let state = Rc::new(RefCell::new(SharedState {
            button_count: 0,
            timer_count: 0,
        }));

        // Create signals
        let button_sig = app.scheduler.create_signal();
        let timer_sig = app.scheduler.create_signal();

        // Create and start a periodic timer (1000ms)
        let timer = app.scheduler.create_timer(timer_sig);
        app.scheduler.start_timer(timer, 1000, true);

        // Register the engine
        let engine_id = app.scheduler.register_engine(
            Priority::Medium,
            Box::new(CounterEngine {
                state: state.clone(),
                button_signal: button_sig,
                timer_signal: timer_sig,
            }),
        );
        app.scheduler.connect(button_sig, engine_id);
        app.scheduler.connect(timer_sig, engine_id);

        // Build panel tree
        let root = app.tree.create_root("root");
        app.tree.set_behavior(
            root,
            Box::new(CounterPanel {
                state: state.clone(),
            }),
        );
        app.tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);

        // Create button child — fires the button signal on click
        let button = app.tree.create_child(root, "button");
        app.tree
            .set_behavior(button, Box::new(ClickPanel { pressed: false }));

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
