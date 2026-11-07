//! Installed photo appearance and input timing, including the retained standings backdrop.
use super::*;

fn picture(harness: &Harness) -> (bool, u8) {
    let tree = harness.screens.trees().pop().expect("photo tree");
    let node = tree.nodes.iter().find(|node| node.name_hash == 0x286A_9CD4).expect("photo picture");
    (node.visible, node.world_colour[3])
}

fn hint_alpha(harness: &Harness) -> u8 {
    let mut tree = harness.screens.trees().pop().unwrap();
    let frame = crate::frontend::prompt_layout::Frame::find(
        &tree,
        screen::PHOTO,
        crate::frontend::logic::Footer::RacePhoto { race: true },
        harness.screens.assets(),
    )
    .unwrap();
    let first = tree.nodes.len();
    tree.nodes.push(blackbox_feng::UiNode { world_colour: [255; 4], z: 1.0, ..Default::default() });
    frame.apply(&mut tree, first);
    tree.nodes[first].world_colour[3]
}

#[test]
#[ignore = "needs installed FEng packages; checks actual alpha and semantic input timing"]
fn installed_race_presentation_photo_waits_then_animates_and_enables_buttons() {
    let mut harness = Harness::open(screen::PHOTO, Args::default()).expect("NFSMW_GAME_DIR and photo package");
    harness.wait(0.5);
    assert_eq!(hint_alpha(&harness), 0, "replacement prompts must wait for the native footer entrance");
    assert_eq!(picture(&harness).1, 0, "the driving/cinematic interval has no photo backdrop");
    harness.press(pad::BUTTON0 << 4);
    assert!(!harness.commands.iter().any(|c| matches!(c, Command::ShowResults(_))), "entrance must gate shortcuts");
    harness.wait(1.3);
    let (visible, alpha) = picture(&harness);
    eprintln!("PHOTO_APPEAR mid visible={visible} alpha={alpha}");
    assert_eq!((visible, alpha), (true, 255), "the frame appears after the camera settles");
    let tree = harness.screens.trees().pop().unwrap();
    let text = tree.nodes.iter().find(|node| node.name_hash == 0xAB6A_AFDD).expect("time text");
    assert!(text.world_colour[3] < 255, "result text has its own delayed entrance: {:?}", text.world_colour);
    harness.wait(2.0);
    assert_eq!(hint_alpha(&harness), 255, "replacement prompts reach full opacity with the footer");
    assert_eq!(picture(&harness), (true, 255), "the entrance must reach the held photo");
    let tree = harness.screens.trees().pop().unwrap();
    for hash in [0xAB6A_AFDD, 0x37AC_A28C] {
        let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
        assert!(node.visible && node.world_colour[3] == 255, "{hash:x}: held state {:?}", node.world_colour);
    }
    harness.press(pad::BUTTON0 << 4);
    assert!(harness.commands.iter().any(|c| matches!(c, Command::ShowResults(_))), "stats must work after entrance");
}

#[test]
#[ignore = "needs installed FEng packages; verifies the photo remains under standings"]
fn installed_race_presentation_standings_preserve_the_finished_photo() {
    let mut harness = Harness::open(screen::PHOTO, Args::default()).expect("NFSMW_GAME_DIR and photo package");
    harness.wait(4.0);
    let mut env =
        Env { settings: &mut harness.settings, changed: &mut harness.changed, caps: test_caps::native(), mods: None };
    harness.screens.open_with(
        screen::POST_RACE_RESULTS,
        Box::new(crate::frontend::post_race::PostRaceLogic::new(crate::viewer::RaceReport::sample())),
        false,
        &mut env,
    );
    assert_eq!(harness.screens.previous(), Some(screen::PHOTO));
    for device in [crate::input::InputDevice::Keyboard, crate::input::InputDevice::Xbox] {
        let shown = harness.screens.presented_trees(&crate::input::Bindings::default(), device);
        assert_eq!(shown.len(), 2);
        let photo = &shown[0];
        assert!(
            photo.nodes.iter().any(|node| node.name_hash == 0x286A_9CD4 && node.visible && node.world_colour[3] > 0)
        );
        assert!(
            photo.nodes.iter().filter(|node| node.name_hash == 0x37AC_A28C).all(|node| !node.visible),
            "covered photo footer must not overlap standings"
        );
        let raw_len = harness.screens.trees()[0].nodes.len();
        assert_eq!(photo.nodes.len(), raw_len, "covered photo adds no replacement buttons");
        assert!(
            photo.nodes.iter().filter(|node| node.name_hash == 0xAB6A_AFDD).all(|node| !node.visible),
            "photo result text must not show through standings rows"
        );
    }
    harness.screens.pop(&mut env);
    assert_eq!(harness.screens.top(), Some(screen::PHOTO));
    assert_eq!(picture(&harness), (true, 255), "back does not replay or erase the photo entrance");
    let restored =
        harness.screens.presented_trees(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
    assert!(
        restored[0].nodes.iter().any(|node| node.name_hash == 0xAB6A_AFDD && node.visible),
        "Back restores the photo's result text"
    );
}

#[test]
#[ignore = "needs installed photo UI, moment stream and camera bank; tests gameplay request delivery"]
fn installed_race_presentation_photo_delivers_transition_and_shutter_audio() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut audio = crate::audio::Audio::replay(dir, 48000);
    let mut harness = Harness::open(screen::PHOTO, Args::default()).unwrap();
    let path = std::env::temp_dir().join(format!("photo-delivery-{}.jsonl", std::process::id()));
    audio.start_trace(&path).unwrap();
    let mut peaks = [0.0_f32; 2];
    for frame in 0..300 {
        harness.run(0, 1);
        audio.update_menu(FRAME);
        for &sample in audio.render_replay(800) {
            let window = usize::from(frame >= 90);
            peaks[window] = peaks[window].max(sample.abs());
        }
    }
    drop(audio);
    let trace = std::fs::read_to_string(&path).unwrap();
    assert_eq!(trace.matches("\"event\":\"photo_transition_trigger\"").count(), 1);
    assert_eq!(trace.matches("\"event\":\"photo_finish_trigger\"").count(), 1);
    assert!(trace.contains("\"class\":\"IMPACT_SWEETENER\""), "transition recording must reach output");
    assert!(trace.contains("\"event\":\"voice_start\""), "shutter module must start a voice");
    assert!(peaks.iter().all(|peak| *peak > 0.001), "both timing windows must produce PCM: {peaks:?}");
    std::fs::remove_file(path).unwrap();
}
