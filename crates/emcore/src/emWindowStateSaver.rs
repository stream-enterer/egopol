use std::path::PathBuf;

use crate::emCore::emRec::RecStruct;
use crate::emCore::emConfigModel::emConfigModel;
use crate::emCore::emRec::RecError;
use crate::emCore::emRecRecord::Record;
use crate::emCore::emSignal::SignalId;

/// Persisted window geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 720,
            maximized: false,
            fullscreen: false,
        }
    }
}

impl Record for WindowGeometry {
    fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        Ok(Self {
            x: rec
                .get_int("x")
                .ok_or_else(|| RecError::MissingField("x".into()))?,
            y: rec
                .get_int("y")
                .ok_or_else(|| RecError::MissingField("y".into()))?,
            width: rec
                .get_int("width")
                .ok_or_else(|| RecError::MissingField("width".into()))? as u32,
            height: rec
                .get_int("height")
                .ok_or_else(|| RecError::MissingField("height".into()))? as u32,
            maximized: rec.get_bool("maximized").unwrap_or(false),
            fullscreen: rec.get_bool("fullscreen").unwrap_or(false),
        })
    }

    fn to_rec(&self) -> RecStruct {
        let mut s = RecStruct::new();
        s.set_int("x", self.x);
        s.set_int("y", self.y);
        s.set_int("width", self.width as i32);
        s.set_int("height", self.height as i32);
        s.set_bool("maximized", self.maximized);
        s.set_bool("fullscreen", self.fullscreen);
        s
    }

    fn SetToDefault(&mut self) {
        *self = Self::default();
    }

    fn IsSetToDefault(&self) -> bool {
        *self == Self::default()
    }
}

/// Saves and restores window geometry via a emConfigModel.
pub struct emWindowStateSaver {
    model: emConfigModel<WindowGeometry>,
    /// Cached normal-mode geometry, preserved when maximized/fullscreen.
    /// Matches C++ OwnNormalX/Y/W/H.
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}

impl emWindowStateSaver {
    pub fn new(path: PathBuf, signal_id: SignalId) -> Self {
        let defaults = WindowGeometry::default();
        Self {
            normal_x: defaults.x,
            normal_y: defaults.y,
            normal_w: defaults.width,
            normal_h: defaults.height,
            model: emConfigModel::new(defaults, path, signal_id),
        }
    }

    /// Save the current window position/size.
    ///
    /// When maximized or fullscreen, the last normal-mode geometry is
    /// preserved (matching C++ emWindowStateSaver::Save behavior).
    pub fn Save(&mut self, window: &super::emWindow::ZuiWindow) {
        use crate::emCore::emWindow::WindowFlags;

        let pos = window.winit_window.outer_position().unwrap_or_default();
        let size = window.winit_window.inner_size();
        let maximized = window.flags.contains(WindowFlags::MAXIMIZED);
        let fullscreen = window.flags.contains(WindowFlags::FULLSCREEN);

        // Only update normal geometry when NOT maximized/fullscreen.
        if !maximized && !fullscreen {
            self.normal_x = pos.x;
            self.normal_y = pos.y;
            self.normal_w = size.width;
            self.normal_h = size.height;
        }

        self.model.Set(WindowGeometry {
            x: self.normal_x,
            y: self.normal_y,
            width: self.normal_w,
            height: self.normal_h,
            maximized,
            fullscreen,
        });
    }

    /// Get the stored geometry for restoring.
    pub fn Restore(&self) -> &WindowGeometry {
        self.model.GetRec()
    }

    pub fn Cycle(&mut self, window: &super::emWindow::ZuiWindow, focused: bool) {
        use crate::emCore::emWindow::WindowFlags;

        let pos = window.winit_window.outer_position().unwrap_or_default();
        let size = window.winit_window.inner_size();
        let maximized = window.flags.contains(WindowFlags::MAXIMIZED);
        let fullscreen = window.flags.contains(WindowFlags::FULLSCREEN);

        let current = WindowGeometry {
            x: if !maximized && !fullscreen {
                pos.x
            } else {
                self.normal_x
            },
            y: if !maximized && !fullscreen {
                pos.y
            } else {
                self.normal_y
            },
            width: if !maximized && !fullscreen {
                size.width
            } else {
                self.normal_w
            },
            height: if !maximized && !fullscreen {
                size.height
            } else {
                self.normal_h
            },
            maximized,
            fullscreen,
        };

        if focused && current != *self.model.GetRec() {
            self.Save(window);
        }
    }

    pub fn model(&self) -> &emConfigModel<WindowGeometry> {
        &self.model
    }
}


#[cfg(kani)]
mod kani_private_proofs {
    use super::*;

    #[kani::proof]
    fn kani_private_WindowGeometry_IsSetToDefault() {
        let mut self_val = WindowGeometry { x: kani::any::<i32>(), y: kani::any::<i32>(), width: kani::any::<u32>(), height: kani::any::<u32>(), maximized: kani::any::<bool>(), fullscreen: kani::any::<bool>() };
        let _r = self_val.IsSetToDefault();
    }

    #[kani::proof]
    fn kani_private_WindowGeometry_SetToDefault() {
        let mut self_val = WindowGeometry { x: kani::any::<i32>(), y: kani::any::<i32>(), width: kani::any::<u32>(), height: kani::any::<u32>(), maximized: kani::any::<bool>(), fullscreen: kani::any::<bool>() };
        let _r = self_val.SetToDefault();
    }
}
