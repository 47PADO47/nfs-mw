//! With the install (`NFSMW_GAME_DIR`): the race and post-race screens have the objects their logic fills, and the race
//! readout drives them (`hud/race.rs`, `frontend/photo.rs`, `pursuit_results.rs`, `rewards.rs`, `post_race.rs`).

use blackbox_feng::{Runtime, fe_hash_upper};
use game_install::GameDir;

use super::bind::HudBinding;
use crate::ui::Catalog;

/// The group names of the executable's race code are objects of the photo screen `InGamePhotoMaster.fng` and of no other
/// package; the other names are Lua and message names, objects of none (`docs/provenance/race-mode.md` section 10).
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_race_group_names_are_objects_of_the_photo_screen_only() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN", "FRONTEND/FrontB.lzc"]);
    let photo = ["TOLL_BOOTH_GROUP", "SPRINT_GROUP", "SPEEDTRAP_GROUP", "RIVAL_GROUP"];
    let absent = [
        "TollboothStat",
        "SpeedStat",
        "StageStat",
        "RaceResultStat",
        "LapStat",
        "TimerStat",
        "Hud_RaceOverMessage",
        "Hud_TimeExtension",
        "Hud_WrongWIndi",
        "Hud_GenericMessage",
        "Hud_Coundown",
        "Hud_LeaderBoard",
    ];
    let mut checked = 0;
    for entry in catalog.entries() {
        let Some(package) = catalog.find(&entry.name) else { continue };
        checked += 1;
        let has = |name: &str| package.objects.iter().any(|o| o.name_hash == fe_hash_upper(name));
        for name in photo {
            assert_eq!(has(name), entry.name == "InGamePhotoMaster.fng", "{name} in {}", entry.name);
        }
        for name in absent {
            assert!(!has(name), "{name} is an object of {}", entry.name);
        }
    }
    assert!(checked > 100, "{checked} packages");
}

/// The objects the post-race screens fill exist in their packages (`frontend/photo.rs`, `pursuit_results.rs`,
/// `rewards.rs`, `post_race.rs`).
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_post_race_screens_have_the_objects_their_logic_fills() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let wanted: [(&str, &[u32]); 4] = [
        (
            "InGamePhotoMaster.fng",
            &[
                0xE057_94EB,
                0x895E_5289,
                0xC591_84C1,
                0xFBE6_5EB3,
                0x857F_B472,
                0x286A_9CD4,
                0x3D17_73DD,
                0xAB6A_AFDD,
                0x9F4D_F5BB,
                0x37BE_A03B,
                0x8BB3_9726,
                0x424B_B244,
                0x4242_3E94,
                0x8A7F_929C,
                0xFCBE_3F57,
                0x3233_F0A4,
                0x47FF_4E7C,
            ],
        ),
        ("PostRace_Pursuit.fng", &[0xB987_AEA9, 0x8481_22A4, 0x38B1_36A8, 0x85AB_7369, 0x38B1_36AD, 0x85AB_736E]),
        ("PostRace_MilestoneRewards.fng", &[0xE526_D0D2, 0xE104_5A4F, 0x324F_7792, 0x14564FB9]),
        ("PostRace_Results.fng", &[0xEB43_CCB0, 0x38B1_36A8, 0x85AB_7369, 0xD2A5_B02A, 0x4ED8_0BD8, 0x275B_B59A]),
    ];
    for (package, hashes) in wanted {
        let def = catalog.find(package).unwrap_or_else(|| panic!("{package}"));
        for hash in hashes {
            assert!(def.objects.iter().any(|o| o.name_hash == *hash), "{hash:#x} is not an object of {package}");
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_race_readout_binds_the_single_race_hud_objects() {
    use super::race::{Count, RaceHud};
    use super::state::HudState;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let mut binding = HudBinding::new(&mut rt, id);
    // The countdown group sits under the mirror's group, which fades in in its first second.
    binding.advance_mirror_fade(1.0);
    let race = RaceHud { place: 3, racers: 6, lap: Some((2, 4)), clock: 75.5, progress: 0.5, count: Some(Count::Two) };
    let state = HudState { race: Some(race), ..Default::default() };
    for _ in 0..10 {
        binding.apply(&mut rt, &state);
        rt.update(1.0 / 60.0);
    }
    let tree = rt.tree(id);
    let node = |hash: u32| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
    let text = |hash: u32| node(hash).text.clone().unwrap_or_default();
    assert_eq!(text(0x9C18_3BF8), "3", "place");
    assert_eq!(text(0x3BBD_6268), "6", "racers");
    // A lapped race says LAP 2/4 in the progress group (the original's `%d/%d`); the group is not the tollbooth's.
    assert_eq!(text(0x9CB5_C95D), "2/4", "lap");
    assert_eq!(fe_hash_upper("Complete_Text"), 0x328F_1B7C, "the caption of the progress group");
    assert_eq!(text(fe_hash_upper("CurrentLapTime")), "01:15.50", "clock");
    assert_eq!(text(0xB969_E9EF), "2", "countdown step");
    assert!(node(fe_hash_upper("CurrentLapTime")).visible, "clock");
    assert!(node(0xE3DD_CD09).visible, "the progress group");
    assert!(!node(0x1A09_1045).visible, "the tollbooth milestone group is not bound");
    let g = node(fe_hash_upper("321_GO_GROUP"));
    assert!(
        g.visible,
        "countdown group: colour {:?} script {:?}",
        g.colour,
        rt.script_of(rt.find(id, fe_hash_upper("321_GO_GROUP")).unwrap())
    );
    assert_eq!(rt.script_of(rt.find(id, fe_hash_upper("321_GO_GROUP")).unwrap()), Some(fe_hash_upper("TWO")));
    // A sprint says COMPLETE 99 % (the original's `%d %%`, with a space, the percent cut off).
    let sprint = RaceHud { lap: None, progress: 0.997, count: None, ..race };
    binding.apply(&mut rt, &HudState { race: Some(sprint), ..Default::default() });
    let tree = rt.tree(id);
    let progress = tree.nodes.iter().find(|n| n.name_hash == 0x9CB5_C95D).unwrap();
    assert_eq!(progress.text.as_deref(), Some("99 %"), "progress");
    // Out of the race the group goes away again.
    let idle = HudState::default();
    binding.apply(&mut rt, &idle);
    assert!(!rt.tree(id).nodes.iter().find(|n| n.name_hash == fe_hash_upper("CurrentLapTime")).unwrap().visible);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_drag_hud_shows_the_race_readout_and_turns_its_needle() {
    use super::race::{Count, RaceHud};
    use super::state::HudState;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_Drag.fng").unwrap().clone());
    let mut binding = HudBinding::new(&mut rt, id);
    binding.advance_mirror_fade(1.0);
    let race = RaceHud { place: 1, racers: 2, lap: None, clock: 12.25, progress: 0.4, count: Some(Count::Go) };
    let state = HudState { race: Some(race), drag: true, rpm: 5000.0, ..Default::default() };
    for _ in 0..10 {
        binding.apply(&mut rt, &state);
        rt.update(1.0 / 60.0);
    }
    let tree = rt.tree(id);
    let node = |hash: u32| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
    assert_eq!(node(0x9C18_3BF8).text.as_deref(), Some("1"), "place");
    assert_eq!(node(fe_hash_upper("CurrentLapTime")).text.as_deref(), Some("00:12.25"), "clock");
    assert!(node(fe_hash_upper("3rdPersonNeedle")).visible, "the drag needle shows");
    assert!(node(fe_hash_upper("321_GO_GROUP")).visible, "the countdown group shows");
    let angle = node(fe_hash_upper("3rdPersonNeedle")).local_rotation.to_euler(glam::EulerRot::XYZ).2.to_degrees();
    assert!((angle - (42.0 + 0.5 * 228.0)).abs() < 0.1, "the drag needle at 5000 rpm of 10000: {angle}");
    // A shift near the red line plays "Perfect Shift!"; the generic message takes a line.
    let perfect = rt.find(id, 0x2579_C809).unwrap();
    let up = |gear, rpm| HudState { gear, rpm, red_line: 8000.0, drag: true, ..Default::default() };
    binding.apply(&mut rt, &up(1, 7900.0));
    binding.apply(&mut rt, &up(2, 5000.0));
    assert_eq!(rt.script_of(perfect), Some(0xC8D8_8528), "a shift at the red line is perfect");
    let message = super::race::Message { serial: 1, text: "Checkpoint 2/3".into() };
    binding.apply(&mut rt, &HudState { message: Some(message), drag: true, ..Default::default() });
    let tree = rt.tree(id);
    let lines: Vec<_> = tree.nodes.iter().filter(|n| n.name_hash == 0xDA76_8EA6).collect();
    assert_eq!(lines.len(), 2, "the shadow and the face");
    assert!(lines.iter().all(|n| n.text.as_deref() == Some("Checkpoint 2/3")), "the generic message text");
}

/// The real `HUD_SingleRace.fng` shows the leaderboard's rows with their text, highlights the player's backing, and hides
/// the rows past the racers and the online headsets (`hud/leaderboard.rs`).
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_leaderboard_binds_the_single_race_hud_rows() {
    use super::leaders::{Entry, LeaderRow, Leaders};
    use super::race::RaceHud;
    use super::state::HudState;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let mut binding = HudBinding::new(&mut rt, id);
    let entry = |place: u32, name: &str, percent: f32| Entry {
        place,
        player: place == 2,
        row: LeaderRow { name: name.into(), percent, points: 0.0, out: false, busted: false },
    };
    let entries = vec![entry(1, "ROG", 60.0), entry(2, "stock", 50.0), entry(3, "DOM", 40.0)];
    let leaders = Leaders::pick(entries, 2000.0, false);
    let race = RaceHud { place: 2, racers: 3, lap: None, clock: 5.0, progress: 0.5, count: None };
    let state = HudState { race: Some(race), leaders: Some(leaders), use_mph: true, ..Default::default() };
    let mut run = |rt: &mut Runtime, frames: usize| {
        for _ in 0..frames {
            binding.advance_mirror_fade(1.0 / 60.0);
            binding.apply(rt, &state);
            rt.update(1.0 / 60.0);
        }
    };
    run(&mut rt, 20);
    let tree = rt.tree(id);
    let node = |hash: u32| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
    for k in 0..3 {
        assert!(node(0x2AF8_9FD7 + k).visible && node(0x4183_E0A1 + k).visible, "row {k}");
        assert!(node(0x4FDD_D64C + k).visible, "backing {k}");
        assert!(!node(0x51F5_B673 + k).visible, "the online headset of row {k}");
    }
    assert!(!node(0x2AF8_9FDA).visible && !node(0x4FDD_D64F).visible, "no fourth row");
    assert_eq!(node(0x2AF8_9FD7).text.as_deref(), Some("ROG"));
    assert_eq!(node(0x4183_E0A2).text.as_deref(), Some("2"));
    let backing = rt.find(id, 0x4FDD_D64D).unwrap();
    assert_eq!(rt.script_of(backing), Some(0x249D_B7B7), "the player's backing is highlighted");
    // Past the swap the rows say the gaps: 10 percent of 2000 m is 200 m, 656 ft.
    run(&mut rt, 96);
    let tree = rt.tree(id);
    let text = |hash: u32| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().text.clone().unwrap_or_default();
    assert_eq!(text(0x2AF8_9FD7), "+656 FT");
    assert_eq!(text(0x2AF8_9FD8), "-----");
    assert_eq!(text(0x2AF8_9FD9), "-656 FT");
}
