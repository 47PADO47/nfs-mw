//! What a hit does to a car: the traffic shock, the accident of a hit by the player, and a cop's damage. The cop's
//! hit points, flipped timer and shock are the rules of `blackbox_pursuit::wreck`; this file reads the car and the hit.
//! Spec: `docs/specs/ai-pursuit-cop-cars.md` section 6.

use blackbox_pursuit::wreck::{Wreck, WreckTuning, is_flipped, zone_of};
use glam::Vec3;

use super::{AiCar, Role, SHOCK_FORCE, SHOCK_MIN_SPEED_CHANGE};

/// Seconds after a hit by the player's car that the car still counts as moved by it (the causality chain;
/// design value, unconfirmed).
const PLAYER_CAUSE: f32 = 2.0;

/// The damage state of a cop car: its numbers, the wreck, and whether a new wreck waits to be counted.
pub(super) struct CopDamage {
    tuning: WreckTuning,
    wreck: Wreck,
    /// A wreck happened and the world has not taken it yet (see [`AiCar::take_wreck`]).
    pending: bool,
    /// Seconds since the player's car last hit this car.
    since_player: f32,
}

impl CopDamage {
    pub(super) fn for_model(model: &str) -> Self {
        Self { tuning: tuning_for(model), wreck: Wreck::default(), pending: false, since_player: f32::INFINITY }
    }
}

/// The numbers of each cop car (`damagespecs`, spec section 6, [verified]). A car not in the table takes the base
/// cop numbers.
fn tuning_for(model: &str) -> WreckTuning {
    let (hit_points, threshold, suppress_dist, front_rear_scale) = match model.to_ascii_lowercase().as_str() {
        "copghost" => (6.0, 0.75, 35.0, 0.5),
        "copgto" => (7.0, 1.5, 35.0, 0.5),
        "copgtoghost" => (8.0, 1.5, 35.0, 0.5),
        "copsport" => (10.0, 1.5, 35.0, 0.5),
        "copsuv" | "copsuvpatrol" | "copsuvl" => (10.0, 2.0, 35.0, 0.5),
        "copcross" => (400.0, 10.0, 25.0, 1.0),
        _ => (5.0, 0.75, 35.0, 0.5),
    };
    WreckTuning { hit_points, threshold, suppress_dist, front_rear_scale }
}

impl AiCar {
    /// A hit of `impulse` N s at `point` (physics space), from `from_camera` metres away from the player: the traffic
    /// shock, the accident of a hit by the player, and a cop's damage. Returns whether the hit is the player's first that
    /// damages this cop (an assault, see `rules::contact_infraction`).
    pub(in crate::scenes::world::ai) fn on_hit(
        &mut self,
        impulse: f32,
        by_player: bool,
        point: Vec3,
        from_camera: f32,
    ) -> bool {
        let speed_change = impulse / self.mass.max(1.0);
        if speed_change > SHOCK_MIN_SPEED_CHANGE {
            self.shock = self.shock.max((speed_change / SHOCK_FORCE).min(1.0));
        }
        if by_player {
            log::info!("traffic: {} was hit by the player (impulse {impulse:.0} N s)", self.name);
            self.accident.on_hit(matches!(self.role, Role::Traffic) && !self.patrol, self.tractor);
        }
        self.wreck_hit(impulse, point, by_player, from_camera)
    }

    /// Whether the car is a cop that the damage has wrecked.
    pub fn is_wrecked(&self) -> bool {
        self.damage.as_ref().is_some_and(|d| d.wreck.is_destroyed())
    }

    /// Whether the car was wrecked since the last call. The world counts each wreck once.
    pub fn take_wreck(&mut self) -> bool {
        self.damage.as_mut().is_some_and(|d| std::mem::take(&mut d.pending))
    }

    /// The impact on a cop: the arm from the car's centre in its frame gives the zone, and the zone's scale and the
    /// impulse give the damage. A traffic car has no damage state and nothing happens. Returns whether this hit by the
    /// player is the first to damage the cop (its damage was zero before and is over zero after).
    pub(super) fn wreck_hit(&mut self, impulse: f32, point: Vec3, by_player: bool, from_camera: f32) -> bool {
        let Some(damage) = self.damage.as_mut() else { return false };
        // The contact's normal is not carried by the hit, so the zone is the front, rear or side (see `zone_of`).
        let view = self.sim.driver_view();
        let side = Vec3::Y.cross(view.forward).normalize_or_zero();
        let offset = point - view.position;
        let arm = Vec3::new(offset.dot(side), offset.dot(Vec3::Y), offset.dot(view.forward));
        let scale = zone_of(arm, self.dim, Vec3::ZERO).hit_point_scale(damage.tuning.front_rear_scale);
        if by_player {
            damage.since_player = 0.0;
        }
        let closing = impulse / self.mass.max(1.0);
        damage.wreck.set_shock(impulse, self.mass);
        let before = damage.wreck.damage();
        if damage.wreck.impact(&damage.tuning, impulse, closing, scale, from_camera) {
            damage.pending = true;
        }
        by_player && before <= 0.0 && damage.wreck.damage() > 0.0
    }

    /// One physics step after the car moved: the shock decays, the flipped check runs, and the time since the player
    /// last hit the car goes on. The car is flipped when not every wheel is down and its body is turned over.
    pub(super) fn wreck_step(&mut self, dt: f32) {
        let height = (self.current.rotation * Vec3::Z).z;
        let all_down = self.sim.telemetry().wheels_on_ground >= 4;
        let flipped = is_flipped(height, height, all_down);
        let Some(damage) = self.damage.as_mut() else { return };
        damage.wreck.step_shock(dt);
        damage.since_player += dt;
        let by_player = damage.since_player <= PLAYER_CAUSE;
        if damage.wreck.flip(dt, flipped, by_player) {
            damage.pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tuning_for;

    #[test]
    fn the_cop_sets_have_the_spec_numbers() {
        // The base set, and the named sets of section 6 (`damagespecs`).
        let base = tuning_for("copmidsize");
        assert_eq!((base.hit_points, base.threshold, base.suppress_dist), (5.0, 0.75, 35.0));
        let suv = tuning_for("CopSUVPatrol");
        assert_eq!((suv.hit_points, suv.threshold), (10.0, 2.0));
        let cross = tuning_for("copcross");
        assert_eq!((cross.hit_points, cross.threshold, cross.suppress_dist), (400.0, 10.0, 25.0));
        assert_eq!(cross.front_rear_scale, 1.0);
        assert_eq!(base.front_rear_scale, 0.5);
    }
}
