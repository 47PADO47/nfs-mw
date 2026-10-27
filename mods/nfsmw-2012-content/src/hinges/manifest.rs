//! The `[[damage_parts]]` tables of the parts of a car, written for the manifest.

use super::*;

/// The `[[damage_parts]]` tables for the parts of `hinged`, whose solids are `stems` (the same order), moved into the
/// car's space by `fit`.
pub fn manifest_tables(hinged: &Hinged, stems: &[String], fit: &Fit) -> String {
    let mut text = String::new();
    for (part, stem) in hinged.parts.iter().zip(stems) {
        let Some(zones) = zones(part.zone) else { continue };
        let still = part.active.is_some() || !part.tracks.is_empty();
        let Some(turn) = turn_of(part, still) else { continue };
        let (axis, angle, stages) = (to_car_space(turn.axis), turn.angle, turn.stages);
        let pivot = fit.apply(to_car_space(part.pivot));
        let name = label(&part.name);
        let zone_list: Vec<String> = zones.iter().map(|zone| format!("\"{zone}\"")).collect();
        text.push_str("\n[[damage_parts]]\n");
        text.push_str(&format!("name = \"{name}\"\nmodel = \"{stem}\"\n"));
        text.push_str(&format!("zones = [{}]\n", zone_list.join(", ")));
        text.push_str(&format!("pivot = [{:.5}, {:.5}, {:.5}]\n", pivot[0], pivot[1], pivot[2]));
        text.push_str(&format!("axis = [{:.4}, {:.4}, {:.4}]\nangle = {angle:.2}\n", axis[0], axis[1], axis[2]));
        text.push_str(&format!("mass_kg = {:.1}\n", part.mass_kg.clamp(0.5, 5000.0)));
        if !stages.is_empty() {
            let list: Vec<String> = stages.iter().map(|s| format!("{s:.4}")).collect();
            text.push_str(&format!("stages = [{}]\n", list.join(", ")));
        }
        if !turn.stage_angles.is_empty() {
            let list: Vec<String> = turn.stage_angles.iter().map(|a| format!("{a:.3}")).collect();
            text.push_str(&format!("stage_angles = [{}]\n", list.join(", ")));
        }
        text.push_str(&release_text(part.damage.map(|damage| damage.release)));
        if let Some(parent) = &part.parent {
            text.push_str(&format!("parent = \"{parent}\"\n"));
        }
        if let Some(motion) = part.active {
            text.push_str(&active_table(&motion, part.delay, fit));
        }
        for drive in part.drives.iter().filter(|drive| convertible(drive)) {
            text.push_str(&drive_table(drive));
        }
        for track in &part.tracks {
            text.push_str(&track_table(track, fit));
        }
    }
    if let Some(machine) = hinged.wing.as_ref().filter(|_| hinged.parts.iter().any(|part| !part.tracks.is_empty())) {
        text.push_str(&wing_table(machine));
    }
    text
}

/// How a part turns with the damage, in the 2012 space.
struct Turn {
    axis: [f32; 3],
    angle: f32,
    stages: Vec<f32>,
    /// The turn at each stage; empty when the stages turn the part evenly.
    stage_angles: Vec<f32>,
}

/// The turn of `part`: from the car's damage clip at the three panel levels when it turns by it (a part that the speed moves
/// does not turn by damage), else from the range of its damage object swept over its ladder. `None` for a part nothing turns.
fn turn_of(part: &Part, still: bool) -> Option<Turn> {
    if let Some(swing) = part.damage.and_then(|damage| damage.swing).filter(|_| !still) {
        let (stages, stage_angles) = (damage_model::STAGES.to_vec(), swing.angles.to_vec());
        return Some(Turn { axis: swing.axis, angle: swing.angles[2], stages, stage_angles });
    }
    let (index, angle) = swing(&part.limits).or((still || part.damage.is_some()).then_some((0, 0.0)))?;
    let mut axis = [0.0f32; 3];
    axis[index] = 1.0;
    let mut stages: Vec<f32> = part.thresholds.iter().map(|t| t.clamp(0.0, 1.0)).collect();
    stages.sort_by(f32::total_cmp);
    stages.truncate(MAX_STAGES);
    Some(Turn { axis, angle, stages, stage_angles: Vec::new() })
}

/// The release lines of a part: where it falls off, or that it never does; nothing for the game's default.
fn release_text(release: Option<damage_model::Release>) -> String {
    match release {
        Some(damage_model::Release::At(at)) => format!("detach_at = {at:.4}\n"),
        Some(damage_model::Release::Never) => "detach = false\n".to_owned(),
        _ => String::new(),
    }
}

/// The manifest name of an input of the data, `None` for one the game does not offer (the cop lights).
fn input_name(input: &str) -> Option<&'static str> {
    match input {
        "speedinmph" => Some("speed_mph"),
        "gas" => Some("gas"),
        "brake" => Some("brake"),
        "steering" => Some("steering"),
        _ => None,
    }
}

/// The comparison of an operator code of the data: 0 equal, 1 not equal, 2 greater, 3 less, 4 at least, 5 at most.
fn operator_symbol(code: &str) -> Option<&'static str> {
    ["==", "!=", ">", "<", ">=", "<="].get(code.parse::<usize>().ok()?).copied()
}

/// Whether every test of the drive can be written.
pub(super) fn convertible(drive: &DriveSpec) -> bool {
    let known = |guard: &Guard| {
        guard.tests.iter().all(|t| input_name(&t.input).is_some() && operator_symbol(&t.operator).is_some())
    };
    known(&drive.open) && known(&drive.close)
}

fn tests_text(guard: &Guard) -> String {
    let tests: Vec<String> = guard
        .tests
        .iter()
        .map(|t| {
            let (input, op) = (input_name(&t.input).unwrap_or("gas"), operator_symbol(&t.operator).unwrap_or("=="));
            format!("{{ input = \"{input}\", op = \"{op}\", value = {} }}", t.value)
        })
        .collect();
    format!("[{}]", tests.join(", "))
}

/// The `[[damage_parts.drive]]` table of a guarded animation.
fn drive_table(drive: &DriveSpec) -> String {
    let mut text = format!("\n[[damage_parts.drive]]\nopen = {}\n", tests_text(&drive.open));
    if drive.open.any {
        text.push_str("open_any = true\n");
    }
    if drive.open_hold > 0.0 {
        text.push_str(&format!("open_after_seconds = {}\n", drive.open_hold));
    }
    text.push_str(&format!("close = {}\n", tests_text(&drive.close)));
    if drive.close.any {
        text.push_str("close_any = true\n");
    }
    text
}

/// The name of a flap by where its bone sits (the engine's x is to the left and z forward).
pub(super) fn flap_name(position: &[f32; 3]) -> String {
    let side = if position[0] > 0.0 { "Left" } else { "Right" };
    let end = if position[2] > 0.0 { "Front" } else { "Rear" };
    format!("{end}{side}Flap")
}

/// The `[[damage_parts.track]]` table of one animation of a part: the keys in the car's space (the cyclic change of axes
/// keeps the handedness, so a quaternion carries over with its vector part permuted).
fn track_table(track: &WingBoneTrack, fit: &Fit) -> String {
    let slide: Vec<String> = track
        .translation
        .iter()
        .map(|key| {
            let moved = to_car_space(*key);
            let [x, y, z] = [0, 1, 2].map(|k| moved[k] * fit.scale[k]);
            format!("[{x:.5}, {y:.5}, {z:.5}]")
        })
        .collect();
    let turn: Vec<String> = track
        .rotation
        .iter()
        .map(|key| {
            let [x, y, z] = to_car_space([key[0], key[1], key[2]]);
            format!("[{x:.6}, {y:.6}, {z:.6}, {:.6}]", key[3])
        })
        .collect();
    format!(
        "\n[[damage_parts.track]]\nslot = {}\ntranslation = [{}]\nrotation = [{}]\n",
        track.slot,
        slide.join(", "),
        turn.join(", ")
    )
}

/// The `[wing]` table: the numbers of the machine the game plays and the length of the five animations.
fn wing_table(machine: &WingMachine) -> String {
    let seconds: Vec<String> = machine.seconds.iter().map(|s| format!("{s:.4}")).collect();
    format!(
        "\n[wing]\nopen_mph = {:.1}\nbrake_mph = {:.1}\n{}brake_hold_seconds = {:.2}\nfade_seconds = {:.2}\nslot_seconds = [{}]\n",
        machine.params.open_mph,
        machine.params.brake_mph,
        machine.params.top_mph.map_or(String::new(), |top| format!("top_mph = {top:.1}\n")),
        machine.params.brake_hold_seconds,
        machine.params.fade_seconds,
        seconds.join(", ")
    )
}

/// The `[damage_parts.active]` table of a spoiler: its slide in the car's space (the fit scales it with the body), its
/// turn about the pivot (the cyclic change of axes keeps the handedness, so the axis and the angle carry over), the
/// speeds and the time. All of them are what the car's data says.
fn active_table(motion: &SpoilerMotion, delay: Option<(f32, f32)>, fit: &Fit) -> String {
    let moved = to_car_space(motion.translation);
    let offset = [0, 1, 2].map(|k| moved[k] * fit.scale[k]);
    let mut text = String::from("\n[damage_parts.active]\n");
    if motion.gas {
        text.push_str("gas = true\n");
    }
    if let Some((open, close)) = delay {
        text.push_str(&format!("open_after_seconds = {open:.2}\nclose_after_seconds = {close:.2}\n"));
    }
    if offset.iter().any(|c| c.abs() >= 1e-4) {
        text.push_str(&format!("offset = [{:.5}, {:.5}, {:.5}]\n", offset[0], offset[1], offset[2]));
    }
    if let Some((axis, degrees)) = motion.rotation_axis_angle() {
        let axis = to_car_space(axis);
        text.push_str(&format!(
            "rotation_axis = [{:.5}, {:.5}, {:.5}]\nrotation_deg = {degrees:.3}\n",
            axis[0], axis[1], axis[2]
        ));
    }
    text.push_str(&format!(
        "above_mph = {:.1}\nbelow_mph = {:.1}\nseconds = {:.3}\n",
        motion.above_mph, motion.below_mph, motion.seconds
    ));
    text
}
