//! Sending the car lighting rig only when it changed.
use blackbox_gfx::{LightingRig, RenderBackend};

/// Calls after which an unchanged rig is sent again anyway. Other code (a car upload, another scene) can set the
/// renderer's rig between our calls, and this puts ours back; the cost is one small uniform write per this many frames.
const REFRESH_EVERY: u32 = 16;

/// The rig last sent to the renderer.
#[derive(Default)]
pub(super) struct RigMemory {
    /// The rig, and the vehicle reflection brightness (as bits) the renderer folded into it.
    last: Option<(LightingRig, u32)>,
    /// Calls since the last send.
    quiet: u32,
}

impl RigMemory {
    /// Sends `rig` unless the renderer already has it. Returns whether it was sent.
    pub fn send(&mut self, renderer: &mut dyn RenderBackend, rig: &LightingRig) -> bool {
        let brightness = renderer.graphics().vehicle_reflection_brightness.0.to_bits();
        let unchanged =
            self.last.as_ref().is_some_and(|(sent, sent_brightness)| sent == rig && *sent_brightness == brightness);
        if unchanged && self.quiet < REFRESH_EVERY {
            self.quiet += 1;
            return false;
        }
        renderer.set_lighting_rig(rig);
        self.last = Some((*rig, brightness));
        self.quiet = 0;
        true
    }

    /// The renderer's rig was set by someone else: send the next one whatever it is.
    pub fn forget(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_backend::NullBackend;

    fn rig() -> LightingRig {
        crate::scenes::car::lighting::rig(glam::Vec3::new(0.3, 0.4, 0.8).normalize())
    }

    #[test]
    fn an_unchanged_rig_is_sent_once() {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let mut memory = RigMemory::default();
        let rig = rig();
        for _ in 0..REFRESH_EVERY {
            memory.send(&mut renderer, &rig);
        }
        assert_eq!(renderer.lighting_updates, 1);
    }

    #[test]
    fn a_changed_rig_or_a_forgotten_one_is_sent_again() {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let mut memory = RigMemory::default();
        let rig = rig();
        assert!(memory.send(&mut renderer, &rig));
        assert!(!memory.send(&mut renderer, &rig));
        let mut changed = rig;
        changed.ambient += glam::Vec3::splat(0.1);
        assert!(memory.send(&mut renderer, &changed));
        assert!(!memory.send(&mut renderer, &changed));
        memory.forget();
        assert!(memory.send(&mut renderer, &changed));
        assert_eq!(renderer.lighting_updates, 3);
    }

    #[test]
    fn an_unchanged_rig_is_refreshed_after_a_while() {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let mut memory = RigMemory::default();
        let rig = rig();
        for _ in 0..=REFRESH_EVERY + 1 {
            memory.send(&mut renderer, &rig);
        }
        assert_eq!(renderer.lighting_updates, 2);
    }
}
