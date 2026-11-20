use super::*;

fn seat(name: &str, car: Option<&str>, stock: bool) -> Seat {
    Seat { name: name.into(), car: car.map(str::to_owned), stock }
}

fn empty_report() -> SceneReport {
    SceneReport {
        scene: "Test".into(),
        description: String::new(),
        kind: String::new(),
        scene_type: String::new(),
        number_of_cars: 0,
        vanish_frame: 0,
        seconds: 1.0,
        overlay: String::new(),
        camera: CameraReport { asked: 0, tracks: 1, plays: Some("Track 0".into()), seconds: Some(1.0) },
        channels: Vec::new(),
        character_records: 0,
        characters_drawn: 0,
        rev_curves: false,
        audio_take: None,
        audio_exact: false,
        frames_played: 0,
        outcome: String::new(),
        events: Vec::new(),
        unhandled: BTreeMap::new(),
        problems: Vec::new(),
        open: Vec::new(),
    }
}

#[test]
fn channels_are_named_as_the_bundles_name_them() {
    assert_eq!(channel_name(0), "car1");
    assert_eq!(channel_name(7), "car8");
    assert_eq!(channel_name(8), "cop1");
    assert_eq!(channel_name(15), "cop8");
}

#[test]
fn a_race_start_channel_drives_the_racer_of_its_grid_slot() {
    let seating = Seating {
        race_start: true,
        grid: vec![0, 1],
        seats: vec![seat("player", Some("BMWM3GTRE46:M3GTRCAREERSTART"), false), seat("razor", Some("mustang"), false)],
        listed: 3,
    };
    let mut report = empty_report();
    assert_eq!(bind(0, NisKind::Intro, &seating, "car1", None, &mut report), "player (BMWM3GTRE46:M3GTRCAREERSTART)");
    assert_eq!(bind(1, NisKind::Intro, &seating, "car2", None, &mut report), "racer 1 razor (mustang)");
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(bind(2, NisKind::Intro, &seating, "car3", None, &mut report), "none");
    assert_eq!(report.problems.len(), 1, "a channel the data has a racer for is a problem");
    assert_eq!(bind(3, NisKind::Intro, &seating, "car4", None, &mut report), "none");
    assert_eq!((report.problems.len(), report.open.len()), (1, 1), "one past the data's racers is open");
}

#[test]
fn a_stock_pick_is_a_problem_and_a_traffic_car_is_open() {
    let seats = vec![seat("p", Some("a"), false), seat("r", Some("b"), true)];
    let seating = Seating { race_start: true, grid: vec![0, 1], seats, listed: 2 };
    let mut report = empty_report();
    bind(1, NisKind::Intro, &seating, "car2", None, &mut report);
    assert_eq!(report.problems.len(), 1);
    let mut report = empty_report();
    assert_eq!(bind(1, NisKind::Outro, &Seating::default(), "car2", None, &mut report), "nearest traffic car");
    assert!(report.problems.is_empty());
    assert_eq!(report.open.len(), 1);
}

#[test]
fn events_after_the_end_are_a_problem() {
    let mut report = empty_report();
    report.events.push(EventReport { at: 26.3, event: "ENISOverlayMessage".into(), handled: false, after_end: true });
    event_findings(&mut report);
    assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
}

#[test]
fn event_names_come_from_the_spec_table() {
    assert_eq!(event_name(0x257A_10A8), "ENISDetach");
    assert_eq!(event_name(0x1234_5678), "12345678");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn black_fade_out_scripts_print_their_colour_keys() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(std::path::PathBuf::from(dir)).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    let package = catalog.find("BLACK_FADE_OUT.fng").expect("BLACK_FADE_OUT.fng");
    for object in &package.objects {
        for script in &object.scripts {
            for track in &script.tracks {
                let keys: Vec<String> = track.keys.iter().map(|k| format!("{}:{:?}", k.time, k.value)).collect();
                println!(
                    "object {:#x} script {:#010x} len {} {:?} base {:?} keys {}",
                    object.guid,
                    script.id,
                    script.length,
                    track.param,
                    track.base.value,
                    keys.join(" ")
                );
            }
        }
    }
}

#[test]
fn a_cop_channel_of_a_listed_scene_names_its_list_vehicle_or_none() {
    let mut report = empty_report();
    let listed = Some(Some("trafminivan"));
    let binding = bind(14, NisKind::Intro, &Seating::default(), "cop7", listed, &mut report);
    assert_eq!(binding, "trafminivan of the scene's special car list");
    assert_eq!(bind(9, NisKind::Arrest, &Seating::default(), "cop2", Some(None), &mut report), "none");
    assert!(report.open.is_empty());
}
