//! The cool-down bar's "HIDING" mark: a text group and a car icon under the cool-down group. They rest at zero alpha
//! and each carries a fade-in and a fade-out script; the package answers two messages with them
//! (`0x4027A94E` plays the fade-in, `0x451CD24C` the fade-out), which the original raises when the hiding spot starts to
//! latch the player and when it lets go. Spec: `docs/specs/pursuit-hiding.md` section 5.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::elements::HIDING;

/// The script that takes an object from zero alpha to its full colour (its track runs from -255 to 0 of alpha). The
/// car icon chains it into a slow pulse.
const FADE_IN: u32 = 1_350_158_584;
/// The script that takes it back to zero alpha.
const FADE_OUT: u32 = 53_547_948;

/// The objects of the mark and whether it is on.
pub struct HidingMark {
    objects: Vec<ObjectRef>,
    on: bool,
}

impl HidingMark {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self { objects: HIDING.iter().filter_map(|hash| rt.find(package, *hash)).collect(), on: false }
    }

    /// Fades the mark in when `hiding` turns on and out when it turns off; nothing happens while it holds.
    pub fn apply(&mut self, rt: &mut Runtime, hiding: bool) {
        if hiding == self.on {
            return;
        }
        self.on = hiding;
        let script = match hiding {
            true => FADE_IN,
            false => FADE_OUT,
        };
        for object in &self.objects {
            rt.run_script(*object, script);
        }
    }
}
