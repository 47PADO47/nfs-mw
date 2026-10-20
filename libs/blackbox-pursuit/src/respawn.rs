//! When a cop that has fallen out of play is quietly taken off the road and may be spawned again elsewhere
//! (`docs/specs/ai-pursuit-cops.md` sections 2, 3.3 and 5). The patrol pass visits every active cop car each run and
//! asks three questions: may a car stand at this spot unseen ([`available`]), has this car been out long enough for it to
//! go ([`Latch::can_respawn`]), and is a chase about to meet it ([`near_target`]).
//!
//! Nothing here reads a world or a clock: the caller measures the distances and times.

use glam::Vec3;

/// The range from the camera a cop spawns in, metres. A spot nearer than `min` is never recycled; one farther than the
/// middle of the range always can be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub min: f32,
    pub max: f32,
}

/// The cop manager's spawn window.
pub const COP_WINDOW: Window = Window { min: 150.0, max: 400.0 };

/// Seconds after its spawn a cop that has been inside the no-recycle area may go (`AIVehicle::CanRespawn`).
pub const SETTLED_SECS: f32 = 8.0;
/// Seconds after its spawn a cop may go whether or not it ever stood in the no-recycle area (`kNeverVisibleRespawnTime`).
pub const NEVER_SEEN_SECS: f32 = 10.0;
/// Seconds a wrecked cop must be off screen before it is removed.
pub const WRECK_OFFSCREEN_SECS: f32 = 3.0;
/// How far ahead of its target a chase keeps cops from being removed (metres along the target's travel).
pub const AHEAD: f32 = 75.0;
/// A cop closer than this to that point is about to meet its target and stays (a helicopter: [`HELICOPTER_NEAR`]).
pub const NEAR: f32 = 375.0;
pub const HELICOPTER_NEAR: f32 = 600.0;

/// The lowest dot product of the direction to a spot with the camera's forward at which the spot is still ahead enough
/// to stay, and the highest. A spot between them (beside or behind the camera, up to just off straight ahead) may be
/// recycled between `min` and the middle of the window.
const OUTSIDE_CONE: (f32, f32) = (-0.707, 0.8);

/// Whether a cop standing at horizontal distance `distance` from the camera may be removed on position alone
/// (`RespawnAvailable`). `to_cop_dot_forward` is the dot of the unit direction camera-to-cop with the camera's forward,
/// or `None` when there is no active view (then only the far rule applies).
pub fn available(window: Window, distance: f32, to_cop_dot_forward: Option<f32>) -> bool {
    if distance <= window.min {
        return false;
    }
    if distance > (window.min + window.max) * 0.5 {
        return true;
    }
    to_cop_dot_forward.is_some_and(|dot| dot <= OUTSIDE_CONE.1 && dot >= OUTSIDE_CONE.0)
}

/// The latch of `AIVehicle::CanRespawn`: whether the car has stood where recycling is not available since it spawned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Latch {
    stood_in_play: bool,
}

impl Latch {
    /// Whether the car may be recycled now. `available` is [`available`] for its position, `age` the seconds since it
    /// spawned. A car that has stood where recycling is not available may go 8 s after its spawn; one that was never
    /// there waits 10 s.
    pub fn can_respawn(&mut self, available: bool, age: f32) -> bool {
        if !available {
            self.stood_in_play = true;
        }
        match (self.stood_in_play, available) {
            (true, _) if age > SETTLED_SECS => true,
            (_, true) => age > NEVER_SEEN_SECS,
            _ => false,
        }
    }
}

/// What the patrol pass knows about one cop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cop {
    /// [`available`] for its position.
    pub available: bool,
    /// The result of [`Latch::can_respawn`].
    pub can_respawn: bool,
    /// The car is wrecked.
    pub destroyed: bool,
    /// Seconds the car has been out of the camera's view.
    pub offscreen: f32,
    /// The cop is in a chase and within [`NEAR`] (or [`HELICOPTER_NEAR`]) of the point [`AHEAD`] its target.
    pub about_to_meet: bool,
}

/// Whether the patrol pass removes the cop: a wreck once it may respawn and has been off screen for
/// [`WRECK_OFFSCREEN_SECS`]; any other cop once it may respawn at an available spot, unless its chase is about to meet it.
pub fn should_remove(cop: &Cop) -> bool {
    if cop.destroyed {
        return cop.can_respawn && (cop.offscreen > WRECK_OFFSCREEN_SECS || cop.available);
    }
    cop.can_respawn && cop.available && !cop.about_to_meet
}

/// The point [`AHEAD`] metres along the target's travel (its velocity above 1 m/s, otherwise its facing), where a chase
/// measures how near a cop is.
pub fn ahead_point(position: Vec3, velocity: Vec3, forward: Vec3) -> Vec3 {
    let along = match velocity.length() > 1.0 {
        true => velocity,
        false => forward,
    };
    position + along.normalize_or_zero() * AHEAD
}

/// Whether a cop at `cop` is within the chase's near radius of `ahead` (the helicopter has a wider one).
pub fn near_target(cop: Vec3, ahead: Vec3, helicopter: bool) -> bool {
    let radius = match helicopter {
        true => HELICOPTER_NEAR,
        false => NEAR,
    };
    cop.distance(ahead) < radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_is_never_recycled_inside_the_minimum_and_always_beyond_the_middle() {
        assert!(!available(COP_WINDOW, 150.0, Some(-1.0)));
        assert!(!available(COP_WINDOW, 100.0, None));
        assert!(available(COP_WINDOW, 275.1, Some(1.0)), "far enough, even straight ahead");
        assert!(available(COP_WINDOW, 500.0, None));
    }

    #[test]
    fn between_the_minimum_and_the_middle_only_a_spot_outside_the_forward_cone_is_available() {
        assert!(!available(COP_WINDOW, 200.0, Some(0.95)), "straight ahead");
        assert!(!available(COP_WINDOW, 200.0, Some(0.81)));
        assert!(available(COP_WINDOW, 200.0, Some(0.8)), "the cone's edge");
        assert!(available(COP_WINDOW, 200.0, Some(0.0)), "to the side");
        assert!(available(COP_WINDOW, 200.0, Some(-0.707)), "behind, within the range");
        assert!(!available(COP_WINDOW, 200.0, Some(-0.9)), "directly behind is the other side of the cone");
        assert!(!available(COP_WINDOW, 200.0, None), "without a view there is no cone to be outside of");
    }

    #[test]
    fn a_cop_that_never_stood_in_play_waits_ten_seconds() {
        let mut latch = Latch::default();
        assert!(!latch.can_respawn(true, 9.0));
        assert!(!latch.can_respawn(true, 10.0));
        assert!(latch.can_respawn(true, 10.1));
    }

    #[test]
    fn a_cop_that_stood_in_play_may_go_after_eight_seconds_wherever_it_is() {
        let mut latch = Latch::default();
        assert!(!latch.can_respawn(false, 3.0), "in play and too young");
        assert!(
            latch.can_respawn(false, 8.5),
            "it stood in play and is old enough: the position rule decides the rest"
        );
        let mut fresh = Latch::default();
        assert!(!fresh.can_respawn(true, 8.5), "a cop that was never in play waits for 10 s");
        assert!(fresh.can_respawn(true, 10.5));
    }

    #[test]
    fn the_latch_remembers_a_visit_to_the_play_area() {
        let mut latch = Latch::default();
        assert!(!latch.can_respawn(false, 1.0));
        assert!(latch.can_respawn(true, 8.1), "after the visit 8 s suffice at an available spot");
    }

    fn cop() -> Cop {
        Cop { available: true, can_respawn: true, destroyed: false, offscreen: 0.0, about_to_meet: false }
    }

    #[test]
    fn a_cop_goes_when_it_may_respawn_at_an_available_spot() {
        assert!(should_remove(&cop()));
        assert!(!should_remove(&Cop { available: false, ..cop() }));
        assert!(!should_remove(&Cop { can_respawn: false, ..cop() }));
    }

    #[test]
    fn a_chase_keeps_the_cops_it_is_about_to_meet() {
        assert!(!should_remove(&Cop { about_to_meet: true, ..cop() }));
    }

    #[test]
    fn a_wreck_goes_once_it_has_been_off_screen_for_three_seconds() {
        let wreck = Cop { destroyed: true, available: false, ..cop() };
        assert!(!should_remove(&Cop { offscreen: 3.0, ..wreck }));
        assert!(should_remove(&Cop { offscreen: 3.1, ..wreck }));
        assert!(!should_remove(&Cop { offscreen: 30.0, can_respawn: false, ..wreck }), "it must be old enough first");
    }

    #[test]
    fn a_wreck_at_an_available_spot_goes_whether_or_not_it_is_in_view() {
        let wreck = Cop { destroyed: true, available: true, offscreen: 0.0, ..cop() };
        assert!(should_remove(&wreck), "the position rule applies to a wreck too");
        assert!(!should_remove(&Cop { about_to_meet: true, available: false, ..wreck }), "in play and in view");
    }

    #[test]
    fn the_near_point_is_seventy_five_metres_along_the_travel_or_the_facing() {
        let at = Vec3::new(10.0, 0.0, 0.0);
        let moving = ahead_point(at, Vec3::new(0.0, 0.0, 20.0), Vec3::X);
        assert!((moving - Vec3::new(10.0, 0.0, 75.0)).length() < 1e-4);
        let standing = ahead_point(at, Vec3::new(0.0, 0.0, 0.5), Vec3::X);
        assert!((standing - Vec3::new(85.0, 0.0, 0.0)).length() < 1e-4, "slower than 1 m/s: the facing");
        assert!(near_target(Vec3::new(10.0, 0.0, 440.0), moving, false), "365 m from the point");
        assert!(!near_target(Vec3::new(10.0, 0.0, 460.0), moving, false), "385 m from it");
        assert!(near_target(Vec3::new(10.0, 0.0, 600.0), moving, true), "the helicopter's wider radius");
    }
}
