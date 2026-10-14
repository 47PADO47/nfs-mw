//! The hand-over between a sidebar and the front end, for `feng-viewer`. The viewer's scene keeps one end: it writes
//! the name clicked in its list, and whether the pointer is over the list. The front end keeps the other: it shows
//! the picked screen, and leaves clicks and wheel turns over the list to the list.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Default)]
struct Shared {
    picked: Mutex<Option<String>>,
    over_panel: AtomicBool,
}

/// The two ends of the hand-over (see the module docs). Clones share the same state.
#[derive(Clone, Default)]
pub struct Browse(Arc<Shared>);

impl Browse {
    /// Asks the front end to show the screen `name` next. A pick not yet shown is replaced by this one.
    pub fn pick(&self, name: &str) {
        *self.0.picked.lock().unwrap_or_else(PoisonError::into_inner) = Some(name.to_owned());
    }

    /// The screen picked since the last call, if any.
    pub fn take(&self) -> Option<String> {
        self.0.picked.lock().unwrap_or_else(PoisonError::into_inner).take()
    }

    /// Set by the scene on every UI pass: whether the pointer is over its list.
    pub fn set_over_panel(&self, over: bool) {
        self.0.over_panel.store(over, Ordering::Relaxed);
    }

    /// Whether the pointer was over the list on the last UI pass.
    pub fn over_panel(&self) -> bool {
        self.0.over_panel.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_is_taken_once_and_the_last_one_wins() {
        let browse = Browse::default();
        assert_eq!(browse.take(), None);
        browse.pick("MainMenu.fng");
        browse.pick("Options.fng");
        assert_eq!(browse.take().as_deref(), Some("Options.fng"));
        assert_eq!(browse.take(), None, "a pick is shown once");
    }

    #[test]
    fn the_clones_share_the_pick_and_the_pointer_state() {
        let scene_end = Browse::default();
        let front_end = scene_end.clone();
        scene_end.pick("Pause_Main.fng");
        assert_eq!(front_end.take().as_deref(), Some("Pause_Main.fng"));
        assert!(!front_end.over_panel());
        scene_end.set_over_panel(true);
        assert!(front_end.over_panel());
    }
}
