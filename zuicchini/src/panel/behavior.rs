use bitflags::bitflags;

use crate::foundation::Color;
use crate::input::{Cursor, InputEvent};
use crate::render::Painter;

use super::ctx::PanelCtx;
use super::tree::{PanelId, PlaybackState};

bitflags! {
    /// Flags indicating what kinds of changes a panel needs to be notified about.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct NoticeFlags: u32 {
        /// Panel's layout rect has changed.
        const LAYOUT_CHANGED  = 0b0000_0001;
        /// Panel gained or lost focus.
        const FOCUS_CHANGED   = 0b0000_0010;
        /// Panel became visible or invisible.
        const VISIBILITY      = 0b0000_0100;
        /// A child was added or removed.
        const CHILDREN_CHANGED = 0b0000_1000;
        /// Canvas color changed.
        const CANVAS_CHANGED  = 0b0001_0000;
        /// Panel is being viewed (visit state changed).
        const VIEW_CHANGED    = 0b0010_0000;
        /// Panel's enable state changed.
        const ENABLE_CHANGED  = 0b0100_0000;
        /// The sought child name (for seeking navigation) changed.
        const SOUGHT_NAME_CHANGED = 0b1000_0000;
    }
}

/// Trait for panel behavior — the logic attached to a panel node.
///
/// All methods have default no-op implementations. Implementors override
/// only the methods they need.
pub trait PanelBehavior {
    /// Paint the panel's content.
    fn paint(&mut self, _painter: &mut Painter, _w: f64, _h: f64) {}

    /// Handle an input event. Returns true if the event was consumed.
    fn input(&mut self, _event: &InputEvent) -> bool {
        false
    }

    /// Get the cursor to display when the mouse is over this panel.
    fn get_cursor(&self) -> Cursor {
        Cursor::Normal
    }

    /// Whether this panel is fully opaque (no need to paint panels behind it).
    fn is_opaque(&self) -> bool {
        false
    }

    /// Layout child panels. Called when the panel's layout rect changes.
    fn layout_children(&mut self, _ctx: &mut PanelCtx) {}

    /// Receive a notice about state changes.
    fn notice(&mut self, _flags: NoticeFlags) {}

    /// Whether the panel wants to auto-expand to fill available space.
    fn auto_expand(&self) -> bool {
        false
    }

    /// Whether the panel wants to auto-shrink to fit its content.
    fn auto_shrink(&self) -> bool {
        false
    }

    /// Called each scheduler cycle while the panel is awake.
    fn cycle(&mut self) {}

    /// Get the minimum size this panel needs.
    fn min_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    /// Get the preferred/natural size of this panel.
    fn preferred_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    /// Get the canvas color for this panel (used for canvas blending).
    fn canvas_color(&self) -> Color {
        Color::TRANSPARENT
    }

    /// Return a title for this panel, or `None` to delegate to the parent.
    ///
    /// The tree walks up the parent chain; the root returns `"untitled"` if
    /// no behavior along the chain provides a title.
    fn get_title(&self) -> Option<String> {
        None
    }

    /// Return an icon filename for this panel, or `None` to delegate to the
    /// parent.
    ///
    /// The tree walks up the parent chain; the root returns `""` if no
    /// behavior along the chain provides an icon filename.
    fn get_icon_file_name(&self) -> Option<String> {
        None
    }

    /// Return the current playback state.
    ///
    /// The default returns `PlaybackState { playing: false, pos: 0.0,
    /// supported: false }`.
    fn get_playback_state(&self) -> PlaybackState {
        PlaybackState::default()
    }

    /// Attempt to set the playback state. Returns `true` if the panel
    /// supports playback and accepted the new state.
    fn set_playback_state(&mut self, _playing: bool, _pos: f64) -> bool {
        false
    }

    /// Whether this panel has hope that a seeking operation can succeed.
    ///
    /// The default returns `false`.
    fn is_hope_for_seeking(&self) -> bool {
        false
    }

    /// Create a control panel as a child of `parent_ctx` with `name`.
    ///
    /// Return the new panel's id, or `None` to delegate to the parent
    /// (the tree walks up the parent chain; the root returns `None`).
    fn create_control_panel(&mut self, _parent_ctx: &mut PanelCtx, _name: &str) -> Option<PanelId> {
        None
    }
}
