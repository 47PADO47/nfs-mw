//! The pursuit look (`docs/specs/pursuit-look.md`). When a pursuit starts, or the player's whole heat level rises, the
//! frame's grade blends to the cold `uves` look and eases back over the transition's length, the frame flashes bright
//! and a radial blur fades out. The first frame after a reset or a scene only records the state, so a load never fires.

use blackbox_gfx::ColourLook;
use nfsmw_data::visual_treatment::pursuit::{Effect, Profile};

/// The share of the pursuit look's grade change, flash and radial blur that a bike's helmet camera keeps (`Frame::scaled`). A
/// modelling choice: the effects are made for a car's outside camera, and from a rider's eyes at the start of a chase the blue
/// flash and the blur hide the road; 0.35 keeps the start readable and still felt (`docs/specs/pursuit-look.md`).
pub(super) const HELMET_SHARE: f32 = 0.35;

/// A running effect: the seconds since it started, on its own clock.
#[derive(Debug, Clone, Copy, Default)]
struct Timer(Option<f32>);

impl Timer {
    fn advance(&mut self, dt: f32) {
        if let Some(elapsed) = self.0.as_mut() {
            *elapsed += dt.max(0.0);
        }
    }

    /// This frame's value of `effect`, 0 while it does not run. It stops once the heat level falls under the effect's
    /// trigger, after giving this frame's value.
    fn value(&mut self, effect: Option<&Effect>, level: f32) -> f32 {
        let (Some(elapsed), Some(effect)) = (self.0, effect) else { return 0.0 };
        if level < effect.heat_trigger {
            self.0 = None;
        }
        effect.value(elapsed)
    }
}

/// What the look gives one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Frame {
    /// The grade's change: the ordinary grade blended towards the pursuit or arrest look, and the flash.
    pub look: ColourLook,
    /// The strength of the pursuit's radial blur, 0 to 1.
    pub radial_blur: f32,
}

impl Frame {
    /// The frame with its grade change, its flash and its blur at `share` of their strength (0 to 1).
    pub fn scaled(self, share: f32) -> Self {
        let look = ColourLook {
            weight: self.look.weight * share,
            brightness: 1.0 + (self.look.brightness - 1.0) * share,
            ..self.look
        };
        Self { look, radial_blur: self.radial_blur * share }
    }
}

pub(super) struct PursuitLook {
    profile: Profile,
    /// The whole heat level and whether a pursuit ran on the last frame; `None` after a reset or during a scene.
    last_level: Option<i32>,
    was_pursued: Option<bool>,
    transition: Timer,
    pulse: Timer,
    radial: Timer,
}

impl PursuitLook {
    pub fn new(profile: Profile) -> Self {
        if profile.look.is_none() || profile.transition.is_none() {
            log::warn!("pursuit look or its transition unavailable; the pursuit grade is left out");
        }
        Self {
            profile,
            last_level: None,
            was_pursued: None,
            transition: Timer::default(),
            pulse: Timer::default(),
            radial: Timer::default(),
        }
    }

    /// Forgets the heat and pursuit state and stops every effect: the next frame only records the state.
    pub fn reset(&mut self) {
        self.hold();
        self.radial = Timer::default();
    }

    /// A frame while an in-world scene plays: the state is forgotten, and the flash and the blend stop. The blur runs
    /// on.
    pub fn hold(&mut self) {
        self.last_level = None;
        self.was_pursued = None;
        self.transition = Timer::default();
        self.pulse = Timer::default();
    }

    /// A frame of an arrest scene: its own look as it is, with no flash or blur. The state is forgotten, so the
    /// scene's end never fires.
    pub fn arrest(&mut self) -> Frame {
        self.reset();
        Frame { look: ColourLook { target: self.profile.arrest, weight: 1.0, brightness: 1.0 }, radial_blur: 0.0 }
    }

    /// This frame's look for the player's `heat` and whether a pursuit runs. `world_dt` is the world's step (the
    /// speedbreaker slows it), `real_dt` the frame's: the blend and the blur run on world time, the flash on real time.
    pub fn update(&mut self, heat: f32, pursued: bool, world_dt: f32, real_dt: f32) -> Frame {
        let level = heat.max(0.0) as i32;
        self.transition.advance(world_dt);
        self.radial.advance(world_dt);
        self.pulse.advance(real_dt);
        let rose = self.last_level.is_some_and(|last| level > last);
        let started = self.was_pursued == Some(false) && pursued;
        if rose || started {
            // A new start restarts a running effect from zero; it does not stack.
            for (timer, effect) in [
                (&mut self.transition, self.profile.transition),
                (&mut self.pulse, self.profile.pulse),
                (&mut self.radial, self.profile.radial_blur),
            ] {
                if effect.is_some() {
                    timer.0 = Some(0.0);
                }
            }
        }
        self.last_level = Some(level);
        self.was_pursued = Some(pursued);
        let level = level as f32;
        let weight = self.transition.value(self.profile.transition.as_ref(), level).clamp(0.0, 1.0);
        let pulse = self.pulse.value(self.profile.pulse.as_ref(), level);
        let radial_blur = self.radial.value(self.profile.radial_blur.as_ref(), level).clamp(0.0, 1.0);
        let look = ColourLook { target: self.profile.look, weight, brightness: (1.0 + pulse).max(0.0) };
        Frame { look, radial_blur }
    }
}

#[cfg(test)]
#[path = "pursuit_look_tests.rs"]
mod tests;
