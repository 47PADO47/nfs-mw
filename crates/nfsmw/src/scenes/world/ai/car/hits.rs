//! What a hit does to a car: the traffic shock, the accident of a hit by the player, and a cop's damage. The cop's
//! hit points, flipped timer and shock are the rules of `blackbox_pursuit::wreck`; this file reads the car and the hit.
//! Spec: `docs/specs/ai-pursuit-cop-cars.md` section 6.

use blackbox_pursuit::health_bar::Sample;
use blackbox_pursuit::wreck::{Wreck, WreckTuning, is_flipped, zone_of};
use glam::Vec3;

use super::{AiCar, Role, SHOCK_FORCE, SHOCK_MIN_SPEED_CHANGE};
use crate::scenes::world::ai::collide::DriverClass;

/// Seconds after a hit by the player's car that the car still counts as moved by it (the causality chain;
/// design value, unconfirmed).
const PLAYER_CAUSE: f32 = 2.0;
/// The camera distance (metres) within which a hit by the player starts a crash (the original's view gate:
/// `ai-traffic-obstacles.md` §5.1).
const CRASH_VIEW: f32 = 80.0;
/// 5 mph in metres per second: a vehicle at this speed or faster starts a crash again when the car is already over.
const CRASH_MIN_SPEED: f32 = 2.235_15;

/// Whether a hit starts a crash (`ai-traffic-obstacles.md` section 5.1): the party that hit the car is a human, a cop
/// or a racer (classes 0, 2 and 3), not traffic, and the hit is within the view distance.
pub(super) fn starts_crash(striker: DriverClass, from_camera: f32) -> bool {
    striker != DriverClass::Traffic && from_camera <= CRASH_VIEW
}

/// Whether a car at `speed` (m/s, either sign) is moving fast enough to start a crash that is over.
pub(super) fn is_fast(speed: f32) -> bool {
    speed.abs() >= CRASH_MIN_SPEED
}

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

/// The health sample of a car: only a cop (`Role::Cop`, wrecked or not) gets one, with its damage and destruction.
fn cop_sample(is_cop: bool, key: u32, position: Vec3, damage: f32, destroyed: bool) -> Option<Sample> {
    if !is_cop {
        return None;
    }
    Some(Sample { key, position, damage, destroyed })
}

impl AiCar {
    pub fn is_damaged_cop(&self) -> bool {
        self.is_police() && self.damage.as_ref().is_some_and(|d| d.wreck.damage() > 0.0)
    }
    /// A hit of `impulse` N s at `point` (physics space), from `from_camera` metres away from the player: the traffic
    /// shock, the accident of a hit by a human, cop or racer, and a cop's damage. `striker` and `speed` (m/s) are the
    /// class and the speed of the party that hit the car; the player's speed is the one the world sets. Returns whether
    /// the hit is the player's first that damages this cop (an assault, see `rules::contact_infraction`).
    pub(in crate::scenes::world::ai) fn on_hit(
        &mut self,
        impulse: f32,
        by_player: bool,
        striker: DriverClass,
        speed: f32,
        point: Vec3,
        from_camera: f32,
    ) -> bool {
        let speed_change = impulse / self.mass.max(1.0);
        if speed_change > SHOCK_MIN_SPEED_CHANGE {
            self.shock = self.shock.max((speed_change / SHOCK_FORCE).min(1.0));
        }
        if starts_crash(striker, from_camera) {
            log::info!("traffic: {} was hit by a {striker:?} (impulse {impulse:.0} N s)", self.name);
            let hitter_speed = match by_player {
                true => self.player_speed,
                false => speed,
            };
            let fast = is_fast(hitter_speed);
            self.accident.on_hit(matches!(self.role, Role::Traffic) && !self.patrol, self.tractor, fast);
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

    /// The driver class of this car (see `DriverClass`): a cop, a patrol car or a roadblock car is a cop, else traffic.
    pub(in crate::scenes::world::ai) fn driver_class(&self) -> DriverClass {
        match matches!(self.role, Role::Cop(_)) || self.patrol || self.roadblock {
            true => DriverClass::Cop,
            false => DriverClass::Traffic,
        }
    }

    /// The car as a cop health bar sees it (`docs/specs/cop-health-bars.md` section 13.2). The key is the car's render
    /// identity, which is stable for the car's life. `None` for traffic, patrol and roadblock cars.
    pub(in crate::scenes::world::ai) fn cop_health_sample(&self) -> Option<Sample> {
        let damage = self.damage.as_ref()?;
        let is_cop = matches!(self.role, Role::Cop(_));
        cop_sample(is_cop, self.render_id, self.position(), damage.wreck.damage(), damage.wreck.is_destroyed())
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
    use super::{cop_sample, is_fast, starts_crash, tuning_for};
    use crate::scenes::world::ai::collide::DriverClass;
    use glam::Vec3;

    #[test]
    fn a_cop_gets_a_health_sample_wrecked_or_not_and_traffic_gets_none() {
        let at = Vec3::new(1.0, 2.0, 3.0);
        let live = cop_sample(true, 7, at, 0.2, false).unwrap();
        assert_eq!((live.key, live.position, live.damage, live.destroyed), (7, at, 0.2, false));
        let wrecked = cop_sample(true, 7, at, 0.4, true).unwrap();
        assert!(wrecked.destroyed, "a wrecked cop keeps its bar");
        assert!(cop_sample(false, 7, at, 0.2, false).is_none(), "traffic, patrol and roadblock cars get none");
    }

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

    #[test]
    fn a_crash_needs_a_hit_by_a_human_cop_or_racer_within_eighty_metres() {
        assert!(starts_crash(DriverClass::Human, 80.0));
        assert!(!starts_crash(DriverClass::Human, 80.5), "beyond the view gate");
        assert!(starts_crash(DriverClass::Cop, 40.0), "an AI cop's hit starts it too");
        assert!(!starts_crash(DriverClass::Traffic, 10.0), "traffic against traffic does not");
    }

    #[test]
    fn only_traffic_is_left_out_of_the_crash_gate() {
        let crashes: Vec<bool> = [DriverClass::Human, DriverClass::Cop, DriverClass::Traffic]
            .into_iter()
            .map(|class| starts_crash(class, 0.0))
            .collect();
        assert_eq!(crashes, [true, true, false]);
    }

    #[test]
    fn the_restart_speed_is_five_miles_an_hour() {
        assert!(!is_fast(2.2));
        assert!(is_fast(2.24));
        assert!(is_fast(-3.0), "reverse counts by its size");
    }
}
