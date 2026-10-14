//! The damage of a cop car as pure rules: the hit points that impacts wear down, the zone an impact lands in, the
//! flipped timer that wrecks an overturned car, and the shock a hard hit leaves. Nothing here reads a world or a clock:
//! the caller measures each impact (its impulse, where it landed, how far the cop is from the camera), ticks the
//! flipped check and applies the outcome. The numbers of each car come from the caller's [`WreckTuning`].
//!
//! Spec: `docs/specs/ai-pursuit-cop-cars.md` section 6 (decomp: `DamageVehicle::OnImpact`, `DamageCopCar`, the
//! `damagespecs` table, [verified] for the per-car numbers).

use glam::Vec3;

/// The force scale of the damage ratio: `ratio = force / (FORCE_SCALE * 1000)` (`FORCE` 20 for every cop set).
pub const FORCE_SCALE: f32 = 20.0;
/// Impacts closing slower than this (m/s) do nothing.
pub const MIN_CLOSING: f32 = 1.0;
/// The other body of a car-to-car or car-to-player impact is a vehicle, so the force is doubled.
pub const VEHICLE_OTHER: f32 = 2.0;
/// Seconds between two flipped checks.
pub const FLIP_CHECK: f32 = 0.1;
/// Seconds a cop may stay flipped before it is wrecked.
pub const FLIP_TIME: f32 = 6.0;
/// The shock's decay time (`SHOCK_TIME`, 3 s for every cop set).
pub const SHOCK_TIME: f32 = 3.0;
/// The shock force that scales to 1 (`SHOCK_FORCE`, 40 for every cop set).
pub const SHOCK_FORCE: f32 = 40.0;
/// A shock scaled to this or less does nothing.
const SHOCK_MIN: f32 = 0.2;
/// Slack for the float sums of the damage and the flipped time, so that five fifths make one.
const DAMAGE_EPS: f32 = 1e-4;
/// The part of a unit contact normal that picks a corner or a top or bottom (`0.8`).
const NORMAL_SIDE: f32 = 0.8;
/// The fraction of the half extents an arm must pass to leave the centre zone (`k = 0.5`).
const EDGE: f32 = 0.5;

/// The per-car numbers of the damage model (`damagespecs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WreckTuning {
    /// `HIT_POINTS`: the damage the car takes before it is wrecked, in impact units (`damage` reaches 1).
    pub hit_points: f32,
    /// `HP_THRESHOLD`: a hit whose scaled force ratio is not over this does nothing.
    pub threshold: f32,
    /// `SUPPRESS_DIST` (metres): impacts farther than this from the camera do nothing. Zero keeps every impact.
    pub suppress_dist: f32,
    /// The `HitPointScale` of the front and rear zones: 0.5 for the cop sets, 1.0 for `copcross`.
    pub front_rear_scale: f32,
}

/// The zone of the car an impact landed in (`DamageVehicle`'s zones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Front,
    Rear,
    Left,
    Right,
    FrontLeft,
    FrontRight,
    RearLeft,
    RearRight,
    Top,
    Bottom,
}

impl Zone {
    /// The `HitPointScale` of the zone for the cop sets: the sides and corners 1, the front and rear as the tuning
    /// gives, the top 0.5 and the bottom 0.
    pub fn hit_point_scale(self, front_rear: f32) -> f32 {
        match self {
            Zone::Front | Zone::Rear => front_rear,
            Zone::Top => 0.5,
            Zone::Bottom => 0.0,
            _ => 1.0,
        }
    }
}

/// The zone of an impact at `arm`, the contact point relative to the car's centre, with the car's `half` extents and
/// the contact `normal`. All three are in the car's frame: x to the side, y up, z forward. A zero normal (the direction
/// of the contact is not known) never picks a corner, a top or a bottom: the impact falls to the front, rear or side.
pub fn zone_of(arm: Vec3, half: Vec3, normal: Vec3) -> Zone {
    let left = arm.x < 0.0;
    let corner = arm.x.abs() > EDGE * half.x && normal.x.abs() > NORMAL_SIDE;
    if let Some(front) = end_of(arm.z, half.z) {
        return end_zone(front, left, corner);
    }
    if arm.y > EDGE * half.y && normal.y < -NORMAL_SIDE {
        return Zone::Top;
    }
    if arm.y < -EDGE * half.y && normal.y > NORMAL_SIDE {
        return Zone::Bottom;
    }
    match left {
        true => Zone::Left,
        false => Zone::Right,
    }
}

/// Which end the arm is past along the car, `Some(true)` for the front, `Some(false)` for the rear, `None` in the middle.
fn end_of(z: f32, half_z: f32) -> Option<bool> {
    match (z > EDGE * half_z, z < -EDGE * half_z) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// The front or rear zone, or the corner of it on the side the impact landed.
fn end_zone(front: bool, left: bool, corner: bool) -> Zone {
    match (front, left, corner) {
        (true, true, true) => Zone::FrontLeft,
        (true, false, true) => Zone::FrontRight,
        (false, true, true) => Zone::RearLeft,
        (false, false, true) => Zone::RearRight,
        (true, _, false) => Zone::Front,
        (false, _, false) => Zone::Rear,
    }
}

/// Whether a car is flipped: not every wheel on the ground, and the body turned over against the ground (the dot of
/// its up with the ground normal under 0.5, and its up's height under 0.5).
pub fn is_flipped(up_dot_ground: f32, up_y: f32, all_wheels_down: bool) -> bool {
    !all_wheels_down && up_dot_ground < 0.5 && up_y < 0.5
}

/// The damage and shock of one cop car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wreck {
    /// The damage taken, 0 at the start; the car is wrecked once it is over 1.
    damage: f32,
    /// Seconds flipped so far, counted per flipped check.
    flipped: f32,
    /// Seconds until the next flipped check.
    check: f32,
    /// The shock, 0 to 1 (`SetShockForce`), decaying over [`SHOCK_TIME`].
    shock: f32,
    /// The car is wrecked: it stays so for the rest of the chase.
    destroyed: bool,
}

impl Default for Wreck {
    fn default() -> Self {
        Self { damage: 0.0, flipped: 0.0, check: FLIP_CHECK, shock: 0.0, destroyed: false }
    }
}

impl Wreck {
    /// The damage taken so far, 0 to 1 and beyond once wrecked.
    pub fn damage(&self) -> f32 {
        self.damage
    }

    /// Whether the car is wrecked.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Whether the car is in shock: a hard hit is still being felt.
    pub fn in_shock(&self) -> bool {
        self.shock > 0.0
    }

    /// An impact of `impulse` (N s) at a closing speed of `closing` (m/s), in a zone whose hit point scale is
    /// `zone_scale`, from `from_camera` metres away. Returns `true` on the impact that wrecks the car. The force is
    /// the impulse doubled (the other body is a vehicle); a ratio not over the threshold does nothing, and the damage
    /// adds `ratio * zone_scale / hit_points`.
    pub fn impact(
        &mut self,
        tuning: &WreckTuning,
        impulse: f32,
        closing: f32,
        zone_scale: f32,
        from_camera: f32,
    ) -> bool {
        if self.destroyed || closing < MIN_CLOSING {
            return false;
        }
        if tuning.suppress_dist > 0.0 && from_camera > tuning.suppress_dist {
            return false;
        }
        let ratio = impulse * VEHICLE_OTHER / (FORCE_SCALE * 1000.0);
        let scaled = ratio * zone_scale;
        if scaled <= tuning.threshold {
            return false;
        }
        self.damage += scaled / tuning.hit_points.max(f32::EPSILON);
        if self.damage > 1.0 + DAMAGE_EPS {
            self.destroyed = true;
            return true;
        }
        false
    }

    /// One physics step of the flipped check: `dt` seconds have passed; `flipped_now` is [`is_flipped`] this step, and
    /// `by_player` whether the player's car last moved the car. A flipped car is wrecked at once when the player did it,
    /// else after [`FLIP_TIME`] of flipped checks. Returns `true` on the step that wrecks it.
    pub fn flip(&mut self, dt: f32, flipped_now: bool, by_player: bool) -> bool {
        if self.destroyed {
            return false;
        }
        self.check -= dt;
        if self.check > 0.0 {
            return false;
        }
        self.check += FLIP_CHECK;
        self.flipped = match flipped_now {
            true => self.flipped + FLIP_CHECK,
            false => 0.0,
        };
        if (flipped_now && by_player) || self.flipped + DAMAGE_EPS >= FLIP_TIME {
            self.destroyed = true;
            return true;
        }
        false
    }

    /// `SetShockForce(force)` for a car of `mass` kg: the shock becomes the force's scale (force over mass over
    /// [`SHOCK_FORCE`]) when that is over [`SHOCK_MIN`], as the larger of the shock and the scale, at most 1.
    pub fn set_shock(&mut self, force: f32, mass: f32) {
        let scale = force / mass.max(1.0) / SHOCK_FORCE;
        if scale <= SHOCK_MIN {
            return;
        }
        self.shock = self.shock.max(scale).min(1.0);
    }

    /// Decays the shock by `dt` seconds.
    pub fn step_shock(&mut self, dt: f32) {
        self.shock = (self.shock - dt / SHOCK_TIME).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cop set of the base numbers (`copmidsize`): hit points 5, threshold 0.75, suppressed beyond 35 m.
    const MID: WreckTuning =
        WreckTuning { hit_points: 5.0, threshold: 0.75, suppress_dist: 35.0, front_rear_scale: 0.5 };
    const HALF: Vec3 = Vec3::new(1.0, 0.7, 2.2);

    /// The impulse (N s) that gives a side hit a ratio of exactly one: `20000 / 2` (the doubling of the force).
    const ONE: f32 = 10_000.0;

    #[test]
    fn the_zone_is_the_end_or_the_side_of_the_arm() {
        assert_eq!(zone_of(Vec3::new(0.0, 0.0, 2.0), HALF, Vec3::ZERO), Zone::Front);
        assert_eq!(zone_of(Vec3::new(0.0, 0.0, -2.0), HALF, Vec3::ZERO), Zone::Rear);
        assert_eq!(zone_of(Vec3::new(-0.9, 0.0, 0.0), HALF, Vec3::ZERO), Zone::Left);
        assert_eq!(zone_of(Vec3::new(0.9, 0.0, 0.0), HALF, Vec3::ZERO), Zone::Right);
    }

    #[test]
    fn a_corner_needs_the_arm_out_to_the_side_and_the_normal_across_it() {
        let across = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(zone_of(Vec3::new(-0.9, 0.0, 2.0), HALF, across), Zone::FrontLeft);
        assert_eq!(zone_of(Vec3::new(0.9, 0.0, -2.0), HALF, across), Zone::RearRight);
        // Without the normal's side the same arm is the plain front.
        assert_eq!(zone_of(Vec3::new(-0.9, 0.0, 2.0), HALF, Vec3::ZERO), Zone::Front);
    }

    #[test]
    fn a_top_or_bottom_needs_the_arm_high_or_low_and_the_normal_down_or_up() {
        let down = Vec3::new(0.0, -1.0, 0.0);
        assert_eq!(zone_of(Vec3::new(0.0, 0.6, 0.0), HALF, down), Zone::Top);
        assert_eq!(zone_of(Vec3::new(0.0, -0.6, 0.0), HALF, -down), Zone::Bottom);
        // A zero normal never picks the top.
        assert_eq!(zone_of(Vec3::new(0.0, 0.6, 0.0), HALF, Vec3::ZERO), Zone::Right);
    }

    #[test]
    fn the_hit_point_scales_follow_the_cop_sets() {
        assert_eq!(Zone::Front.hit_point_scale(0.5), 0.5);
        assert_eq!(Zone::Rear.hit_point_scale(1.0), 1.0);
        assert_eq!(Zone::Left.hit_point_scale(0.5), 1.0);
        assert_eq!(Zone::FrontLeft.hit_point_scale(0.5), 1.0);
        assert_eq!(Zone::Top.hit_point_scale(0.5), 0.5);
        assert_eq!(Zone::Bottom.hit_point_scale(0.5), 0.0);
    }

    #[test]
    fn a_hit_under_the_threshold_does_nothing() {
        let mut w = Wreck::default();
        // An impulse of 3750 N s is a ratio of 0.375 on the doubled force: under the 0.75 threshold.
        assert!(!w.impact(&MID, 3_750.0, 5.0, 1.0, 10.0));
        assert_eq!(w.damage(), 0.0);
    }

    #[test]
    fn a_slow_or_distant_impact_does_nothing() {
        let mut w = Wreck::default();
        assert!(!w.impact(&MID, ONE, 0.5, 1.0, 10.0), "closing under 1 m/s");
        assert!(!w.impact(&MID, ONE, 5.0, 1.0, 40.0), "beyond the suppress distance");
        assert_eq!(w.damage(), 0.0);
    }

    #[test]
    fn side_hits_wreck_the_base_car_on_the_sixth() {
        let mut w = Wreck::default();
        // Each side hit at a ratio of one adds 1 / 5 = 0.2: five leave it at 1.0, the sixth wrecks it.
        for _ in 0..5 {
            assert!(!w.impact(&MID, ONE, 5.0, 1.0, 10.0));
        }
        assert!(!w.is_destroyed());
        assert!(w.impact(&MID, ONE, 5.0, 1.0, 10.0));
        assert!(w.is_destroyed());
        // Once wrecked, nothing more happens and it does not report twice.
        assert!(!w.impact(&MID, ONE, 5.0, 1.0, 10.0));
    }

    #[test]
    fn a_front_hit_at_half_scale_needs_a_bigger_hit() {
        let mut w = Wreck::default();
        // Ratio 1.0 times 0.5 is 0.5, under the threshold: nothing. Ratio 2.0 times 0.5 is 1.0: counted at 1/5.
        assert!(!w.impact(&MID, ONE, 5.0, 0.5, 10.0));
        assert_eq!(w.damage(), 0.0);
        assert!(!w.impact(&MID, 2.0 * ONE, 5.0, 0.5, 10.0));
        assert!((w.damage() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_flipped_car_is_wrecked_after_six_seconds() {
        let mut w = Wreck::default();
        let mut t = 0.0;
        let mut wrecked_at = None;
        while t < 7.0 && wrecked_at.is_none() {
            t += 0.01;
            if w.flip(0.01, true, false) {
                wrecked_at = Some(t);
            }
        }
        let at = wrecked_at.expect("flipped for seven seconds");
        assert!((at - FLIP_TIME).abs() < 0.11, "wrecked at {at}");
    }

    #[test]
    fn righting_the_car_resets_the_flipped_time() {
        let mut w = Wreck::default();
        for _ in 0..500 {
            w.flip(0.01, true, false);
        }
        // Five seconds flipped, then righted: the count starts again.
        for _ in 0..200 {
            w.flip(0.01, false, false);
        }
        for _ in 0..500 {
            assert!(!w.flip(0.01, true, false), "not wrecked a second time at five seconds");
        }
        assert!(!w.is_destroyed());
    }

    #[test]
    fn a_car_the_player_flipped_is_wrecked_at_once() {
        let mut w = Wreck::default();
        assert!(w.flip(FLIP_CHECK, true, true));
        assert!(w.is_destroyed());
    }

    #[test]
    fn the_flipped_test_needs_a_wheel_off_and_the_body_turned() {
        assert!(is_flipped(-1.0, -0.9, false));
        assert!(!is_flipped(-1.0, -0.9, true), "all wheels down: not flipped");
        assert!(!is_flipped(0.9, 0.9, false), "upright");
    }

    #[test]
    fn a_hard_hit_sets_the_shock_to_its_scale() {
        let mut w = Wreck::default();
        // Force 20000 N s on 1000 kg: scale 20000 / 1000 / 40 = 0.5.
        w.set_shock(20_000.0, 1000.0);
        assert!(w.in_shock());
        // It loses 1 / SHOCK_TIME a second: 0.5 becomes 1/3 after half a second, and nothing after 1.1 s more.
        w.step_shock(0.5);
        assert!(w.in_shock(), "a third of the shock is left after half a second");
        w.step_shock(1.1);
        assert!(!w.in_shock());
    }

    #[test]
    fn a_light_hit_does_not_shock() {
        let mut w = Wreck::default();
        // Scale 4000 / 1000 / 40 = 0.1, under the 0.2 floor.
        w.set_shock(4_000.0, 1000.0);
        assert!(!w.in_shock());
    }
}
