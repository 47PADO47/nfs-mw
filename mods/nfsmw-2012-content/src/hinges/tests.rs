use super::*;

fn vertex(x: f32) -> Vertex {
    Vertex { position: [x, 0.0, 0.0], normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [x, 0.0] }
}

fn body() -> SolidSource {
    // One group of four triangles on five vertices; the middle two belong to part 0 and part 1.
    let vertices = (0..5).map(|n| vertex(n as f32)).collect();
    let indices = vec![0, 1, 2, 1, 2, 3, 2, 3, 4, 0, 3, 4];
    let mut group = GroupSource::new(vertices, indices);
    group.flags = 7;
    let mut solid = SolidSource::new("CAR_KIT00_BODY_A", vec![group]);
    solid.texture_hashes = vec![5];
    solid
}

#[test]
fn a_cut_keeps_every_triangle_exactly_once() {
    let classes = vec![vec![None, Some(0), Some(1), None]];
    let stems = vec!["CAR_KIT00_BONNET".to_owned(), "CAR_KIT00_DOOR".to_owned()];
    let (rest, parts) = split(&body(), &stems, 'A', &classes).unwrap();
    let triangles = |solid: &SolidSource| solid.groups.iter().map(|g| g.indices.len() / 3).sum::<usize>();
    assert_eq!(triangles(&rest), 2);
    let bonnet = parts[0].as_ref().unwrap();
    assert_eq!((bonnet.name.as_str(), triangles(bonnet)), ("CAR_KIT00_BONNET_A", 1));
    assert_eq!(triangles(parts[1].as_ref().unwrap()), 1);
    let group = &bonnet.groups[0];
    assert_eq!(group.flags, 7, "the material of the group is kept");
    assert_eq!(bonnet.texture_hashes, vec![5]);
    let xs: Vec<f32> = group.indices.iter().map(|&i| group.vertices[usize::from(i)].position[0]).collect();
    assert_eq!(xs, vec![1.0, 2.0, 3.0], "the triangle keeps its corners");
    assert_eq!(group.vertices.len(), 3, "only the vertices it uses");
}

#[test]
fn a_part_that_owns_nothing_has_no_solid_and_a_misaligned_cut_is_refused() {
    let stems = vec!["A".to_owned(), "B".to_owned()];
    let (_, parts) = split(&body(), &stems, 'B', &[vec![None, Some(0), None, None]]).unwrap();
    assert!(parts[0].is_some() && parts[1].is_none());
    assert!(split(&body(), &stems, 'A', &[vec![None; 3]]).is_none());
    assert!(split(&body(), &stems, 'A', &[]).is_none());
}

#[test]
fn part_names_become_solid_stems() {
    assert_eq!(stem("BonnetHinge"), "BONNET");
    assert_eq!(stem("FrontLeftDoorHinge"), "FRONT_LEFT_DOOR");
    assert_eq!(stem("LeftMirrorHinge1"), "LEFT_MIRROR1");
    assert_eq!(stem("Hinge"), "HINGE");
}

#[test]
fn the_swing_is_the_largest_end_of_the_widest_axis() {
    // The 911's bonnet turns about x, to -15; its left mirror about y, to 15; its doors about z, to 1.
    assert_eq!(swing(&[[-15.0, 0.0], [0.0, 0.0], [0.0, 0.0]]), Some((0, -15.0)));
    assert_eq!(swing(&[[0.0, 0.0], [-5.0, 15.0], [-15.0, 15.0]]), Some((1, 15.0)));
    assert_eq!(swing(&[[0.0, 0.0], [-15.0, 5.0], [-15.0, 15.0]]), Some((1, -15.0)));
    assert_eq!(swing(&[[0.0, 0.0], [0.0, 0.0], [-1.0, 1.0]]), Some((2, 1.0)));
    assert_eq!(swing(&[[0.0; 2]; 3]), None);
}

#[test]
fn corner_zones_reach_the_parts_beside_them() {
    assert_eq!(zones(1), Some(&["front", "front_left", "front_right"][..]));
    assert_eq!(zones(5), Some(&["front_left"][..]));
    assert_eq!(zones(0), None);
    assert_eq!(zones(9), None);
}

#[test]
fn the_manifest_table_carries_the_swing_in_car_space() {
    use crate::fit::Layout;
    let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
    let part = Part {
        name: "BonnetHinge".into(),
        zone: 1,
        pivot: [-0.7, 0.88, 0.74],
        limits: [[-15.0, 0.0], [0.0; 2], [0.0; 2]],
        thresholds: vec![0.5, 0.04],
        mass_kg: 50.0,
        active: None,
        parent: None,
        delay: None,
        drives: Vec::new(),
        tracks: Vec::new(),
        damage: None,
    };
    let hinged = Hinged { parts: vec![part], wing: None, classes: Vec::new() };
    let text = manifest_tables(&hinged, &["CAR_KIT00_BONNET".into()], &Fit::own(layout));
    let table: toml::Table = toml::from_str(&text).unwrap();
    let entry = &table["damage_parts"][0];
    assert_eq!(entry["name"].as_str(), Some("Bonnet"));
    assert_eq!(entry["angle"].as_float(), Some(-15.0));
    // The lateral axis (2012 x, to the left) is the game's y.
    assert_eq!(entry["axis"][1].as_float(), Some(1.0));
    assert_eq!(entry["pivot"][0].as_float(), Some(0.74), "forward is x");
    assert_eq!(entry["stages"][0].as_float(), Some(0.04), "stages are sorted");
    assert_eq!(entry["mass_kg"].as_float(), Some(50.0), "the part's own mass is the mass of what falls off");
    assert!(entry.get("detach_at").is_none(), "the game's default ladder rule decides when it falls off");
}

#[test]
fn an_active_spoiler_writes_its_movement_and_needs_no_damage_swing() {
    use crate::fit::Layout;
    let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
    let half = 15f32.to_radians();
    let motion = SpoilerMotion {
        bone: 3,
        translation: [0.0, 0.02, -0.07],
        rotation: [half.sin(), 0.0, 0.0, half.cos()],
        seconds: 0.612,
        above_mph: 70.0,
        below_mph: 40.0,
        gas: false,
    };
    let part = Part {
        name: "SpoilerHinge".into(),
        zone: 2,
        pivot: [0.0, 0.78, -1.83],
        limits: [[0.0; 2]; 3],
        thresholds: Vec::new(),
        mass_kg: 30.0,
        active: Some(motion),
        parent: None,
        delay: None,
        drives: Vec::new(),
        tracks: Vec::new(),
        damage: None,
    };
    let hinged = Hinged { parts: vec![part.clone()], wing: None, classes: Vec::new() };
    let text = manifest_tables(&hinged, &["CAR_KIT00_SPOILER".into()], &Fit::own(layout));
    let table: toml::Table = toml::from_str(&text).unwrap();
    let entry = &table["damage_parts"][0];
    assert_eq!((entry["name"].as_str(), entry["angle"].as_float()), (Some("Spoiler"), Some(0.0)));
    let active = &entry["active"];
    // The engine's up (+y) is the game's up (+z); back (-z) is -x.
    assert_eq!(active["offset"][0].as_float(), Some(-0.07));
    assert_eq!(active["offset"][2].as_float(), Some(0.02));
    // The lateral axis (2012 x) is the game's y; the angle is kept.
    assert_eq!(active["rotation_axis"][1].as_float(), Some(1.0));
    assert!((active["rotation_deg"].as_float().unwrap() - 30.0).abs() < 1e-3);
    assert_eq!((active["above_mph"].as_float(), active["below_mph"].as_float()), (Some(70.0), Some(40.0)));
    assert_eq!(active["seconds"].as_float(), Some(0.612));
    // Without a movement, a part with no swing is not written at all.
    let still = Hinged { parts: vec![Part { active: None, ..part }], wing: None, classes: Vec::new() };
    assert!(manifest_tables(&still, &["CAR_KIT00_SPOILER".into()], &Fit::own(layout)).is_empty());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_911_has_hinged_parts_and_an_active_spoiler() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = crate::source::Source::open(std::path::Path::new(&dir)).unwrap();
    let file = source.cars().iter().find(|car| car.number == 1085007).unwrap();
    let car = source.load(file).unwrap();
    let hinged = car.hinged.as_ref().expect("the 911 has hinged parts");
    let names: Vec<&str> = hinged.parts.iter().map(|part| part.name.as_str()).collect();
    // The 8 parts of its damage object and spoiler, and the pieces its damage clips break (lamps, glass, bumper lips).
    assert_eq!(hinged.parts.len(), 22, "{names:?}");
    let spoiler = hinged.parts.iter().find(|part| part.name == "SpoilerHinge").expect("the spoiler");
    let motion = spoiler.active.expect("it moves");
    assert_eq!((motion.above_mph, motion.below_mph), (70.0, 40.0));
    assert!(hinged.parts.iter().filter(|part| part.active.is_some()).count() == 1);
}

#[test]
fn a_flap_is_named_by_the_side_it_moves_to() {
    let motion = |translation: [f32; 3]| SpoilerMotion {
        bone: 1,
        translation,
        rotation: [0.0, 0.0, 0.0, 1.0],
        seconds: 1.0,
        above_mph: 70.0,
        below_mph: 50.0,
        gas: false,
    };
    assert_eq!(attached_name(&motion([0.142, -0.009, 0.0]), 1), "SpoilerFlapLeft");
    assert_eq!(attached_name(&motion([-0.144, -0.009, 0.0]), 2), "SpoilerFlapRight");
    assert_eq!(attached_name(&motion([0.0, 0.05, -0.05]), 3), "SpoilerFlap3");
}

#[test]
fn the_manifest_names_the_part_a_flap_hangs_from() {
    use crate::fit::Layout;
    let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
    let motion = SpoilerMotion {
        bone: 3,
        translation: [0.142, -0.009, 0.002],
        rotation: [0.0, 0.0, 0.0, 1.0],
        seconds: 3.0,
        above_mph: 70.0,
        below_mph: 50.0,
        gas: false,
    };
    let part = Part {
        name: "SpoilerFlapLeft".into(),
        zone: 2,
        pivot: [0.0, 0.78, -1.83],
        limits: [[0.0; 2]; 3],
        thresholds: Vec::new(),
        mass_kg: 30.0,
        active: Some(motion),
        parent: Some("Spoiler".into()),
        delay: None,
        drives: Vec::new(),
        tracks: Vec::new(),
        damage: None,
    };
    let hinged = Hinged { parts: vec![part], wing: None, classes: Vec::new() };
    let text = manifest_tables(&hinged, &["CAR_KIT00_SPOILER_FLAP_LEFT".into()], &Fit::own(layout));
    let table: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(table["damage_parts"][0]["parent"].as_str(), Some("Spoiler"));
    assert!(table["damage_parts"][0]["active"].get("offset").is_some());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_panamera_has_a_spoiler_with_two_flaps_that_hang_from_it() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = crate::source::Source::open(std::path::Path::new(&dir)).unwrap();
    let file = source.cars().iter().find(|car| car.number == 535621).unwrap();
    let car = source.load(file).unwrap();
    let hinged = car.hinged.as_ref().expect("the Panamera has hinged parts");
    let active: Vec<(&str, Option<&str>)> = hinged
        .parts
        .iter()
        .filter(|part| part.active.is_some())
        .map(|part| (part.name.as_str(), part.parent.as_deref()))
        .collect();
    assert_eq!(active.len(), 3, "{active:?}");
    assert_eq!(active.iter().filter(|(_, parent)| *parent == Some("Spoiler")).count(), 2, "{active:?}");
}

fn layout() -> crate::fit::Layout {
    crate::fit::Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 }
}

/// The hinged parts of the car `number` of the 2012 install and the manifest tables the converter writes for them.
fn manifest_of(number: u32) -> Option<(Hinged, blackbox_modding::content::CarManifest)> {
    let dir = std::env::var_os("MW2012_GAME_DIR")?;
    let source = crate::source::Source::open(std::path::Path::new(&dir)).unwrap();
    let file = source.cars().iter().find(|car| car.number == number).unwrap();
    let car = source.load(file).unwrap();
    let hinged = car.hinged?;
    let stems: Vec<String> = hinged.parts.iter().map(|part| format!("CAR_KIT00_{}", stem(&part.name))).collect();
    let text = manifest_tables(&hinged, &stems, &Fit::own(layout()));
    let manifest = blackbox_modding::content::parse_car("CAR", &text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    Some((hinged, manifest))
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_veyron_writes_a_wing_with_five_animations_that_the_game_accepts() {
    let Some((hinged, manifest)) = manifest_of(122692) else { return };
    let wing = manifest.wing.expect("the wing machine");
    assert_eq!(
        (wing.open_mph, wing.brake_mph, wing.top_mph, wing.brake_hold_seconds, wing.fade_seconds),
        (140.0, 125.0, Some(200.0), 0.4, 0.25)
    );
    assert!(wing.slot_seconds.iter().all(|s| *s > 1.0), "{:?}", wing.slot_seconds);
    let tracked: Vec<usize> =
        manifest.damage_parts.iter().map(|part| part.tracks.len()).filter(|tracks| *tracks > 0).collect();
    assert_eq!(
        tracked,
        vec![5, 4, 4, 5],
        "the wing and its three arms, with a key set for each animation that moves them"
    );
    assert_eq!(tracked.len(), hinged.parts.iter().filter(|part| !part.tracks.is_empty()).count());
    assert!(manifest.damage_parts.iter().any(|part| part.parent.is_some()), "the arms hang from the wing");
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_agera_follows_the_gas_and_moves_two_flaps() {
    let Some((_, manifest)) = manifest_of(589261) else { return };
    let active: Vec<_> = manifest.damage_parts.iter().filter_map(|part| part.active).collect();
    assert_eq!(active.len(), 2, "two flaps");
    assert!(active.iter().all(|a| a.gas && a.above_mph == 70.0 && a.below_mph == 0.0));
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_air_vents_write_their_waits_and_the_game_accepts_them() {
    let Some((_, marussia)) = manifest_of(1551590) else { return };
    let vents: Vec<_> = marussia.damage_parts.iter().filter_map(|part| part.active).collect();
    assert!(vents.len() >= 2, "{} vents", vents.len());
    assert!(vents.iter().all(|a| a.delay == Some((3.0, 5.0)) && a.above_mph == 120.0 && a.below_mph == 100.0));
    let Some((_, concept)) = manifest_of(1085830) else { return };
    let waits: Vec<_> = concept.damage_parts.iter().filter_map(|part| part.active).map(|a| a.delay.is_some()).collect();
    assert!(
        waits.contains(&true) && waits.contains(&false),
        "the spoiler follows the speed, the vents wait: {waits:?}"
    );
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_mclaren_and_the_venom_write_a_wing_with_three_animations_and_the_numbers_of_their_data() {
    for number in [122765, 2076137] {
        let Some((_, manifest)) = manifest_of(number) else { return };
        let wing = manifest.wing.expect("the wing machine");
        assert_eq!(
            (wing.open_mph, wing.brake_mph, wing.top_mph, wing.brake_hold_seconds, wing.fade_seconds),
            (120.0, 80.0, None, 0.4, 0.25)
        );
        let slots: std::collections::BTreeSet<usize> =
            manifest.damage_parts.iter().flat_map(|part| part.tracks.iter().map(|t| t.slot)).collect();
        assert_eq!(slots.into_iter().collect::<Vec<_>>(), vec![0, 1, 2], "three animations move the parts");
    }
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_huayra_writes_four_flaps_with_their_drives_and_the_game_accepts_them() {
    let Some((_, manifest)) = manifest_of(535435) else { return };
    let flaps: Vec<_> = manifest.damage_parts.iter().filter(|part| !part.drives.is_empty()).collect();
    assert_eq!(flaps.len(), 4, "four flaps");
    let counts: std::collections::BTreeMap<&str, usize> =
        flaps.iter().map(|p| (p.name.as_str(), p.drives.len())).collect();
    assert_eq!(counts["FrontLeftFlap"], 2, "turning and the speed band: {counts:?}");
    assert_eq!(counts["FrontRightFlap"], 2);
    assert_eq!(counts["RearLeftFlap"], 3, "turning, the speed band and the brake");
    assert_eq!(counts["RearRightFlap"], 3);
    let rear = flaps.iter().find(|p| p.name == "RearRightFlap").unwrap();
    assert!(rear.drives.iter().any(|d| d.open_after_seconds == 0.4 && d.close_any), "the brake drive waits 0.4 s");
    let right_turn = rear
        .drives
        .iter()
        .find(|d| d.open.iter().any(|t| t.input == blackbox_modding::content::DriveInput::Steering))
        .unwrap();
    assert!(
        right_turn.open.iter().any(|t| t.op == blackbox_modding::content::DriveOp::Gt && t.value == 0.3),
        "{right_turn:?}"
    );
}

fn damaged(damage: PartDamage) -> Part {
    Part {
        name: "FrontBumperHinge".into(),
        zone: 1,
        pivot: [0.56, 0.66, 1.85],
        limits: [[0.0; 2]; 3],
        thresholds: vec![0.04, 0.05, 0.5, 0.75],
        mass_kg: 50.0,
        active: None,
        parent: None,
        delay: None,
        drives: Vec::new(),
        tracks: Vec::new(),
        damage: Some(damage),
    }
}

fn written(part: Part) -> toml::Table {
    let hinged = Hinged { parts: vec![part], wing: None, classes: Vec::new() };
    let text = manifest_tables(&hinged, &["CAR_KIT00_FRONT_BUMPER".into()], &Fit::own(layout()));
    blackbox_modding::content::parse_car("CAR", &text).unwrap_or_else(|e| panic!("the game refuses it: {e}\n{text}"));
    toml::from_str(&text).unwrap()
}

#[test]
fn the_damage_clip_writes_the_turn_of_each_level_and_where_the_part_breaks_off() {
    use crate::damage_model::{Release, Swing};
    let swing = Swing { axis: [1.0, 0.0, 0.0], angles: [-3.0, -6.0, -9.0] };
    let table = written(damaged(PartDamage { swing: Some(swing), release: Release::At(0.75) }));
    let entry = &table["damage_parts"][0];
    assert_eq!(entry["angle"].as_float(), Some(-9.0), "the last level");
    assert_eq!(entry["axis"][1].as_float(), Some(1.0), "the engine's left is the game's y");
    let stages: Vec<f64> = entry["stages"].as_array().unwrap().iter().filter_map(|s| s.as_float()).collect();
    assert_eq!(stages.len(), 3, "the clip's levels replace the ladder of the damage object: {stages:?}");
    assert_eq!(entry["stage_angles"][1].as_float(), Some(-6.0));
    assert_eq!(entry["detach_at"].as_float(), Some(0.75));
    let stays = written(damaged(PartDamage { swing: Some(swing), release: Release::Never }));
    assert_eq!(stays["damage_parts"][0]["detach"].as_bool(), Some(false));
    assert!(stays["damage_parts"][0].get("detach_at").is_none());
}

#[test]
fn a_piece_that_only_breaks_off_is_written_without_a_turn() {
    use crate::damage_model::Release;
    let table = written(damaged(PartDamage { swing: None, release: Release::At(0.25) }));
    let entry = &table["damage_parts"][0];
    assert_eq!(entry["angle"].as_float(), Some(0.0));
    assert!(entry.get("stage_angles").is_none());
    assert_eq!(entry["detach_at"].as_float(), Some(0.25));
    let default = written(damaged(PartDamage { swing: None, release: Release::Default }));
    assert!(
        default["damage_parts"][0].get("detach_at").is_none() && default["damage_parts"][0].get("detach").is_none()
    );
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_911_breaks_pieces_off_by_its_clips_and_the_game_accepts_the_tables() {
    let Some((hinged, manifest)) = manifest_of(1085007) else { return };
    let parts = &manifest.damage_parts;
    let find = |name: &str| parts.iter().find(|part| part.name == name);
    assert_eq!(find("FrontBumper").unwrap().detach_at, Some(1.0), "the bumper drops at full damage");
    assert!(!find("Bonnet").unwrap().detach, "the bonnet only tilts");
    assert_eq!(find("LeftMirror").unwrap().detach_at, Some(0.25));
    assert!(
        parts.iter().any(|part| part.name.starts_with("Piece") && part.detach_at == Some(0.25)),
        "lamps break at 0.25"
    );
    let pieces = parts.iter().filter(|part| part.name.starts_with("Piece")).count();
    assert!(
        pieces >= 6 && hinged.parts.len() >= parts.len(),
        "lamps, glass and lips of the bumpers are parts of their own: {pieces}"
    );
    assert!(parts.len() <= blackbox_modding::content::MAX_DAMAGE_PARTS);
}
