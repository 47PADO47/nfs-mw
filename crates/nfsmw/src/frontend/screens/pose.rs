//! What the screen stack asks of the backdrop: its look and the camera's pose.

use super::{Look, Screens};

impl Screens {
    /// The backdrop the stack wants: the look of the topmost screen that names one (its own, or the one its file name has).
    pub fn look(&self) -> Option<Look> {
        self.stack.iter().rev().find_map(|s| s.logic.look().or_else(|| s.look.clone()))
    }

    /// The pose collection the camera goes to: the topmost screen that asks for one (`factory::pose_for`). A screen that
    /// closes gives it back to the one below.
    pub fn pose(&self) -> Option<&'static str> {
        self.stack.iter().rev().find_map(|s| s.pose)
    }
}
