//! Mouse capture for mouse-look scenes: the cursor is hidden and held in the
//! window, so mouse motion turns the camera without holding a button.

use winit::window::{CursorGrabMode, Window};

/// Capture (`true`) or release the cursor. Returns whether it is captured now.
pub fn set_captured(window: &Window, captured: bool) -> bool {
    if !captured {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        return false;
    }
    // Not every platform can lock the cursor in place (Windows can only confine it).
    let grabbed = window
        .set_cursor_grab(CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        .is_ok();
    if grabbed {
        window.set_cursor_visible(false);
    } else {
        log::warn!("could not capture the mouse; hold the right button to look around");
    }
    grabbed
}
