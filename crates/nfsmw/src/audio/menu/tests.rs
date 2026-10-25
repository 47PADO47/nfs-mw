use super::*;

#[test]
fn menu_messages_choose_authored_triggers_and_ignore_releases() {
    for (message, id) in [
        (PAD_UP, 0),
        (PAD_DOWN, 0),
        (PAD_LEFT, 2),
        (PAD_RIGHT, 2),
        (PAD_ACCEPT, 4),
        (BUTTON_PRESSED, 4),
        (PAD_BACK, 5),
        (PAD_START, 6),
        (0x6BC1_CD5D, 10),
        (0xE3FB_93BE, 11),
        (0x145B_13C0, 88),
    ] {
        assert_eq!(trigger(message), Some(id));
    }
    for message in [PAD_ACCEPT_RELEASED, PAD_BACK_RELEASED, BUTTON_RELEASED, 0] {
        assert_eq!(trigger(message), None);
    }
    assert_eq!(target(4), ("GLOBAL/FE_COMMON_MB.abk", "PlayCommonSample", 4));
    assert_eq!(target(88), ("FE/FE_MB.abk", "PlayFrontEndSample", 65));
}

#[test]
fn menu_requests_are_drained_once() {
    take();
    request(PAD_ACCEPT);
    request(PAD_ACCEPT_RELEASED);
    request(PAD_BACK);
    assert_eq!(take(), [4, 5]);
    assert!(take().is_empty());
}

#[test]
fn hud_cues_queue_apart_from_the_menu_sounds() {
    take();
    take_hud();
    request_hud(10);
    request(PAD_BACK);
    request_hud(4);
    assert_eq!(take(), [5], "a HUD cue is not a menu sound");
    assert_eq!(take_hud(), [10, 4], "the HUD's cues keep their order");
    assert!(take_hud().is_empty());
}

#[test]
fn one_package_accepts_once_even_with_pad_button_and_script_announcements() {
    take();
    let package = blackbox_feng::PackageId(1);
    request_batch([(package, PAD_ACCEPT), (package, BUTTON_PRESSED), (package, 0x7B6B_89D7)]);
    assert_eq!(take(), [4]);
}

#[test]
fn only_the_six_common_auxiliary_buttons_have_a_semantic_mapping() {
    for button in PAD_BUTTON0..=PAD_BUTTON0 + 5 {
        assert_eq!(trigger(button), Some(7));
    }
    for button in PAD_BUTTON0 + 6..=PAD_BUTTON0 + 9 {
        assert_eq!(trigger(button), None);
    }
    for released in PAD_BUTTON_RELEASED {
        assert_eq!(trigger(released), None);
    }
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn installed_menu_effects_produce_audio_and_finish() {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("menu audio tests require NFSMW_GAME_DIR"),
    ))
    .unwrap();
    let mut failures = Vec::new();
    // Zero is an authored empty sample-group entry, not a missing decoder implementation.
    let cues = [(0, 1), (2, 2), (4, 3), (5, 4), (6, 0), (7, 5), (10, 8), (11, 9), (28, 0), (88, 0), (89, 21)];
    for fps in [30usize, 60, 144] {
        for (id, expected_sample) in cues {
            let mut audio = Audio::replay(dir.clone(), 48000);
            let mut player = MenuPlayer::default();
            player.start(&mut audio, id);
            if player.active.len() != 1 {
                failures.push(format!("trigger {id} at {fps} FPS failed to load"));
                continue;
            }
            let (bank, class, slot) = target(id);
            let parsed = audio.module_banks[bank].clone();
            let bytes = audio.bank_bytes(bank).unwrap();
            let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let module = &parsed.modules()[parsed.module_of(class).unwrap()];
            for &offset in module.node_offsets.iter().take(usize::from(module.players)) {
                let group = word(module.data + offset + 4);
                let sample = word(group + 4 + 12 * slot as usize + 4);
                if sample != expected_sample {
                    failures.push(format!("trigger {id}: authored slot changed: {sample}, expected {expected_sample}"));
                }
            }
            player.start(&mut audio, id);
            if !std::sync::Arc::ptr_eq(&parsed, &audio.module_banks[bank]) {
                failures.push(format!("trigger {id}: bank reparsed for repeated event"));
            }
            // The second instance proves cache reuse; render one gesture's independent lifetime.
            player.active.pop();
            let mut peak = 0.0f32;
            let mut finite = true;
            for frame in 0..fps * 9 {
                player.update(&mut audio, 1.0 / fps as f32);
                let frames = (frame + 1) * 48000 / fps - frame * 48000 / fps;
                for sample in audio.render_replay(frames) {
                    finite &= sample.is_finite();
                    peak = peak.max(sample.abs());
                }
            }
            let audible = peak > 0.0001;
            let finished = player.active.is_empty();
            println!(
                "trigger={id} fps={fps} sample={expected_sample} peak={peak:.8} finite={finite} finished={finished}"
            );
            if !finite || audible != (expected_sample != 0) || !finished {
                failures.push(format!("trigger {id} at {fps} FPS: sample={expected_sample}, peak={peak}, finite={finite}, finished={finished}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_trigger_can_be_requested_by_id() {
    take();
    request_trigger(103);
    assert_eq!(take(), [103]);
    assert_eq!(target(103), ("FE/FE_MB.abk", "PlayFrontEndSample", 80));
}
