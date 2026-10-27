//! The original launch cue complements the tachometer needle, after custom RGB.
use blackbox_feng::{ObjectRef, Runtime};

#[derive(Default)]
pub(super) struct LaunchNeedle {
    highlighted: bool,
    base: [u8; 3],
    applied: [u8; 3],
}

impl LaunchNeedle {
    pub fn apply(&mut self, rt: &mut Runtime, needle: Option<ObjectRef>, active: bool, custom: Option<[u8; 3]>) {
        let Some(needle) = needle else { return };
        let Some([r, g, b, _]) = rt.colour_rgba(needle) else { return };
        let current = [r, g, b];
        if !active && !self.highlighted {
            return;
        }
        let base = custom.unwrap_or_else(|| match self.highlighted && current == self.applied {
            true => self.base,
            false => current,
        });
        let rgb = match active {
            true => base.map(|channel| !channel),
            false => base,
        };
        rt.set_colour_rgba(needle, [rgb[0], rgb[1], rgb[2], 255]);
        self.highlighted = active;
        self.base = base;
        self.applied = rgb;
    }
}
