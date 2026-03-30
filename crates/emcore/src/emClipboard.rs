use std::cell::RefCell;
use std::rc::Rc;

use crate::emCore::emContext::emContext;

/// Abstract clipboard interface matching C++ emClipboard.
///
/// Provides clipboard and selection buffer operations. Concrete
/// implementations are installed into a emContext via `install`.
pub trait emClipboard {
    /// Put text into the clipboard or selection buffer.
    /// If `selection` is true, returns a selection ID (incrementing counter).
    /// If `selection` is false, returns 0.
    fn PutText(&mut self, text: &str, selection: bool) -> i64;

    /// Clear the clipboard or selection buffer.
    /// For selections, only clears if `selection_id` matches the current ID.
    fn Clear(&mut self, selection: bool, selection_id: i64);

    /// Get the current text from the clipboard or selection buffer.
    fn GetText(&self, selection: bool) -> String;
}

/// In-memory clipboard implementation matching C++ emPrivateClipboard.
///
/// Maintains separate clipboard and selection buffers with no platform integration.
pub struct emPrivateClipboard {
    clip_text: String,
    sel_text: String,
    sel_id: i64,
}

impl emPrivateClipboard {
    pub fn new() -> Self {
        Self {
            clip_text: String::new(),
            sel_text: String::new(),
            sel_id: 0,
        }
    }

    /// Install this clipboard into the given context.
    /// Port of C++ emPrivateClipboard::Install(emContext&).
    pub fn Install(context: &Rc<emContext>) {
        let clipboard: Rc<RefCell<dyn emClipboard>> = Rc::new(RefCell::new(Self::new()));
        context.set_clipboard(clipboard);
    }
}

impl Default for emPrivateClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl emClipboard for emPrivateClipboard {
    fn PutText(&mut self, text: &str, selection: bool) -> i64 {
        if selection {
            self.sel_text = text.to_string();
            self.sel_id += 1;
            self.sel_id
        } else {
            self.clip_text = text.to_string();
            0
        }
    }

    fn Clear(&mut self, selection: bool, selection_id: i64) {
        if selection {
            if selection_id == self.sel_id {
                self.sel_text.clear();
                self.sel_id += 1;
            }
        } else {
            self.clip_text.clear();
        }
    }

    fn GetText(&self, selection: bool) -> String {
        if selection {
            self.sel_text.clone()
        } else {
            self.clip_text.clone()
        }
    }
}

/// emLook up the installed clipboard by walking the context hierarchy.
/// Port of C++ emClipboard::LookupInherited(emContext&).
pub fn LookupInherited(context: &Rc<emContext>) -> Option<Rc<RefCell<dyn emClipboard>>> {
    context.LookupClipboard()
}
