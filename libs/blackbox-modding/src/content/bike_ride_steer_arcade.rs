//! The arcade steering of the steering audits' runs (`RIDE6_ARCADE`, see `bike_ride_steer.rs`).

use blackbox_vehicle::bike::{ArcadeAir, ArcadeRough, ArcadeSteering, TurnLaw};

/// The arcade steering of the run: `RIDE6_ARCADE=reference` the reference game's, `RIDE6_ARCADE=grip=1.2,rate=3,...` the game's values with
/// those fields changed (`grip`, `rate`, `floor`, `mirror`, `spring`, `roll`, `assist`, `reach`, `slow` and `fast` for the speed authority's ends,
/// `drop` for the wheelie's steer drop in degrees, `turn=0` for no turn law, `slowlean` and `maxassist` for the turn law's quicker lean-in
/// at a low speed and its most assist in g), unset the game's values; `RIDE6_ARCADE=previous` is the game's values of the round before the
/// slower lean-in and the speed authority (`docs/specs/ride6-arcade-steering.md`, sections 10 and 11), and `RIDE6_ARCADE=round2` the game's of
/// the round before the turn law (its authority and lean-in, `docs/specs/ride6-bike-turn-radius.md`).
pub(super) fn arcade() -> ArcadeSteering {
    let text = std::env::var("RIDE6_ARCADE").unwrap_or_default();
    if text == "reference" {
        return ArcadeSteering::REFERENCE;
    }
    // The steering the game had before the arcade steering (rounds 12 and 13): the data's controller, the speed curve held at 0.65, the stick curve mirrored.
    if text == "before" {
        return ArcadeSteering { time_scale_speed_floor: 0.65, mirror_stick_curve: true, ..ArcadeSteering::REFERENCE };
    }
    let mut arcade = ArcadeSteering::default();
    // This game's steering without the help on jumps and on rough ground (`docs/specs/ride6-bike-jumps.md`, `ride6-bike-rough-ground.md`).
    if text == "air_before" {
        return ArcadeSteering { air: ArcadeAir::REFERENCE, rough: ArcadeRough::REFERENCE, ..arcade };
    }
    if text == "previous" {
        return ArcadeSteering {
            lean_rate: 3.0,
            roll_spring: 2.0,
            stick_response: 2.0,
            handlebar_response: 1.25,
            authority: [1.0, 1.0],
            wheelie_steer_drop: None,
            turn: None,
            ..arcade
        };
    }
    if text == "round2" {
        return ArcadeSteering { turn: None, ..arcade };
    }
    for part in text.split(',').filter(|p| !p.is_empty()) {
        let Some((key, value)) = part.split_once('=') else { continue };
        let Ok(v) = value.parse::<f32>() else { continue };
        match key {
            "grip" => arcade.lateral_grip = v,
            "lean" => arcade.lean_limit = v,
            "rate" => arcade.lean_rate = v,
            "floor" => arcade.time_scale_speed_floor = v,
            "mirror" => arcade.mirror_stick_curve = v > 0.5,
            "spring" => arcade.roll_spring = v,
            "help" => arcade.lean_help = v,
            "helplimit" => arcade.lean_help_limit = v,
            "spin" => arcade.fall_angular_speed = v,
            "stick" => arcade.stick_response = v,
            "bar" => arcade.handlebar_response = v,
            "assist" => arcade.cornering_assist = v,
            "reach" => arcade.stick_reach = v,
            "slow" => arcade.authority[0] = v,
            "fast" => arcade.authority[1] = v,
            "drop" => arcade.wheelie_steer_drop = Some(v.to_radians()),
            "turn" if v < 0.5 => arcade.turn = None,
            "slowlean" => arcade.turn = arcade.turn.map(|law| TurnLaw { slow_lean_rate: v, ..law }),
            "maxassist" => arcade.turn = arcade.turn.map(|law| TurnLaw { max_assist: v, ..law }),
            "air" if v < 0.5 => arcade.air = ArcadeAir::REFERENCE,
            "rough" if v < 0.5 => arcade.rough = ArcadeRough::REFERENCE,
            _ => {}
        }
    }
    arcade
}
