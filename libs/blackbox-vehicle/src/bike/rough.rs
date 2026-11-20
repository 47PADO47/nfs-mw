//! What a bike knows about the ground it rides on (`docs/specs/ride6-bike-rough-ground.md`): the arcade help on loose ground reads the grip
//! multipliers the host's ground gives each tyre, takes the penalty of a bike made for rough ground down, and remembers the result so the lean
//! controller and the cornering assist follow it.

use crate::bike::class::TerrainClass;
use crate::bike::{ArcadeRough, Bike};
use crate::ground::SurfaceGrip;

/// The grip a tyre gets on `grip` for a bike of class `class` and how loose that surface is (0 to 1). In the reference steering the grip is
/// returned as it came, bit for bit.
pub(in crate::bike) fn feel(rough: &ArcadeRough, class: TerrainClass, grip: SurfaceGrip) -> (SurfaceGrip, f32) {
    let index = class.index();
    let relieved =
        SurfaceGrip { lateral: rough.relieved(index, grip.lateral), drive: rough.relieved(index, grip.drive), ..grip };
    (relieved, rough.looseness(grip.lateral))
}

/// What the bike believes about the surface: the lateral grip multiplier it rides on (after the class's relief) and how loose it is, both
/// followed at `ArcadeRough::surface_rate` so a change of ground is met progressively, and what the two wheels saw in this sub-step.
#[derive(Clone, Copy, Debug)]
pub struct RoughState {
    surface: f32,
    loose: f32,
    seen: [Option<(f32, f32)>; 2],
}

impl Default for RoughState {
    /// Tarmac: full grip, nothing loose.
    fn default() -> Self {
        Self { surface: 1.0, loose: 0.0, seen: [None; 2] }
    }
}

impl RoughState {
    /// The lateral grip multiplier the bike believes in (1 on tarmac).
    pub fn surface(&self) -> f32 {
        self.surface
    }

    /// How loose the ground the bike believes it is on is (0 to 1).
    pub fn loose(&self) -> f32 {
        self.loose
    }

    /// Wheel `wheel` touched a surface of lateral multiplier `lateral` (after the relief) and looseness `looseness` in this sub-step.
    pub fn see(&mut self, wheel: usize, lateral: f32, looseness: f32) {
        self.seen[wheel.min(1)] = Some((lateral, looseness));
    }

    /// The sub-step is over: what the wheels on the ground saw (their mean) is followed at `rate` per second; with both in the air the belief
    /// is kept.
    pub fn advance(&mut self, rate: f32, dt: f32) {
        let seen = std::mem::take(&mut self.seen);
        let count = seen.iter().flatten().count();
        if count == 0 {
            return;
        }
        let (lateral, looseness) =
            seen.iter().flatten().fold((0.0, 0.0), |sum, wheel| (sum.0 + wheel.0, sum.1 + wheel.1));
        let share = (rate * dt).clamp(0.0, 1.0);
        let count = count as f32;
        self.surface += (lateral / count - self.surface) * share;
        self.loose += (looseness / count - self.loose) * share;
    }
}

impl Bike {
    /// The lateral grip multiplier the lean controller and the cornering assist plan with: the surface the bike believes it is on, or 1 when
    /// the steering does not follow the surface (the reference's).
    pub(in crate::bike) fn surface_factor(&self) -> f32 {
        match self.spec.steering.arcade.rough.follow_surface {
            true => self.chassis.rough.surface(),
            false => 1.0,
        }
    }

    /// The factor on the suspension's damping now (1 on tarmac and in the reference steering).
    pub(in crate::bike) fn damping_factor(&self) -> f32 {
        let class = self.spec.terrain_class();
        self.spec.steering.arcade.rough.damping_factor(class.index(), self.chassis.rough.loose())
    }

    /// The seconds a lean must stay beyond its fall limit to count, over the data's duration (0 in the reference steering).
    pub(in crate::bike) fn lean_hold(&self) -> f32 {
        let class = self.spec.terrain_class();
        self.spec.steering.arcade.rough.hold(class.index(), self.chassis.rough.loose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_belief_follows_the_wheels_on_the_ground_progressively_and_is_kept_in_the_air() {
        let mut state = RoughState::default();
        assert_eq!((state.surface(), state.loose()), (1.0, 0.0));
        state.advance(5.0, 1.0 / 60.0);
        assert_eq!(state.surface(), 1.0, "nothing seen: nothing changes");
        for _ in 0..3 {
            state.see(0, 0.6, 1.0);
            state.see(1, 0.8, 0.5);
            state.advance(5.0, 1.0 / 60.0);
        }
        assert!(state.surface() < 1.0 && state.surface() > 0.7, "{}", state.surface());
        assert!(state.loose() > 0.0 && state.loose() < 0.75);
        let kept = state;
        state.advance(5.0, 1.0 / 60.0);
        assert_eq!((state.surface(), state.loose()), (kept.surface(), kept.loose()));
        for _ in 0..600 {
            state.see(0, 0.6, 1.0);
            state.see(1, 0.8, 0.5);
            state.advance(5.0, 1.0 / 60.0);
        }
        assert!((state.surface() - 0.7).abs() < 1e-3 && (state.loose() - 0.75).abs() < 1e-3);
    }

    #[test]
    fn the_reference_hands_the_grip_back_bit_for_bit() {
        let grip = SurfaceGrip { lateral: 0.63, drive: 0.71, rolling: 2.0, tag: 7 };
        let (out, looseness) = feel(&ArcadeRough::REFERENCE, TerrainClass::Offroad, grip);
        assert_eq!((out.lateral, out.drive, out.rolling, out.tag), (grip.lateral, grip.drive, grip.rolling, 7));
        assert!(looseness > 0.0, "it still says how loose the surface is");
    }

    #[test]
    fn an_offroad_bike_keeps_more_of_a_loose_surface_than_a_road_bike() {
        let grip = SurfaceGrip { lateral: 0.6, drive: 0.6, rolling: 1.0, tag: 0 };
        let rough = ArcadeRough::default();
        let road = feel(&rough, TerrainClass::Road, grip).0;
        let motard = feel(&rough, TerrainClass::Motard, grip).0;
        let offroad = feel(&rough, TerrainClass::Offroad, grip).0;
        assert!(road.lateral > 0.6 && road.lateral < motard.lateral, "a road bike loses half of the penalty");
        assert!(motard.lateral > road.lateral && offroad.lateral > motard.lateral);
        assert!(offroad.drive > road.drive);
    }
}
