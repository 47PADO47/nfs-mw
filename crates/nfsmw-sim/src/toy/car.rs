//! One toy car: a point on the ground plane with a heading, a signed speed along it and a few meters.
//!
//! The model is kinematic. Throttle pushes the speed up against a top speed, brake slows it and then reverses, steering turns
//! the heading at a rate that grows with speed up to a point and shrinks at high speed, the handbrake scrubs speed and tightens
//! turns, nitrous doubles the push and raises the top speed while it lasts. Only `+ - * /` and `sqrt` are used (the heading
//! rotates by a short series, not `sin` and `cos`), so the result is bit-exact on any IEEE 754 machine.

use blackbox_vehicle::InputState;

use crate::hash::StateHasher;

/// The fixed step, in seconds.
pub const DT: f32 = 1.0 / 60.0;

/// A car is a disc of this radius, in metres.
pub const CAR_RADIUS: f32 = 1.8;

const ACCEL: f32 = 9.0;
const BRAKE_DECEL: f32 = 16.0;
const REVERSE_ACCEL: f32 = 5.0;
const REVERSE_MAX: f32 = 12.0;
const REVERSE_BELOW: f32 = 0.5;
const DRAG: f32 = 0.04;
const ROLL: f32 = 0.6;
const HAND_DECEL: f32 = 12.0;
const MAX_YAW: f32 = 1.6;
const NOS_DRAIN: f32 = 0.25;
const NOS_REGEN: f32 = 0.04;
const NOS_TOP_SPEED: f32 = 1.25;
const NOS_PUSH: f32 = 2.0;

/// The controls of one tick after the input is made safe: finite, in range.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Controls {
    pub throttle: f32,
    pub brake: f32,
    /// -1..1, + = right.
    pub steer: f32,
    pub handbrake: f32,
    pub nos: bool,
}

impl Controls {
    /// Make a player's or bot's input safe. NaN becomes neutral; infinities and out-of-range values clamp.
    pub fn from_input(input: &InputState) -> Self {
        Self {
            throttle: clamp_or(input.throttle, 0.0, 1.0),
            brake: clamp_or(input.brake, 0.0, 1.0),
            steer: clamp_or(input.steer, -1.0, 1.0),
            handbrake: clamp_or(input.handbrake, 0.0, 1.0),
            nos: input.nos,
        }
    }
}

fn clamp_or(v: f32, lo: f32, hi: f32) -> f32 {
    if v.is_nan() {
        return 0.0;
    }
    v.clamp(lo, hi)
}

/// A toy car's whole state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Car {
    /// Position on the ground plane, `[x, z]` metres.
    pub pos: [f32; 2],
    /// Unit forward vector `[x, z]`; a heading of 0 faces +z and a positive turn goes towards +x (to the left).
    pub dir: [f32; 2],
    /// Signed speed along `dir`, metres per second; negative is reversing.
    pub speed: f32,
    /// Turn rate of the last step, radians per second, + = counter-clockwise seen from above.
    pub yaw_rate: f32,
    /// Nitrous left, 0..1.
    pub nitrous: f32,
    /// Speedbreaker meter, 0..1: charges while drifting fast.
    pub boost: f32,
    /// Engine strength factor of the car type.
    pub power: f32,
    /// Top speed without nitrous, metres per second.
    pub top_speed: f32,
    pub braking: bool,
    pub nos_active: bool,
    pub handbrake_on: bool,
}

impl Car {
    pub fn new(pos: [f32; 2], dir: [f32; 2], power: f32, top_speed: f32) -> Self {
        Self {
            pos,
            dir,
            speed: 0.0,
            yaw_rate: 0.0,
            nitrous: 1.0,
            boost: 0.0,
            power,
            top_speed,
            braking: false,
            nos_active: false,
            handbrake_on: false,
        }
    }

    /// The velocity in world metres per second, `[x, z]`.
    pub fn velocity(&self) -> [f32; 2] {
        [self.dir[0] * self.speed, self.dir[1] * self.speed]
    }

    /// Advance one fixed step under `c`, then keep the car inside the square `[-bounds, bounds]`.
    pub fn step(&mut self, c: Controls, bounds: f32) {
        let nos = c.nos && self.nitrous > 0.0;
        self.nos_active = nos;
        self.braking = c.brake > 0.0;
        self.handbrake_on = c.handbrake > 0.0;
        self.nitrous = match nos {
            true => (self.nitrous - NOS_DRAIN * DT).max(0.0),
            false => (self.nitrous + NOS_REGEN * DT).min(1.0),
        };
        let mut limit = self.top_speed;
        let mut push = ACCEL * self.power;
        if nos {
            limit *= NOS_TOP_SPEED;
            push *= NOS_PUSH;
        }
        let mut v = self.speed;
        let room = (1.0 - v.max(0.0) / limit).clamp(0.0, 1.0);
        v += c.throttle * push * room * DT;
        if c.brake > 0.0 {
            v = brake_or_reverse(v, c.brake);
        }
        v -= v * DRAG * DT;
        if c.throttle == 0.0 && c.brake == 0.0 {
            v = toward_zero(v, ROLL * DT);
        }
        v = toward_zero(v, c.handbrake * HAND_DECEL * DT);
        self.speed = v.clamp(-REVERSE_MAX, limit);
        self.turn(c);
        self.charge_boost(c);
        self.pos[0] += self.dir[0] * self.speed * DT;
        self.pos[1] += self.dir[1] * self.speed * DT;
        self.keep_inside(bounds);
    }

    fn turn(&mut self, c: Controls) {
        let v = self.speed;
        let grip = (v.abs() / 5.0).min(1.0) / (1.0 + v.abs() / 40.0);
        let mut rate = c.steer * MAX_YAW * grip * (1.0 + 0.6 * c.handbrake);
        if v < 0.0 {
            rate = -rate;
        }
        self.yaw_rate = -rate;
        self.dir = rotate(self.dir, self.yaw_rate * DT);
    }

    fn charge_boost(&mut self, c: Controls) {
        let drifting = self.speed.abs() > 20.0 && c.handbrake > 0.3;
        self.boost = match drifting {
            true => (self.boost + 0.1 * DT).min(1.0),
            false => (self.boost - 0.02 * DT).max(0.0),
        };
    }

    /// Clamp the position to the plane. A car that hits the edge loses half its speed. Returns whether it hit.
    pub fn keep_inside(&mut self, bounds: f32) -> bool {
        let mut hit = false;
        for p in &mut self.pos {
            let clamped = p.clamp(-bounds, bounds);
            if clamped != *p {
                hit = true;
                *p = clamped;
            }
        }
        if hit {
            self.speed *= 0.5;
        }
        hit
    }

    /// The gear a driver would be in at this speed: 0 reverse, 1 neutral, 2.. forward.
    pub fn gear(&self) -> u8 {
        if self.speed < -REVERSE_BELOW {
            return 0;
        }
        if self.speed < REVERSE_BELOW {
            return 1;
        }
        2 + ((self.speed / 12.0) as u8).min(5)
    }

    /// Engine speed from the speed inside the current gear's band.
    pub fn rpm(&self) -> f32 {
        let gear = self.gear();
        if gear < 2 {
            return 900.0 + self.speed.abs() * 200.0;
        }
        let band = self.speed - f32::from(gear - 2) * 12.0;
        900.0 + (band / 12.0).clamp(0.0, 1.0) * 6100.0
    }

    /// Flag bits for replication: 1 brake lights, 2 nitrous burning, 4 handbrake.
    pub fn flags(&self) -> u32 {
        u32::from(self.braking) | u32::from(self.nos_active) << 1 | u32::from(self.handbrake_on) << 2
    }

    /// The orientation as a quaternion `[x, y, z, w]` about the up axis, from the forward vector.
    pub fn orientation(&self) -> [f32; 4] {
        let half_cos = ((1.0 + self.dir[1]) * 0.5).max(0.0).sqrt();
        let mut half_sin = ((1.0 - self.dir[1]) * 0.5).max(0.0).sqrt();
        if self.dir[0] < 0.0 {
            half_sin = -half_sin;
        }
        [0.0, half_sin, 0.0, half_cos]
    }

    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_f32_slice(&[self.pos[0], self.pos[1], self.dir[0], self.dir[1], self.speed, self.yaw_rate]);
        h.write_f32_slice(&[self.nitrous, self.boost, self.power, self.top_speed]);
        h.write_u32(self.flags());
    }
}

/// Brake while moving forward, otherwise back up.
fn brake_or_reverse(v: f32, brake: f32) -> f32 {
    if v > REVERSE_BELOW {
        return (v - brake * BRAKE_DECEL * DT).max(0.0);
    }
    v - brake * REVERSE_ACCEL * DT
}

fn toward_zero(v: f32, amount: f32) -> f32 {
    if v.abs() <= amount {
        return 0.0;
    }
    match v > 0.0 {
        true => v - amount,
        false => v + amount,
    }
}

/// Rotate a unit vector by a small angle `d` (radians) with short series for `sin` and `cos`, then normalise it.
fn rotate(dir: [f32; 2], d: f32) -> [f32; 2] {
    let d2 = d * d;
    let s = d * (1.0 - d2 / 6.0);
    let c = 1.0 - d2 * 0.5 + d2 * d2 / 24.0;
    let x = dir[0] * c + dir[1] * s;
    let z = dir[1] * c - dir[0] * s;
    let len = (x * x + z * z).sqrt();
    if len > 0.0 && len.is_finite() {
        return [x / len, z / len];
    }
    [0.0, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car() -> Car {
        Car::new([0.0, 0.0], [0.0, 1.0], 1.0, 60.0)
    }

    fn full_throttle() -> Controls {
        Controls { throttle: 1.0, ..Controls::default() }
    }

    #[test]
    fn throttle_accelerates_and_reaches_a_top_speed() {
        let mut c = car();
        for _ in 0..2000 {
            c.step(full_throttle(), 1.0e6);
        }
        assert!(c.speed > 40.0 && c.speed <= 60.0, "{}", c.speed);
    }

    #[test]
    fn brake_stops_then_reverses() {
        let mut c = car();
        c.speed = 20.0;
        let brake = Controls { brake: 1.0, ..Controls::default() };
        for _ in 0..120 {
            c.step(brake, 1.0e6);
        }
        assert!(c.speed < 0.0, "{}", c.speed);
        for _ in 0..1000 {
            c.step(brake, 1.0e6);
        }
        assert!(c.speed >= -REVERSE_MAX);
    }

    #[test]
    fn steering_turns_more_at_speed_and_not_when_stopped() {
        let right = Controls { steer: 1.0, ..Controls::default() };
        let mut stopped = car();
        stopped.step(right, 1.0e6);
        assert_eq!(stopped.yaw_rate, 0.0);
        let mut slow = car();
        slow.speed = 5.0;
        slow.step(right, 1.0e6);
        assert!(slow.yaw_rate < 0.0, "right is a negative turn");
        assert!(slow.dir[0] < 0.0);
    }

    #[test]
    fn heading_stays_unit_length_over_long_turns() {
        let mut c = car();
        let hard = Controls { throttle: 1.0, steer: 1.0, handbrake: 0.5, ..Controls::default() };
        for _ in 0..100_000 {
            c.step(hard, 1.0e6);
        }
        let len = (c.dir[0] * c.dir[0] + c.dir[1] * c.dir[1]).sqrt();
        assert!((len - 1.0).abs() < 1.0e-5, "{len}");
    }

    #[test]
    fn nitrous_boosts_and_runs_out() {
        let mut plain = car();
        let mut boosted = car();
        let nos = Controls { throttle: 1.0, nos: true, ..Controls::default() };
        for _ in 0..180 {
            plain.step(full_throttle(), 1.0e6);
            boosted.step(nos, 1.0e6);
        }
        assert!(boosted.speed > plain.speed);
        assert!(boosted.nitrous < 1.0);
        for _ in 0..600 {
            boosted.step(nos, 1.0e6);
        }
        assert!(boosted.nitrous < 0.01, "{}", boosted.nitrous);
    }

    #[test]
    fn handbrake_scrubs_speed() {
        let mut c = car();
        c.speed = 30.0;
        let hand = Controls { handbrake: 1.0, ..Controls::default() };
        for _ in 0..240 {
            c.step(hand, 1.0e6);
        }
        assert_eq!(c.speed, 0.0);
    }

    #[test]
    fn the_edge_clamps_and_slows() {
        let mut c = car();
        c.pos = [99.0, 0.0];
        c.dir = [1.0, 0.0];
        c.speed = 40.0;
        for _ in 0..30 {
            c.step(full_throttle(), 100.0);
        }
        assert_eq!(c.pos[0], 100.0);
    }

    #[test]
    fn controls_are_made_safe() {
        let wild = InputState {
            throttle: f32::NAN,
            brake: f32::INFINITY,
            steer: -f32::INFINITY,
            handbrake: 9.0,
            ..Default::default()
        };
        let c = Controls::from_input(&wild);
        assert_eq!((c.throttle, c.brake, c.steer, c.handbrake), (0.0, 1.0, -1.0, 1.0));
    }

    #[test]
    fn orientation_is_a_unit_quaternion_for_every_heading() {
        let mut c = car();
        let spin = Controls { throttle: 1.0, steer: 1.0, ..Controls::default() };
        for _ in 0..2000 {
            c.step(spin, 1.0e6);
            let q = c.orientation();
            let len = (q[1] * q[1] + q[3] * q[3]).sqrt();
            assert!((len - 1.0).abs() < 1.0e-4);
        }
    }

    #[test]
    fn gear_and_rpm_follow_speed() {
        let mut c = car();
        assert_eq!(c.gear(), 1);
        c.speed = 30.0;
        assert_eq!(c.gear(), 4);
        assert!((900.0..=7000.0).contains(&c.rpm()));
        c.speed = -3.0;
        assert_eq!(c.gear(), 0);
    }
}
