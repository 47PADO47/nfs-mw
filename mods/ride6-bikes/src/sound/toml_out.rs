//! The `[engine_sound]` and `[car_sound]` tables of a bike's manifest, as text (`docs/modding/car-sound.md`), plus the
//! `[car_sound.overrun]` extension (`docs/specs/ride6-engine-sound.md`, section "Manifest extension").

use std::fmt::Write;

use super::data::OverrunParams;

/// One engine layer as the manifest names it.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerEntry {
    pub file: String,
    pub rpm: f32,
}

/// The lists of `[car_sound]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectLists {
    pub shift_up: Vec<String>,
    pub shift_down: Vec<String>,
    pub backfire: Vec<String>,
    pub pops: Vec<String>,
    /// `Some(false)` writes `shift_clunk = false`.
    pub shift_clunk: Option<bool>,
    /// Writes `shift_trigger = "gear"`: the shifts play at every change of the gear number (spec section 3).
    pub shift_on_gear: bool,
}

fn list(name: &str, files: &[String]) -> Option<String> {
    if files.is_empty() {
        return None;
    }
    let items: Vec<String> = files.iter().map(|file| format!("\"{file}\"")).collect();
    Some(format!("{name} = [{}]\n", items.join(", ")))
}

/// `[engine_sound]` with its layers, slowest first.
pub fn engine_sound(idle_rpm: f32, redline_rpm: f32, layers: &[LayerEntry]) -> String {
    let mut text = String::from("[engine_sound]\n");
    let _ = writeln!(text, "idle_rpm = {idle_rpm:?}");
    let _ = writeln!(text, "redline_rpm = {redline_rpm:?}");
    text.push_str("layers = [\n");
    for layer in layers {
        let _ = writeln!(text, "  {{ file = \"{}\", rpm = {:?} }},", layer.file, layer.rpm);
    }
    text.push_str("]\n");
    text
}

/// `[car_sound]` (empty text when nothing is named and the clunk is left alone), then the optional overrun table.
pub fn car_sound(lists: &EffectLists, overrun: Option<&OverrunParams>) -> String {
    let mut body = String::new();
    for (name, files) in [
        ("shift_up", &lists.shift_up),
        ("shift_down", &lists.shift_down),
        ("backfire", &lists.backfire),
        ("pops", &lists.pops),
    ] {
        body.extend(list(name, files));
    }
    if let Some(clunk) = lists.shift_clunk {
        let _ = writeln!(body, "shift_clunk = {clunk}");
    }
    if lists.shift_on_gear {
        body.push_str("shift_trigger = \"gear\"\n");
    }
    let mut text = String::new();
    if !body.is_empty() {
        text.push_str("[car_sound]\n");
        text.push_str(&body);
    }
    let Some(params) = overrun else { return text };
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(&overrun_table(params));
    text
}

/// `[car_sound.overrun]`: the accumulator backfire of the spec. The key names are the ones the spec defines.
pub fn overrun_table(p: &OverrunParams) -> String {
    let mut text = String::from("[car_sound.overrun]\n");
    let rows: [(&str, f32); 11] = [
        ("accumulator_rate", p.accumulator_rate),
        ("throttle_arm", p.throttle_arm),
        ("throttle_fire", p.throttle_fire),
        ("window_seconds", p.window_seconds),
        ("flame_delay", p.flame_delay),
        ("flame_min_rpm", p.flame_min_rpm),
        ("bang_audio_threshold", p.bang_audio_threshold),
        ("bang_effect_threshold", p.bang_effect_threshold),
        ("flame_effect_threshold", p.flame_effect_threshold),
        ("probability_closed", p.probability_closed),
        ("probability_open", p.probability_open),
    ];
    for (key, value) in rows {
        let _ = writeln!(text, "{key} = {value:?}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> OverrunParams {
        OverrunParams {
            simulation_enabled: true,
            new_system: true,
            accumulator_rate: 0.2,
            throttle_arm: 0.9,
            throttle_fire: 0.1,
            window_seconds: 2.0,
            flame_delay: 0.2,
            flame_min_rpm: 0.1,
            bang_audio_threshold: 0.95,
            bang_effect_threshold: 0.8,
            flame_effect_threshold: 1.0,
            probability_closed: 0.8,
            probability_open: 0.3,
        }
    }

    #[test]
    fn the_engine_table_lists_its_layers() {
        let text = engine_sound(2300.0, 11000.0, &[LayerEntry { file: "CARS/X/SOUND/ENG0.WAV".into(), rpm: 3000.0 }]);
        assert!(text.starts_with("[engine_sound]\nidle_rpm = 2300.0\nredline_rpm = 11000.0\n"));
        assert!(text.contains("{ file = \"CARS/X/SOUND/ENG0.WAV\", rpm = 3000.0 },"));
    }

    #[test]
    fn an_empty_car_sound_is_empty_text() {
        assert_eq!(car_sound(&EffectLists::default(), None), "");
    }

    #[test]
    fn the_gear_trigger_alone_is_a_table() {
        let lists = EffectLists { shift_on_gear: true, ..EffectLists::default() };
        assert_eq!(car_sound(&lists, None), "[car_sound]\nshift_trigger = \"gear\"\n");
    }

    #[test]
    fn lists_clunk_and_overrun_are_written() {
        let lists = EffectLists {
            backfire: vec!["CARS/X/SOUND/BACKFIRE0.WAV".into(), "CARS/X/SOUND/BACKFIRE1.WAV".into()],
            shift_clunk: Some(false),
            ..EffectLists::default()
        };
        let text = car_sound(&lists, Some(&params()));
        assert!(
            text.starts_with(
                "[car_sound]\nbackfire = [\"CARS/X/SOUND/BACKFIRE0.WAV\", \"CARS/X/SOUND/BACKFIRE1.WAV\"]\n"
            )
        );
        assert!(text.contains("shift_clunk = false\n"));
        assert!(text.contains("\n[car_sound.overrun]\naccumulator_rate = 0.2\n"));
        assert!(text.contains("probability_open = 0.3\n"));
    }
}
