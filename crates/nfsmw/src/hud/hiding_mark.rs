//! The cool-down bar's "HIDING" mark (`docs/specs/pursuit-hiding.md` section 5). Under the cool-down group the package
//! holds an orange bar (`0x5c2a4f20`), the "HIDING" text group with its label, and a car icon, all resting at zero
//! alpha. The game drives the bar alone: while the cool-down shows and a hiding spot has started to latch the player it
//! plays the bar's appear, and otherwise its leave once the appear has run. The bar's appear raises `0x4027A94E` and its
//! leave `0x451CD24C`; the package answers them by fading the group, the label and the icon in (the icon then pulses)
//! and out.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

/// The orange bar the board drives.
pub(super) const BAR: u32 = 0x5c2a_4f20;
/// Its appear and leave (320 ms each), whose events fade the mark in and out.
pub(super) const APPEAR: u32 = 0x5079_C8F8;
const LEAVE: u32 = 0x0331_13AC;

/// The script the bar starts this update. `shown`: the cool-down shows and a hiding spot latches the player; `up`: the bar
/// plays its appear or rests at its end.
pub fn bar_script(shown: bool, up: bool) -> Option<u32> {
    match (shown, up) {
        (true, false) => Some(APPEAR),
        (false, true) => Some(LEAVE),
        _ => None,
    }
}

/// The bar the mark hangs on.
pub struct HidingMark {
    bar: Option<ObjectRef>,
}

impl HidingMark {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self { bar: rt.find(package, BAR) }
    }

    /// Plays the bar's appear when `shown` turns on and its leave when it turns off; nothing while it holds.
    pub fn apply(&self, rt: &mut Runtime, shown: bool) {
        let Some(bar) = self.bar else { return };
        let up = rt.script_of(bar) == Some(APPEAR);
        if let Some(script) = bar_script(shown, up) {
            rt.run_script(bar, script);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_appears_once_and_leaves_once() {
        assert_eq!(bar_script(true, false), Some(APPEAR), "the spot starts to latch in the cool-down");
        assert_eq!(bar_script(true, true), None, "an appear that runs or has run is not restarted");
        assert_eq!(bar_script(false, true), Some(LEAVE), "the spot lets go, or the cool-down ends");
        assert_eq!(bar_script(false, false), None, "a bar at rest stays at rest");
    }
}
