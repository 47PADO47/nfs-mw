//! The arcade help of braking (`docs/specs/ride6-bike-braking.md`): **a modification made exclusively for the NFS:MW rewrite, not the behaviour of the
//! reference game.** Part of [`super::ArcadeSteering`]; [`ArcadeBrake::REFERENCE`] is neutral in every field.
//!
//! The reference game's bikes stop at 0.85 to 1.0 g on tarmac, the most the brake torque of the data, the tyre and the weight transfer give, and a
//! bike braked hard lifts its rear wheel. For a street racing game that is "the bike hardly brakes". The help has two parts:
//!
//! 1. **the brake force** (`gain`, `max_decel_g`): a deceleration force at the centre of gravity, along the road against the velocity, that makes up
//!    what the tyres do not give to the deceleration the levers aim at (`max_decel_g` of the bike's kind at the full lever, on a good road, in
//!    proportion to the levers and to the surface's grip), at most `gain` g. The tyres are left as they are, so the slip, the anti-lock and the
//!    squeal still read what the tyres do;
//! 2. **the pitch hold** (`pitch_hold`): a torque that takes a share of the nose-down moment of the tyres' braking force off the body while the
//!    rider's weight is not forward, so a full brake is a controlled strong brake and the rear wheel stays down; a rider who pushes his weight forward
//!    still lifts it (a stoppie of his own).

use crate::bike::curve::ramp;

/// The share of the brake force the front lever stands for in the mix of the levers; the rear lever has the rest. The front brake does most of the
/// braking of a bike, so a front lever alone aims for 0.7 of the full lever's deceleration (the joined brakes add to the rear lever of a bike that
/// has them, `brakes::joined`).
pub const FRONT_SHARE: f32 = 0.7;

/// The deceleration (g) a full lever aims for on a good road, in all, per kind of bike (`ArcadeBrake::max_decel_g`). A project's choices
/// (`docs/specs/ride6-bike-braking.md`): the playtest asked for "way more" braking, the brief for a stop from 100 km/h in about 30 m and from 200 km/h in
/// about 120 m with a mean of at least 1.3 g for a road bike, 1.0 g for a supermoto or an offroad bike and 0.9 g for a scooter. The aimed value is the
/// *steady* one: the lever's rise (a third of a second) and the last metres cost the mean about 0.1 g, so the road bike aims for 1.4 g to stop in 30 m
/// from 100 km/h with a mean of 1.3 g.
pub const ROAD_DECEL_G: f32 = 1.4;
pub const MOTARD_DECEL_G: f32 = 1.0;
pub const OFFROAD_DECEL_G: f32 = 1.0;
pub const SCOOTER_DECEL_G: f32 = 0.9;
/// The most extra deceleration (g) the help adds to the tyres' (`ArcadeBrake::gain`): the shortfall of the worst bike measured is 0.55 g (a road bike
/// that stops at 0.85 g aiming for 1.4 g), so 0.7 g leaves room and still bounds the help on a bike whose tyres have lost their grip.
pub const GAIN: f32 = 0.7;
/// The share of the tyres' nose-down moment the hold takes off the body (`ArcadeBrake::pitch_hold`): 0.5 keeps the rear wheel of every bike measured
/// on the road (the supermotos and the scooters lifted it for seconds at 0.0) and still leaves the fork to dive.
pub const PITCH_HOLD: f32 = 0.5;

/// The values of the help on braking; see the module's documentation. `REFERENCE` is neutral.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcadeBrake {
    /// The most extra deceleration (g) the help adds to what the tyres give, at the full lever on a good road. 0: no force.
    pub gain: f32,
    /// The deceleration (g) the full lever aims for in all (the tyres' and the help's), on a good road, per class of bike
    /// ([`crate::bike::TerrainClass::index`]: road, motard, offroad); the surface's grip scales it.
    pub max_decel_g: [f32; 3],
    /// The same for a scooter ([`crate::bike::BikeSpec::is_scooter`]), whatever its class.
    pub scooter_decel_g: f32,
    /// The speeds (m/s) between which the force comes in: none at or under the first (a bike that stands is the standing hold's), whole at or over
    /// the second.
    pub speed: [f32; 2],
    /// The throttle (0 to 1) from which the force fades out and at which it is gone: a rider on the throttle does not brake (a burnout has none at all).
    pub throttle: [f32; 2],
    /// The share (0 to 1) of the nose-down moment of the tyres' braking force (their force times the height of the centre of gravity) that the hold
    /// takes off the body. 0: none.
    pub pitch_hold: f32,
    /// The rider's forward weight (0 to 1) between which the hold is let go: whole at or under the first, gone at or over the second.
    pub weight_release: [f32; 2],
}

impl ArcadeBrake {
    /// Everything neutral: the reference game's brakes.
    pub const REFERENCE: Self = Self {
        gain: 0.0,
        max_decel_g: [0.0; 3],
        scooter_decel_g: 0.0,
        speed: [2.0, 6.0],
        throttle: [0.1, 0.5],
        pitch_hold: 0.0,
        weight_release: [0.2, 0.5],
    };

    /// The deceleration (m/s^2) the levers aim for in all: `max_decel_g` of the bike's kind, in proportion to the lever mix (`front` and `rear`, 0 to 1)
    /// and to the surface's grip `surface` (1 on tarmac).
    pub fn target(&self, class: usize, scooter: bool, front: f32, rear: f32, surface: f32) -> f32 {
        let full = match scooter {
            true => self.scooter_decel_g,
            false => self.max_decel_g[class.min(2)],
        };
        let mix = (FRONT_SHARE * front + (1.0 - FRONT_SHARE) * rear).clamp(0.0, 1.0);
        full * 9.81 * mix * surface.clamp(0.0, 1.0)
    }

    /// The force's share at `speed` m/s with the throttle at `throttle`: it comes in with the speed and fades out with the throttle.
    pub fn fade(&self, speed: f32, throttle: f32) -> f32 {
        ramp(speed.abs(), self.speed) * (1.0 - ramp(throttle, self.throttle))
    }

    /// The deceleration (m/s^2) the help adds when the levers aim for `target` and the tyres give `tyres` (both positive for a braking force): the
    /// shortfall, held to `gain` g.
    pub fn extra(&self, target: f32, tyres: f32) -> f32 {
        (target - tyres).clamp(0.0, self.gain * 9.81)
    }

    /// The share of the pitch hold at the rider's forward weight `weight` (-1 to 1): whole while it is not forward, gone when he pushes it forward.
    pub fn hold_share(&self, weight: f32) -> f32 {
        self.pitch_hold * (1.0 - ramp(weight, self.weight_release))
    }
}

impl Default for ArcadeBrake {
    /// This game's values (`docs/specs/ride6-bike-braking.md`).
    fn default() -> Self {
        Self {
            gain: GAIN,
            max_decel_g: [ROAD_DECEL_G, MOTARD_DECEL_G, OFFROAD_DECEL_G],
            scooter_decel_g: SCOOTER_DECEL_G,
            speed: [2.0, 6.0],
            throttle: [0.1, 0.5],
            pitch_hold: PITCH_HOLD,
            weight_release: [0.2, 0.5],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_adds_nothing() {
        let reference = ArcadeBrake::REFERENCE;
        assert_eq!(reference.target(0, false, 1.0, 1.0, 1.0), 0.0);
        assert_eq!(reference.extra(9.0, 0.0), 0.0);
        assert_eq!(reference.hold_share(0.0), 0.0);
    }

    #[test]
    fn the_target_follows_the_levers_the_kind_of_bike_and_the_surface() {
        let game = ArcadeBrake::default();
        let full = game.target(0, false, 1.0, 1.0, 1.0);
        assert!((full - ROAD_DECEL_G * 9.81).abs() < 1e-4);
        assert!((game.target(0, false, 0.5, 0.5, 1.0) - 0.5 * full).abs() < 1e-4);
        assert!((game.target(0, false, 1.0, 0.0, 1.0) - FRONT_SHARE * full).abs() < 1e-4);
        assert!((game.target(0, false, 0.0, 1.0, 1.0) - (1.0 - FRONT_SHARE) * full).abs() < 1e-4);
        assert!((game.target(1, false, 1.0, 1.0, 1.0) - 9.81).abs() < 1e-4, "a supermoto: 1.0 g");
        assert!(
            (game.target(0, true, 1.0, 1.0, 1.0) - 0.9 * 9.81).abs() < 1e-4,
            "a scooter, whatever its class: 0.9 g"
        );
        assert!((game.target(2, false, 1.0, 1.0, 0.6) - 0.6 * 9.81).abs() < 1e-4, "gravel scales it");
        const { assert!(ROAD_DECEL_G >= 1.3) };
        assert_eq!(game.target(0, false, 0.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn the_help_makes_up_the_shortfall_up_to_the_gain_and_never_takes_away() {
        let game = ArcadeBrake::default();
        let target = game.target(0, false, 1.0, 1.0, 1.0);
        let tyres = 0.9 * 9.81;
        assert!((game.extra(target, tyres) - (target - tyres)).abs() < 1e-4);
        assert_eq!(game.extra(target, 1.5 * 9.81), 0.0, "tyres that give more are left alone");
        assert!((game.extra(target, 0.0) - 0.7 * 9.81).abs() < 1e-4, "held to the gain");
    }

    #[test]
    fn the_force_comes_in_with_the_speed_and_goes_with_the_throttle() {
        let game = ArcadeBrake::default();
        assert_eq!(game.fade(0.0, 0.0), 0.0);
        assert_eq!(game.fade(game.speed[0], 0.0), 0.0);
        assert_eq!(game.fade(game.speed[1], 0.0), 1.0);
        assert!(game.fade(4.0, 0.0) > 0.0 && game.fade(4.0, 0.0) < 1.0);
        assert_eq!(game.fade(30.0, game.throttle[1]), 0.0);
        assert_eq!(game.fade(-30.0, 0.0), 1.0, "the speed's sign does not matter");
    }

    #[test]
    fn the_hold_is_let_go_when_the_rider_pushes_his_weight_forward() {
        let game = ArcadeBrake::default();
        assert_eq!(game.hold_share(-1.0), game.pitch_hold);
        assert_eq!(game.hold_share(game.weight_release[0]), game.pitch_hold);
        assert_eq!(game.hold_share(1.0), 0.0);
        assert!(game.hold_share(0.35) > 0.0 && game.hold_share(0.35) < game.pitch_hold);
    }
}
