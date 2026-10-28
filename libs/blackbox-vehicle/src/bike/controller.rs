//! The rider's balance: the handlebar angle that makes the bike lean where the stick asks.
//!
//! A bike standing on two points falls over unless it is steered. Near the ground the roll of a bike at speed `v` behaves
//! like an inverted pendulum whose support point is moved by the steering:
//!
//! ```text
//! I phi'' = m g h sin(phi) - m h (v^2 / b) delta
//! ```
//!
//! with `phi` the lean (positive right), `delta` the handlebar angle (positive right), `b` the wheelbase, `h` the height of
//! the centre of gravity and `I` the roll inertia about the contact line. The controller asks for a lean acceleration
//! that pulls `phi` to the target with a critically damped response and solves this equation for `delta`. To start a lean
//! to the right it steers left first; once the lean is reached it steers into the turn by `b g tan(phi) / v^2`.
//!
//! Below walking speed the equation loses its grip on the steering, so the handlebar follows the stick directly and the
//! rider's feet keep the bike up.

use super::spec::SteeringSpec;

/// Below this speed the handlebar follows the stick; above `FULL_DYNAMIC_SPEED` the balance law has all the say (m/s).
const DIRECT_SPEED: f32 = 2.0;
const FULL_DYNAMIC_SPEED: f32 = 5.0;

/// What the controller needs to know about the bike this step.
#[derive(Clone, Copy, Debug)]
pub struct Balance {
    /// Speed along the bike (m/s).
    pub speed: f32,
    /// Lean (rad) and its rate (rad/s), positive right.
    pub lean: f32,
    pub lean_rate: f32,
    pub mass: f32,
    pub cg_height: f32,
    pub wheelbase: f32,
    /// The roll inertia about the centre of gravity (kg m^2).
    pub roll_inertia: f32,
    /// Gravity as a positive number (m/s^2).
    pub gravity: f32,
}

/// The lean the stick asks for (rad, positive right): the stick's share of the steepest lean, but no steeper than the
/// grip allows at `lateral_grip` (the lateral force as a multiple of the load).
pub fn target_lean(steering: &SteeringSpec, stick: f32, lateral_grip: f32) -> f32 {
    let limit = steering.max_lean.min(lateral_grip.max(0.0).atan());
    stick.clamp(-1.0, 1.0) * limit
}

/// The handlebar angle (rad, positive right) that moves the lean to `target`.
pub fn handlebar(steering: &SteeringSpec, balance: &Balance, target: f32, stick: f32) -> f32 {
    let direct = stick.clamp(-1.0, 1.0) * steering.max_angle;
    let blend = ((balance.speed - DIRECT_SPEED) / (FULL_DYNAMIC_SPEED - DIRECT_SPEED)).clamp(0.0, 1.0);
    if blend == 0.0 {
        return direct;
    }
    let inertia = balance.roll_inertia + balance.mass * balance.cg_height * balance.cg_height;
    let fall = balance.mass * balance.gravity * balance.cg_height / inertia;
    let speed = balance.speed.max(FULL_DYNAMIC_SPEED * 0.5);
    let steer_gain = balance.mass * balance.cg_height * speed * speed / (balance.wheelbase * inertia);
    let kp = steering.lean_response * steering.lean_response;
    let kd = 2.0 * steering.lean_response;
    let wanted = fall * balance.lean.sin() + kp * (balance.lean - target) + kd * balance.lean_rate;
    let dynamic = (wanted / steer_gain).clamp(-steering.max_angle, steering.max_angle);
    blend * dynamic + (1.0 - blend) * direct
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn balance(speed: f32, lean: f32) -> Balance {
        Balance {
            speed,
            lean,
            lean_rate: 0.0,
            mass: 150.0,
            cg_height: 0.62,
            wheelbase: 1.47,
            roll_inertia: 8.0,
            gravity: 9.81,
        }
    }

    #[test]
    fn the_target_lean_follows_the_stick_and_is_capped_by_the_grip() {
        let s = BikeSpec::example().steering;
        assert_eq!(target_lean(&s, 0.0, 1.2), 0.0);
        assert!(target_lean(&s, 1.0, 1.2) > 0.0 && target_lean(&s, -1.0, 1.2) < 0.0);
        assert!(target_lean(&s, 1.0, 0.3) < target_lean(&s, 1.0, 1.2));
        assert!(target_lean(&s, 5.0, 1.2) <= s.max_lean + 1e-6);
    }

    #[test]
    fn an_upright_bike_asked_to_lean_right_first_steers_left() {
        let s = BikeSpec::example().steering;
        assert!(handlebar(&s, &balance(20.0, 0.0), 0.5, 1.0) < 0.0);
    }

    #[test]
    fn a_bike_at_its_target_steers_into_the_turn() {
        let s = BikeSpec::example().steering;
        let delta = handlebar(&s, &balance(20.0, 0.4), 0.4, 1.0);
        // At the target the balance law reduces to b g sin(phi) / v^2, whatever the inertia.
        let expected = 1.47 * 9.81 * 0.4_f32.sin() / (20.0 * 20.0);
        assert!(delta > 0.0 && (delta - expected).abs() < 1e-4, "{delta} {expected}");
    }

    #[test]
    fn a_bike_leaning_past_its_target_steers_harder_into_the_lean() {
        let s = BikeSpec::example().steering;
        let at = handlebar(&s, &balance(20.0, 0.3), 0.3, 1.0);
        let over = handlebar(&s, &balance(20.0, 0.3), 0.0, 0.0);
        assert!(over > at);
    }

    #[test]
    fn the_handlebar_follows_the_stick_at_a_walking_pace() {
        let s = BikeSpec::example().steering;
        assert_eq!(handlebar(&s, &balance(0.5, 0.0), 0.0, 0.5), 0.5 * s.max_angle);
    }

    #[test]
    fn the_handlebar_never_exceeds_its_lock() {
        let s = BikeSpec::example().steering;
        assert!(handlebar(&s, &balance(30.0, 1.2), -0.9, -1.0).abs() <= s.max_angle + 1e-6);
    }
}
