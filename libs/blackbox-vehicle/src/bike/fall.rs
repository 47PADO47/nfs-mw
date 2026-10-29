//! The fall tests (`docs/specs/ride6-chassis-round6.md`, "Fall tests"): the lean and the pitch, the angular speed, and the angle
//! between the velocity and the heading. A fallen rider stays fallen until the bike is placed again.

use glam::Vec3;

use super::chassis_state::Environment;
use crate::bike::release::FallKind;
use crate::bike::{Bike, FRONT, REAR, lean_of, pitch_of};

impl Bike {
    /// The day the bike rides in: the temperature of the air and of the road, which the tyres cool towards. The host sets it from
    /// its track and weather; until it does the stand-in `Environment::default()` is used.
    pub fn set_environment(&mut self, environment: Environment) {
        self.chassis.environment = environment;
    }

    /// The day the bike rides in.
    pub fn environment(&self) -> Environment {
        self.chassis.environment
    }

    /// Whether no wheel touches the road (the reference game's "distance from the ground" is above its limit, which is 0 for the
    /// bikes read).
    fn airborne(&self) -> bool {
        !self.wheels[FRONT].on_ground && !self.wheels[REAR].on_ground
    }

    /// The angle (rad) between the horizontal velocity and the horizontal heading; 0 when either is too small to have a direction.
    pub(in crate::bike) fn heading_error(&self) -> f32 {
        let forward = self.body.rotation() * Vec3::Z;
        let velocity = self.body.linear_velocity;
        let flat_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z);
        match flat_velocity.length() > 1.0 && flat_forward.length() > 1e-3 {
            true => flat_velocity.angle_between(flat_forward),
            false => 0.0,
        }
    }

    /// The bank of the road under the bike (rad), positive when the road is lower on the right of the heading: the sine is the tilt
    /// of the normal (the two wheels' contact normals added) across the heading, measured on the road's plane. 0 with no wheel down.
    pub(in crate::bike) fn terrain_slope(&self) -> f32 {
        let down = |index: usize| self.wheels[index].on_ground.then_some(self.wheels[index].normal);
        let normal = match (down(FRONT), down(REAR)) {
            (Some(front), Some(rear)) => front + rear,
            (Some(only), None) | (None, Some(only)) => only,
            (None, None) => return 0.0,
        };
        let Some(normal) = normal.try_normalize() else { return 0.0 };
        let forward = self.body.rotation() * Vec3::Z;
        let Some(along) = (forward - normal * forward.dot(normal)).try_normalize() else { return 0.0 };
        // normal x along points to the right of the heading in the road's plane; its vertical part is minus the bank's sine.
        (-normal.cross(along).y).clamp(-1.0, 1.0).asin()
    }

    /// The tests, once per step, in the order of the reference (the first that fires ends the run and says what kind of fall it
    /// was): the lean (for `lean_duration` on the ground, at once in the air), the pitch, the angle of the velocity to the heading
    /// (only above `min_speed`), the angular speed. A fall lets the rider go ([`Bike::take_rider_release`]).
    pub(in crate::bike) fn check_fall(&mut self, dt: f32, speed: f32) {
        if self.fallen {
            return;
        }
        let Some(kind) = self.tripped(dt, speed) else { return };
        self.fallen = true;
        self.release_rider(kind);
    }

    /// The kind of fall the tests find now, if any.
    fn tripped(&mut self, dt: f32, speed: f32) -> Option<FallKind> {
        let rotation = self.body.rotation();
        let fall = self.spec.fall;
        let extra = fall.extra;
        let air = self.airborne();
        let pick = |airborne: f32, ground: f32| match air && airborne > 0.0 {
            true => airborne,
            false => ground,
        };
        let signed = lean_of(rotation);
        // The lean the tests see is relative to the road: shifted by the bank, by the influence when the two lean the same way.
        let slope = self.terrain_slope();
        let shift = match signed * slope >= 0.0 {
            true => extra.terrain_influence,
            false => extra.terrain_reduction,
        } * slope;
        let shift = match extra.terrain_max_delta > 0.0 {
            true => shift.clamp(-extra.terrain_max_delta, extra.terrain_max_delta),
            false => shift,
        };
        let lean = (signed - shift).abs();
        let lean_over = match air {
            true => lean > pick(extra.airborne_lean, fall.lean),
            false => lean > self.chassis.fall_lean,
        };
        // On the ground the lean must stay beyond the limit for `lean_duration` before it counts.
        let mut found = None;
        match lean_over && !air {
            true => {
                if self.chassis.fall_timer >= extra.lean_duration {
                    found = Some(FallKind::LowSide);
                }
                self.chassis.fall_timer += dt;
            }
            false => {
                if lean_over {
                    found = Some(FallKind::Generic);
                }
                self.chassis.fall_timer = 0.0;
            }
        }
        if found.is_some() {
            return found;
        }
        if pitch_of(rotation).abs() > pick(extra.airborne_pitch, fall.pitch) {
            return Some(FallKind::Generic);
        }
        // A velocity against the heading is a high side when the bike is not leaning far, a low side when it is.
        if speed > fall.min_speed && self.heading_error() > fall.speed_direction {
            return Some(match extra.highside_lean > 0.0 && lean < extra.highside_lean {
                true => FallKind::HighSide,
                false => FallKind::LowSide,
            });
        }
        if self.body.angular_velocity.length() > pick(extra.airborne_angular, fall.angular_speed) {
            return Some(FallKind::LowSide);
        }
        None
    }
}
