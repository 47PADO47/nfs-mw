//! The pursuit camera: the pan when the player is first caught, and the arrest framing when busted. Procedural: the
//! game's arrest camera tracks are not read (the ICE reader is deferred). The state advances once per physics tick
//! (`Drive::update_pursuit`), and the drive's camera reads its shot. Spec: `docs/specs/camera-views.md`.

mod framing;
#[cfg(test)]
mod tests;

use glam::Vec3;

use super::ChaseView;
use framing::Framing;

/// Seconds the caught pan lasts, from the first catch until the chase camera has its way back.
const CATCH_SECS: f32 = 2.5;
/// Seconds the arrest framing holds: the busted HUD's three seconds (`docs/specs/ai-pursuit.md` section 5.2).
const ARREST_SECS: f32 = 3.0;
/// Seconds the arrest framing takes to blend in and out.
const ARREST_BLEND_SECS: f32 = 0.6;

/// What the chase gives the pursuit camera each tick. Positions are render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitFeed {
    /// The busted meter, 0 to 1.
    pub meter: f32,
    /// The player has been busted in this chase (it stays so until the next chase).
    pub busted: bool,
    /// The nearest cop whose height matches the player's, if there is one.
    pub cop: Option<Vec3>,
    /// The player's position.
    pub player: Vec3,
}

/// The arrest framing in progress: seconds since it began, and the framing fixed when the bust happened.
#[derive(Debug, Clone, Copy)]
struct Arrest {
    elapsed: f32,
    framing: Framing,
}

/// The shot this tick: a framing and how far it takes over from the chase view, 0 to 1.
#[derive(Debug, Clone, Copy)]
struct Shot {
    framing: Framing,
    weight: f32,
}

pub struct PursuitCamera {
    /// A catch may start a pan: true at the start of each chase and again once the meter drains to zero.
    armed: bool,
    /// The busted flag of the last tick, for the bust's rising edge.
    was_busted: bool,
    /// Seconds into the caught pan, while it runs.
    catch: Option<f32>,
    arrest: Option<Arrest>,
    /// The cop last seen, kept when the feed loses it.
    last_cop: Option<Vec3>,
    shot: Option<Shot>,
}

impl Default for PursuitCamera {
    fn default() -> Self {
        Self { armed: true, was_busted: false, catch: None, arrest: None, last_cop: None, shot: None }
    }
}

impl PursuitCamera {
    /// Advances the pursuit camera by `dt` seconds of simulated time and reads the chase's feed.
    pub fn update(&mut self, dt: f32, feed: PursuitFeed) {
        let bust_starts = feed.busted && !self.was_busted;
        self.was_busted = feed.busted;
        self.last_cop = feed.cop.or(self.last_cop);
        self.step(dt);
        if !feed.busted {
            self.arrest = None;
        }
        if feed.meter <= 0.0 {
            self.armed = true;
        }
        if bust_starts {
            self.begin_arrest(feed.player);
        }
        if !feed.busted && self.armed && feed.meter > 0.0 {
            self.begin_catch();
        }
        self.shot = self.current_shot(feed.player);
    }

    /// The chase view with this tick's pursuit shot blended in; the chase view unchanged with no shot running.
    pub fn shot(&self, base: ChaseView) -> ChaseView {
        let Some(shot) = self.shot.filter(|shot| shot.weight > 0.0) else {
            return base;
        };
        let weight = shot.weight;
        let framing = shot.framing;
        ChaseView::reframed(
            base.position.lerp(framing.position, weight),
            base.target().lerp(framing.target, weight),
            base.fov_degrees() + (framing.fov_degrees - base.fov_degrees()) * weight,
        )
    }

    /// Moves the caught pan and the arrest framing on by `dt`; each ends when its time is up.
    fn step(&mut self, dt: f32) {
        self.catch = self.catch.map(|t| t + dt).filter(|t| *t < CATCH_SECS);
        self.arrest =
            self.arrest.take().map(|a| Arrest { elapsed: a.elapsed + dt, ..a }).filter(|a| a.elapsed < ARREST_SECS);
    }

    fn begin_arrest(&mut self, player: Vec3) {
        self.catch = None;
        let Some(cop) = self.last_cop else {
            return;
        };
        self.arrest = Some(Arrest { elapsed: 0.0, framing: framing::between(player, cop) });
    }

    fn begin_catch(&mut self) {
        if self.last_cop.is_none() {
            return;
        }
        self.armed = false;
        self.catch = Some(0.0);
    }

    fn current_shot(&self, player: Vec3) -> Option<Shot> {
        if let Some(arrest) = self.arrest {
            return Some(Shot { framing: arrest.framing, weight: arrest_weight(arrest.elapsed) });
        }
        let elapsed = self.catch?;
        let cop = self.last_cop?;
        Some(Shot { framing: framing::between(player, cop), weight: catch_weight(elapsed / CATCH_SECS) })
    }
}

/// The caught pan's blend: 0 at its start and its end, 1 halfway, eased at both ends.
fn catch_weight(progress: f32) -> f32 {
    smooth((std::f32::consts::PI * progress.clamp(0.0, 1.0)).sin())
}

/// The arrest framing's blend: eases in and out over the blend time, and holds between.
fn arrest_weight(elapsed: f32) -> f32 {
    let in_ = smooth(elapsed / ARREST_BLEND_SECS);
    let out = smooth((ARREST_SECS - elapsed) / ARREST_BLEND_SECS);
    in_.min(out)
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
