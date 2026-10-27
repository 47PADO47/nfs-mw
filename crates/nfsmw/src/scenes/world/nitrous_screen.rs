//! The screen and camera effects of a mod's nitrous variant (`[screen]` of its profile, `docs/modding/nitrous-variants.md`):
//! one struct fed every frame with the burn, holding the level that follows it and the camera nudge that level gives. The
//! focus filter takes the grade from it (`focus_feed.rs`) and the chase lens takes the nudge (`camera_choice.rs`).
//! Without a variant, or with one that has no `[screen]`, nothing here moves.

use blackbox_gfx::ScreenGrade;
use blackbox_modding::content::{BurnFollower, ScreenProfile};
use glam::{Mat4, Vec3};

use crate::viewer::camera::{Lens, projection};

/// The near plane the chase camera uses, kept when the field of view changes.
const NEAR_PLANE: f32 = 0.3;
/// The widest and narrowest field of view the nudge leaves (degrees), as the dynamic field of view camera does.
const FOV_LIMITS: (f32, f32) = (20.0, 150.0);
/// The clock of the shake wraps here (seconds), so its phase keeps its precision over a long drive.
const CLOCK_WRAP: f32 = 1000.0;

/// What the nitrous does to the camera this frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct CameraNudge {
    /// Degrees added to the field of view.
    pub fov: f32,
    /// Metres the eye moves back along its view.
    pub pull_back: f32,
    /// The shake: the eye's offset in the camera's frame (right, up, forward), metres.
    pub shake: [f32; 3],
}

impl CameraNudge {
    pub(super) fn is_zero(&self) -> bool {
        *self == Self::default()
    }

    /// `lens` with the nudge applied: the eye moved by the shake (and back by the pull-back when `pull_back` allows it, a
    /// mounted camera keeps its place), the field of view widened. The view keeps its rotation, so a rolled camera stays
    /// rolled.
    pub(super) fn apply(&self, lens: Lens, aspect: f32, pull_back: bool) -> Lens {
        if self.is_zero() {
            return lens;
        }
        let forward = lens.forward;
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(forward);
        let back = match pull_back {
            true => self.pull_back,
            false => 0.0,
        };
        let offset = right * self.shake[0] + up * self.shake[1] + forward * (self.shake[2] - back);
        let mut nudged =
            Lens { view: lens.view * Mat4::from_translation(-offset), position: lens.position + offset, ..lens };
        if self.fov != 0.0 {
            nudged.fov_y_degrees = (lens.fov_y_degrees + self.fov).clamp(FOV_LIMITS.0, FOV_LIMITS.1);
            nudged.projection = projection(nudged.fov_y_degrees, aspect, NEAR_PLANE);
        }
        nudged
    }
}

/// The grade of the frame at `level` of the burn: neutral when the profile has none.
pub(super) fn grade_for(screen: &ScreenProfile, level: f32) -> ScreenGrade {
    if !screen.has_grade() {
        return ScreenGrade::default();
    }
    ScreenGrade {
        edge: screen.edge_at(level),
        inner: screen.edge_inner,
        tint: screen.tint_at(level),
        contrast: screen.contrast_at(level),
    }
}

/// The follower of the burn and what it gives the frame.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct NitrousScreen {
    burn: BurnFollower,
    clock: f32,
    camera: CameraNudge,
}

impl NitrousScreen {
    /// Advance by `dt` seconds with the nitrous `burning` or not; returns the level, 0 to 1. Without a profile everything
    /// is at rest and the follower forgets the burn.
    pub(super) fn step(&mut self, screen: Option<ScreenProfile>, burning: bool, dt: f32) -> f32 {
        let Some(screen) = screen else {
            *self = Self::default();
            return 0.0;
        };
        let level = self.burn.step(burning, screen.fade_in, screen.fade_out, dt).level;
        self.clock = (self.clock + dt.max(0.0)) % CLOCK_WRAP;
        self.camera = CameraNudge {
            fov: screen.fov_at(level),
            pull_back: screen.pull_back_at(level),
            shake: screen.shake_at(self.clock, level),
        };
        level
    }

    /// The camera nudge of the last step.
    pub(super) fn camera(&self) -> CameraNudge {
        self.camera
    }
}

#[cfg(test)]
#[path = "nitrous_screen_tests.rs"]
mod tests;
