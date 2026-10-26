//! What a hit does to a car: the traffic shock, the accident of a hit by the player, and a cop's damage. The cop's
//! hit points, flipped timer and shock are the rules of `blackbox_pursuit::wreck`; this file reads the car and the hit.
//! Spec: `docs/specs/ai-pursuit-cop-cars.md` section 6.

use blackbox_pursuit::health_bar::Sample;
use blackbox_pursuit::wreck::{Wreck, WreckTuning, is_flipped};
use glam::Vec3;

use super::{AiCar, Role, SHOCK_FORCE, SHOCK_MIN_SPEED_CHANGE};
use crate::scenes::world::ai::collide::DriverClass;
use crate::scenes::world::events::DamageHit;
use nfsmw_data::car::physics::CarEventSpec;

/// Seconds after a hit by the player's car that the car still counts as moved by it (the causality chain;
/// design value, unconfirmed).
const PLAYER_CAUSE: f32 = 2.0;
/// The camera distance (metres) within which a hit by the player starts a crash (the original's view gate:
/// `ai-traffic-obstacles.md` Â§5.1).
const CRASH_VIEW: f32 = 80.0;
/// 5 mph in metres per second: a vehicle at this speed or faster starts a crash again when the car is already over.
const CRASH_MIN_SPEED: f32 = 2.235_15;

#[cfg(test)]
mod installed_tests;

/// Whether a hit starts a crash (`ai-traffic-obstacles.md` section 5.1): the party that hit the car is a human, a cop
/// or a racer (classes 0, 2 and 3), not traffic, and the hit is within the view distance.
pub(super) fn starts_crash(striker: DriverClass, from_view: f32) -> bool {
    striker != DriverClass::Traffic && from_view <= CRASH_VIEW
}

/// Whether a car at `speed` (m/s, either sign) is moving fast enough to start a crash that is over.
pub(super) fn is_fast(speed: f32) -> bool {
    speed.abs() >= CRASH_MIN_SPEED
}

/// The damage state of a cop car: its numbers, the wreck, and whether a new wreck waits to be counted.
pub(super) struct CopDamage {
    tuning: WreckTuning,
    scales: [f32; 10],
    shock_force: f32,
    shock_time: f32,
    wreck: Wreck,
    /// A wreck happened and the world has not taken it yet (see [`AiCar::take_wreck`]).
    pending: bool,
    /// Seconds since the player's car last hit this car.
    since_player: f32,
}

impl CopDamage {
    pub(super) fn for_model(model: &str, spec: Option<&CarEventSpec>) -> Self {
        // A mod cop has no damage record: the damage numbers of its unit class stand in (`car/mods/cop.rs`). The racing
        // numbers its physics carry have no hit points, so they are not read for it.
        let class = nfsmw_data::car::mods::cop_kind(model);
        let mut tuning = class.map_or_else(|| tuning_for(model), class_tuning);
        let mut scales = [1.0; 10];
        scales[0] = tuning.front_rear_scale;
        scales[1] = tuning.front_rear_scale;
        scales[8] = 0.5;
        scales[9] = 0.0;
        if let Some(spec) = spec.filter(|_| class.is_none()) {
            tuning.hit_points = spec.hit_points;
            tuning.threshold = spec.hit_point_threshold;
            tuning.suppress_dist = spec.suppress_distance;
            tuning.strength = spec.strength;
            scales = spec.hit_point_scale;
        }
        Self {
            tuning,
            scales,
            shock_force: spec.map_or(40.0, |s| s.shock_force),
            shock_time: spec.map_or(3.0, |s| s.shock_time),
            wreck: Wreck::default(),
            pending: false,
            since_player: f32::INFINITY,
        }
    }
}

/// The generic damage numbers of a mod cop's unit class (`CopKind::damage`).
fn class_tuning(kind: nfsmw_data::car::mods::CopKind) -> WreckTuning {
    let damage = kind.damage();
    WreckTuning {
        hit_points: damage.hit_points,
        threshold: damage.threshold,
        suppress_dist: damage.suppress_distance,
        front_rear_scale: damage.front_rear_scale,
        strength: 20_000.0,
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
    WreckTuning { hit_points, threshold, suppress_dist, front_rear_scale, strength: 20_000.0 }
}

/// The health sample of a car: only a cop (`Role::Cop`, wrecked or not) gets one, with its damage and destruction.
fn cop_sample(is_cop: bool, key: u32, position: Vec3, damage: f32, destroyed: bool) -> Option<Sample> {
    if !is_cop {
        return None;
    }
    Some(Sample { key, position, damage, destroyed })
}

impl AiCar {
    pub(super) fn damage_health(&self) -> Option<f32> {
        let damage = self.damage.as_ref()?;
        if damage.wreck.is_destroyed() {
            return Some(0.0);
        }
        Some((1.0 - damage.wreck.damage()).clamp(0.0, 1.0))
    }
    pub fn is_damaged_cop(&self) -> bool {
        self.is_police() && self.damage.as_ref().is_some_and(|d| d.wreck.damage() > 0.0)
    }
    /// A hit of `impulse` N s at `point` (physics space), from `from_view` metres from the nearest player's view: the traffic
    /// shock, the accident of a hit by a human, cop or racer, and a cop's damage. `striker` and `speed` (m/s) are the
    /// class and the speed of the party that hit the car; the player's speed is the one the world sets. Returns whether
    /// the hit is the player's first that damages this cop (an assault, see `rules::contact_infraction`).
    pub(in crate::scenes::world::ai) fn on_hit(
        &mut self,
        impulse: f32,
        by_player: bool,
        striker: DriverClass,
        speed: f32,
        hit: DamageHit,
        from_view: f32,
    ) -> bool {
        let speed_change = impulse / self.mass.max(1.0);
        if speed_change > SHOCK_MIN_SPEED_CHANGE {
            self.shock = self.shock.max((speed_change / SHOCK_FORCE).min(1.0));
        }
        if starts_crash(striker, from_view) {
            log::info!("traffic: {} was hit by a {striker:?} (impulse {impulse:.0} N s)", self.name);
            let hitter_speed = match by_player {
                true => self.player_speed,
                false => speed,
            };
            let fast = is_fast(hitter_speed);
            self.accident.on_hit(matches!(self.role, Role::Traffic) && !self.patrol, self.tractor, fast);
        }
        self.wreck_hit(hit, from_view)
    }

    /// Whether the car is a cop that the damage has wrecked.
    pub fn is_wrecked(&self) -> bool {
        self.damage.as_ref().is_some_and(|d| d.wreck.is_destroyed())
    }

    /// A pursuit breaker zone destroys the car (`docs/specs/pursuit-breakers.md`, "Cop zones"). Only a police car is
    /// destroyed; the wreck is counted once, through [`AiCar::take_wreck`], like a wreck from damage. Returns whether
    /// the car was newly wrecked.
    pub(in crate::scenes::world::ai) fn destroy_by_breaker(&mut self) -> bool {
        if !self.is_police() {
            return false;
        }
        let name = self.name.clone();
        let damage =
            self.damage.get_or_insert_with(|| CopDamage::for_model(&name, self.sim.events.as_ref().map(|e| e.spec())));
        if !damage.wreck.destroy() {
            return false;
        }
        damage.pending = true;
        if let Some(events) = self.sim.events.as_mut() {
            events.destroy();
        }
        true
    }

    /// The damage a cop car has taken (0 for none and for traffic), as its wreck state counts it.
    pub fn cop_damage(&self) -> f32 {
        self.damage.as_ref().map_or(0.0, |d| d.wreck.damage())
    }

    /// Whether the car was wrecked since the last call. The world counts each wreck once.
    pub fn take_wreck(&mut self) -> bool {
        self.damage.as_mut().is_some_and(|d| std::mem::take(&mut d.pending))
    }

    /// Health consumes the same resolved callback as the visual sequencer, exactly once.
    pub(super) fn wreck_hit(&mut self, hit: DamageHit, from_view: f32) -> bool {
        let Some(damage) = self.damage.as_mut() else { return false };
        if hit.closing_speed < blackbox_pursuit::wreck::MIN_CLOSING {
            return false;
        }
        if damage.shock_time > 0.0 && hit.shock_force > 0.0 {
            damage.wreck.set_shock_scaled(hit.shock_force, self.mass, damage.shock_force);
        }
        if !hit.allows_cop() {
            return false;
        }
        let scale = damage.scales[hit.zone()];
        if hit.other.player {
            damage.since_player = 0.0;
        }
        let before = damage.wreck.damage();
        let was_destroyed = damage.wreck.is_destroyed();
        if damage.wreck.impact(&damage.tuning, hit.force, hit.closing_speed, scale, from_view) {
            damage.pending = true;
        }
        if !was_destroyed
            && damage.wreck.is_destroyed()
            && let Some(events) = self.sim.events.as_mut()
        {
            events.note_health_destroyed();
        }
        if damage.wreck.damage() != before {
            log::info!(
                "cop damage {}: zone {}, {:.3} -> {:.3}, force {:.0}, closing {:.1}, disabled={}",
                self.name,
                hit.zone(),
                before,
                damage.wreck.damage(),
                hit.force,
                hit.closing_speed,
                damage.wreck.is_destroyed()
            );
        }
        hit.other.player && before <= 0.0 && damage.wreck.damage() > 0.0
    }

    pub(super) fn wreck_brake(&self) -> f32 {
        self.damage.as_ref().map_or(0.0, |d| {
            let v = self.sim.velocity();
            d.wreck.retirement_brake(v.x.hypot(v.z))
        })
    }

    pub(super) fn damage_status(&self) -> String {
        self.damage.as_ref().map_or_else(String::new, |d| {
            format!(
                "; health {:.0}%{}",
                (1.0 - d.wreck.damage()).clamp(0.0, 1.0) * 100.0,
                if self.is_wrecked() { ", disabled" } else { "" }
            )
        })
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
        let up = self.sim.up();
        let height = up.y;
        let all_down = self.sim.telemetry().wheels_on_ground >= 4;
        let flipped = is_flipped(up.dot(self.sim.ground_normal()), height, all_down);
        let Some(damage) = self.damage.as_mut() else { return };
        damage.wreck.step_shock_scaled(dt, damage.shock_time);
        damage.since_player += dt;
        let by_player = damage.since_player <= PLAYER_CAUSE;
        if damage.wreck.flip(dt, flipped, by_player) {
            damage.pending = true;
            if let Some(events) = &mut self.sim.events {
                events.destroy();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{class_tuning, cop_sample, is_fast, starts_crash, tuning_for};
    use crate::scenes::world::ai::collide::DriverClass;
    use glam::Vec3;

    #[test]
    fn a_mod_cops_class_gives_it_hit_points_so_it_can_be_wrecked() {
        use nfsmw_data::car::mods::CopKind;
        let patrol = class_tuning(CopKind::Patrol);
        let heavy = class_tuning(CopKind::Heavy);
        assert!(patrol.hit_points > 0.0, "the racing numbers of a mod car have no hit points: a cop would be immortal");
        assert!(heavy.hit_points > patrol.hit_points && heavy.threshold > patrol.threshold);
        assert_eq!(patrol.suppress_dist, 35.0);
    }

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
