//! The Quick Play brief against the install: the package accepts every object the screen fills, Randomize, Accept and Back do
//! what the spec says, and the labels and textures the screen names exist. Skipped without `NFSMW_GAME_DIR`.

use blackbox_feng::fe_hash_upper;
use blackbox_text::label_hash;
use nfsmw_data::gameplay::RaceType;

use super::*;
use crate::frontend::career_screens::car_lot_bars::strip_fills;
use crate::frontend::look::Look;
use crate::frontend::quick_play::pool::{self, CarPool};
use crate::frontend::quick_play::{fill, text};
use crate::frontend::quick_race::{self, Board};
use crate::ui::Text;

/// Key 1 presses pad buttons 1 and 5; button 5 is the message `0xC519BFC4` the brief answers.
const RANDOMIZE_KEY: u32 = pad::BUTTON0 << 5;

/// The install's directory, when the tests have one.
fn install() -> Option<GameDir> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(root)).unwrap())
}

/// The cars and the board, read before the screen opens (the front end reads them in the background).
fn sources(dir: &GameDir) -> (Arc<CarPool>, Arc<Board>) {
    let cars = pool::load_now(dir);
    quick_race::load_in_background(dir);
    for _ in 0..2400 {
        if let Some(board) = quick_race::board() {
            return (cars, board);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("the quick race board was not read");
}

fn open() -> Option<(Harness, Arc<CarPool>, Arc<Board>)> {
    let dir = install()?;
    let (cars, board) = sources(&dir);
    let mut h = Harness::open(screen::QUICK_RACE_BRIEF, Args::default())?;
    h.wait(1.5);
    Some((h, cars, board))
}

/// The folder of the car the backdrop shows.
fn shown_folder(h: &Harness) -> String {
    match h.screens.look() {
        Some(Look::Car(look)) => look.car,
        other => panic!("the brief should show a car, not {other:?}"),
    }
}

fn node_text(h: &Harness, name_hash: u32) -> Option<String> {
    let tree = h.screens.trees().pop()?;
    tree.nodes.iter().find(|n| n.name_hash == name_hash).and_then(|n| n.text.clone())
}

fn node(h: &Harness, guid: u32) -> blackbox_feng::UiNode {
    let tree = h.screens.trees().pop().expect("a tree");
    tree.nodes.into_iter().find(|n| n.guid == guid).unwrap_or_else(|| panic!("the package has no object {guid:#x}"))
}

fn texture(h: &Harness, guid: u32) -> u32 {
    match node(h, guid).kind {
        blackbox_feng::NodeKind::Image { texture, .. } => texture,
        other => panic!("{guid:#x} should be an image, not {other:?}"),
    }
}

fn stage() -> u8 {
    crate::career::CareerHandle::shared().with_loaded(|career| career.stage()).unwrap_or(pool::PROLOGUE_STAGE)
}

#[test]
fn the_brief_fills_every_object_it_owns() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((h, cars, _)) = open() else { return };
    let folder = shown_folder(&h);
    let car = cars.cars.iter().find(|c| c.folder == folder).expect("the shown car is in the pool");
    // The race: a name that is not the package's placeholder, a length in the metric unit, the laps and the opponents.
    let name = node_text(&h, fill::TRACK_NAME).expect("the track name");
    assert!(!name.is_empty() && name != "Sidewinder", "track name {name:?}");
    let length = node_text(&h, fill::LENGTH).expect("the length");
    let (number, unit) = length.split_once(' ').expect("number and unit");
    assert!(number.parse::<f32>().is_ok(), "{length:?}");
    assert_eq!(unit, "KMs", "the default units are metric: {length:?}");
    assert!(node_text(&h, fill::LAPS).and_then(|t| t.parse::<u32>().ok()).is_some_and(|n| n >= 1));
    assert!(node_text(&h, fill::OPPONENTS).and_then(|t| t.parse::<usize>().ok()).is_some());
    // The kind: its name on the text and its shadow, and one of the six pictures.
    let kind = node_text(&h, 0xBEB9_E79D).expect("the mode text");
    assert_eq!(Some(kind.clone()), node_text(&h, 0x4FA2_2DFC), "the shadow says the same");
    assert!(!kind.is_empty() && kind != "Circuit - K.O.", "mode {kind:?}");
    let icons = [
        RaceType::Sprint,
        RaceType::Circuit,
        RaceType::Drag,
        RaceType::Knockout,
        RaceType::Tollbooth,
        RaceType::SpeedTrap,
    ]
    .map(text::mode_icon);
    assert!(icons.contains(&texture(&h, fill::MODE_ICON)));
    // The car: the logos are swapped for textures the install has, and the unlock line follows the stage.
    let assets = UiAssets::load(&install().unwrap()).unwrap();
    for guid in [fill::MANUFACTURER_LOGO, fill::MODEL_LOGO] {
        assert!(assets.texture(texture(&h, guid)).is_some(), "logo {guid:#x} of {}", car.name);
    }
    let unlock = text::unlock_of(car, stage());
    assert_eq!(node(&h, fill::UNLOCK_GROUP).visible, unlock != text::Unlock::None, "{} at stage {}", car.name, stage());
    // The bars: the full strip is as wide as the car's fraction of the pool says.
    let [top, _, _] =
        car.stats.map_or([0.0; 3], |own| nfsmw_career::carlot::stats::bar_fractions(&own, &cars.population));
    let strip = node(&h, fill::TOP_SPEED[1].guid);
    let fills = strip_fills(top, &fill::TOP_SPEED);
    assert_eq!(strip.visible, fills[1].shown, "{} top speed {top}", car.name);
    if fills[1].shown {
        assert!((strip.size.x - fills[1].width).abs() < 0.5, "{} vs {}", strip.size.x, fills[1].width);
    }
}

#[test]
fn the_option_column_and_the_comparison_ticks_stay_hidden() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((h, _, _)) = open() else { return };
    for guid in fill::OPTION_COLUMN.into_iter().chain(fill::DELTA_TICKS) {
        assert!(!node(&h, guid).visible, "{guid:#x} should be hidden");
    }
}

#[test]
fn the_backdrop_look_is_the_car_the_brief_picked() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((h, cars, _)) = open() else { return };
    let folder = shown_folder(&h);
    assert!(cars.cars.iter().any(|c| c.folder == folder));
    assert!(nfsmw_data::car::list(&install().unwrap()).contains(&folder), "{folder} is under CARS/");
}

#[test]
fn randomize_picks_another_pair_and_the_screen_follows() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((mut h, _, _)) = open() else { return };
    let before = (shown_folder(&h), node_text(&h, fill::TRACK_NAME), node_text(&h, fill::LENGTH));
    h.press(RANDOMIZE_KEY);
    h.wait(0.5);
    let after = (shown_folder(&h), node_text(&h, fill::TRACK_NAME), node_text(&h, fill::LENGTH));
    assert_ne!(before, after, "key 1 picks again");
}

#[test]
fn accept_starts_the_pick_after_the_leave_script() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((mut h, _, board)) = open() else { return };
    let folder = shown_folder(&h);
    h.press(pad::ACCEPT);
    h.wait(3.0);
    let started = h.commands.iter().find_map(|c| match c {
        Command::StartQuickPlay { race, car } => Some((race.clone(), car.clone())),
        _ => None,
    });
    let (race, car) = started.unwrap_or_else(|| panic!("Accept should start the race: {:?}", h.commands));
    assert_eq!(car, folder);
    assert!(board.tracks.iter().any(|t| t.name == race), "{race} is a track of the board");
}

#[test]
fn back_returns_to_the_quick_race_menu() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some((mut h, _, _)) = open() else { return };
    h.press(pad::BACK);
    h.wait(1.5);
    // The screens carry out the switch themselves, so it is the screen on top that shows it happened.
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "{:?}", h.commands);
    assert!(!h.commands.iter().any(|c| matches!(c, Command::StartQuickPlay { .. })), "{:?}", h.commands);
}

#[test]
fn the_footer_names_back_accept_and_randomize() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    use crate::input::{Bindings, InputDevice};
    let Some((h, _, _)) = open() else { return };
    let tree = h.screens.presented_trees(&Bindings::default(), InputDevice::Keyboard).pop().unwrap();
    let words: Vec<&str> = tree.nodes.iter().filter(|n| n.visible).filter_map(|n| n.text.as_deref()).collect();
    let at = |word| words.iter().position(|w| w.trim() == word);
    let (back, accept, randomize) = (at("Back"), at("Accept"), at("Randomize"));
    assert!(back.is_some() && accept.is_some() && randomize.is_some(), "{words:?}");
    assert!(back < accept && accept < randomize, "{words:?}");
}

#[test]
fn the_labels_and_textures_the_brief_names_are_in_the_install() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let label = |hash: u32| Text::Game(hash).resolve(&assets);
    let kinds = [
        RaceType::Sprint,
        RaceType::Circuit,
        RaceType::Drag,
        RaceType::Knockout,
        RaceType::Tollbooth,
        RaceType::SpeedTrap,
    ];
    for kind in kinds {
        assert!(assets.texture(text::mode_icon(kind)).is_some(), "{kind:?} icon");
        assert!(label(text::mode_label(kind)).is_some(), "{kind:?} name");
    }
    for (metric, word) in [(true, "KMs"), (false, "Miles")] {
        assert_eq!(label(text::unit_label(metric)).as_deref(), Some(word));
    }
    let sentence = label(text::RIVAL_LABEL).expect("the rival sentence");
    assert!(sentence.contains("%s") && sentence.contains("%d"), "{sentence}");
    for member in 2..=15 {
        assert!(label(label_hash(&text::alias_key(member))).is_some(), "alias of member {member}");
    }
    for bonus in [0xBD8B_AC91, 0xBD8B_AC92, 0xBD8B_AC93, 0xBD8B_AC94] {
        assert!(label(bonus).is_some(), "bonus sentence {bonus:#x}");
    }
    // The mode icon names the exe hashes stand for.
    assert_eq!(text::mode_icon(RaceType::Circuit), fe_hash_upper("MODE_ICON_CIRCUIT"));
    assert_eq!(text::mode_icon(RaceType::Sprint), fe_hash_upper("MODE_ICON_SPRINT"));
    assert_eq!(text::mode_icon(RaceType::Drag), fe_hash_upper("MODE_ICON_DRAG"));
    assert_eq!(text::mode_icon(RaceType::Knockout), fe_hash_upper("MODE_ICON_LAP_KNOCKOUT"));
}

#[test]
fn the_pool_has_cars_with_folders_logos_and_stats_and_races_with_names() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let (cars, board) = sources(&dir);
    assert!(cars.cars.len() >= 30, "{} cars", cars.cars.len());
    let folders = nfsmw_data::car::list(&dir);
    let mut without_logo = Vec::new();
    let mut without_stats = Vec::new();
    for car in &cars.cars {
        assert!(folders.contains(&car.folder), "{} has no folder {}", car.name, car.folder);
        if car.stats.is_none() {
            without_stats.push(car.name.clone());
        }
        let logos = [text::manufacturer_texture(car), text::model_texture(car)];
        if logos.iter().any(|name| assets.texture(fe_hash_upper(name)).is_none()) {
            without_logo.push(format!("{} ({} {})", car.name, logos[0], logos[1]));
        }
    }
    eprintln!(
        "cars: {:?}",
        cars.cars.iter().map(|c| format!("{}={}/{}", c.name, c.folder, c.rank)).collect::<Vec<_>>()
    );
    eprintln!("cars without stats: {without_stats:?}");
    assert!(without_stats.len() * 4 < cars.cars.len(), "most cars have stats: {without_stats:?}");
    eprintln!("cars without a logo texture: {without_logo:?}");
    assert!(without_logo.len() * 2 < cars.cars.len(), "most cars have both logos: {without_logo:?}");
    let named = board
        .tracks
        .iter()
        .filter(|t| {
            Text::Game(label_hash(&text::track_label_key(&crate::career::event_id(&t.name)))).resolve(&assets).is_some()
        })
        .count();
    let outside: Vec<String> =
        board.tracks.iter().filter(|t| t.bin.is_none()).map(|t| format!("{}:{}", t.name, t.kind)).collect();
    eprintln!("tracks outside the bins: {outside:?}");
    eprintln!("tracks with a name label: {named} of {}", board.tracks.len());
    assert!(board.tracks.iter().all(|t| t.laps >= 1 && t.brief_length() > 0.0), "every track has laps and a length");
    // At the prologue the pool reaches ranks 16, 15 and 14 and bins 15 and 14 only.
    let reach = |stage: u8| cars.cars.iter().filter(|c| pool::car_offered(c.rank, stage)).count();
    assert!(reach(16) > 0 && reach(16) < cars.cars.len());
    assert_eq!(reach(1), cars.cars.len());
}

#[test]
fn a_click_on_each_footer_word_presses_its_action() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    use crate::input::{Action, Bindings, InputDevice};
    let Some((h, _, _)) = open() else { return };
    let (bindings, device) = (Bindings::default(), InputDevice::Keyboard);
    let tree = h.screens.presented_trees(&bindings, device).pop().unwrap();
    for (word, action) in
        [("Back", Action::MenuBack), ("Accept", Action::MenuAccept), ("Randomize", Action::MenuButton1)]
    {
        let shown: Vec<_> =
            tree.nodes.iter().filter(|n| n.visible && n.text.as_deref().is_some_and(|t| t.trim() == word)).collect();
        assert_eq!(shown.len(), 1, "{word} is drawn once: the package's own hint is hidden");
        let point = shown[0].world.transform_point3(glam::Vec3::ZERO).truncate();
        assert_eq!(h.screens.hint_at(point, &bindings, device), Some(action), "{word} at {point:?}");
    }
}
