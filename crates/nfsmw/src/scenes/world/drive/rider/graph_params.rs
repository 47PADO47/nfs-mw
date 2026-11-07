//! The values the graph's parameters take, by the names the graph gives them.

use super::params::Values;

/// The graph's parameters by name: the names are the ones the animation spec gives (`lean`, `speed_kmh`, `tuck`, `wheelie`,
/// `stoppie`, and `wheelie_alpha`, `wheelie_direction`, `stoppie_time` for the tricks, `standing`, `steer`, `standing_entry`
/// for the standing wheelie, `drift` for the drift poses and `reverse`, `reverse_start`, `reverse_loop` and `reverse_cycle` for the
/// walk back, `lean_out` and `straight` for the lean pose, `riding_style` for the lean set, `stand`, `stand_state`, `stand_leg`,
/// `stop_time` and `start_time` for the standstill and `clutch`, `clutch_alpha`, `brake` and `brake_alpha` for the fingers on the
/// levers); a parameter the graph has and this does not know is 0.
pub fn param_values(names: &[String], values: &Values) -> Vec<f32> {
    names
        .iter()
        .map(|name| match name.as_str() {
            "lean" => values.lean,
            "speed_kmh" => values.speed_kmh,
            "tuck" => values.tuck,
            "wheelie" => values.wheelie,
            "stoppie" => values.stoppie,
            "wheelie_alpha" => values.wheelie_alpha,
            "wheelie_direction" => values.wheelie_direction,
            "stoppie_time" => values.stoppie_time,
            "standing" => values.standing,
            "steer" => values.steer,
            "standing_entry" => values.standing_entry,
            "drift" => values.drift,
            "reverse" => values.reverse,
            "reverse_start" => values.reverse_start,
            "reverse_loop" => values.reverse_loop,
            "reverse_cycle" => values.reverse_cycle,
            "lean_out" => values.lean_out,
            "straight" => values.straight,
            "riding_style" => values.riding_style,
            "stand" => values.stand,
            "stand_state" => values.stand_state,
            "stand_leg" => values.stand_leg,
            "stop_time" => values.stop_time,
            "start_time" => values.start_time,
            "clutch" => values.clutch,
            "clutch_alpha" => values.clutch_alpha,
            "brake" => values.brake,
            "brake_alpha" => values.brake_alpha,
            _ => 0.0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_graph_gets_its_parameters_by_name_in_its_own_order() {
        let values = Values {
            lean: -0.5,
            speed_kmh: 90.0,
            tuck: 0.25,
            wheelie: 0.75,
            stoppie: 0.125,
            wheelie_alpha: 0.5,
            wheelie_direction: -1.0,
            stoppie_time: 0.375,
            standing: 0.625,
            steer: -0.875,
            standing_entry: 0.1875,
            drift: 0.0625,
            reverse: 0.9375,
            reverse_start: 1.25,
            reverse_loop: 1.0,
            reverse_cycle: 0.4375,
            lean_out: 0.3125,
            straight: 0.8125,
            riding_style: 2.0,
            stand: 0.28125,
            stand_state: 1.0,
            stand_leg: 2.0,
            stop_time: 1.5,
            start_time: 2.5,
            clutch: 0.34375,
            clutch_alpha: 0.40625,
            brake: 0.53125,
            brake_alpha: 0.59375,
            wrist: 0.71875,
            foot_release: [0.5, 0.25],
        };
        let names: Vec<String> = [
            "speed_kmh",
            "lean",
            "mystery",
            "stoppie",
            "tuck",
            "wheelie",
            "wheelie_alpha",
            "stoppie_time",
            "standing",
            "steer",
            "standing_entry",
            "drift",
            "reverse",
            "reverse_start",
            "reverse_loop",
            "reverse_cycle",
            "lean_out",
            "straight",
            "riding_style",
            "stand",
            "stand_state",
            "stand_leg",
            "stop_time",
            "start_time",
            "clutch",
            "clutch_alpha",
            "brake",
            "brake_alpha",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        assert_eq!(
            param_values(&names, &values),
            vec![
                90.0, -0.5, 0.0, 0.125, 0.25, 0.75, 0.5, 0.375, 0.625, -0.875, 0.1875, 0.0625, 0.9375, 1.25, 1.0,
                0.4375, 0.3125, 0.8125, 2.0, 0.28125, 1.0, 2.0, 1.5, 2.5, 0.34375, 0.40625, 0.53125, 0.59375
            ]
        );
        assert!(param_values(&[], &values).is_empty());
    }
}
