use super::*;

pub(super) fn request_batch(messages: impl IntoIterator<Item = (PackageId, u32)>) {
    let outgoing: Vec<_> = messages
        .into_iter()
        .map(|(package, message)| Outgoing::Sound {
            package,
            message,
            from: None,
            authored: stock_trigger(message).is_some() || message == 0x4ADD_E92C,
        })
        .collect();
    let mut frame = FrameSounds::new(&outgoing);
    for event in outgoing {
        if let Outgoing::Sound { package, message, authored, .. } = event
            && let Some(Some(id)) = frame.candidate(package, message, authored)
        {
            request_trigger(id);
        }
    }
}

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
fn pursuit_results_milestones_and_car_select_route_package_cues() {
    for (message, expected) in [
        (0x8F73_85AB, Some(110)),
        (0xD792_FF31, Some(110)),
        (0x4A80_5994, Some(2)),
        (0x9AFA_53A7, Some(3)),
        (0x9197_0159, Some(21)),
        (0x1CFA_D52B, Some(20)),
        (0xFA27_4177, Some(128)),
        (0xBA01_59B3, Some(129)),
        (0xB647_1CC6, Some(90)),
        (0x4ADD_E92C, None),
        (0xB11F_B113, Some(147)),
        (0x50B3_FA79, Some(135)),
        (0x44D4_3738, Some(145)),
    ] {
        assert_eq!(trigger(message), expected, "{message:08X}");
    }
    take();
    request_batch([(blackbox_feng::PackageId(1), 0x8F73_85AB); 8]);
    assert_eq!(take(), [110; 8]);
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
fn equal_authored_events_survive_even_with_matching_input_feedback() {
    take();
    let package = PackageId(1);
    request_batch([(package, PAD_LEFT), (package, 0x4812_2792), (package, 0x4AC5_E165)]);
    assert_eq!(take(), [2, 2]);
    let events = [Outgoing::Sound { package, message: 0x4812_2792, from: None, authored: true }];
    let mut frame = FrameSounds::new(&events);
    assert_eq!(frame.candidate(package, PAD_LEFT, false), None);
}

#[test]
fn slider_gain_is_consumed_only_by_the_next_common_menu_cue() {
    let gain = blackbox_mixmap::q15_from_db(-1423);
    preview_common_level(0.5);
    assert_eq!(class_gain(90, Group::Sfx), blackbox_mixmap::q15_from_db(-1534));
    assert_eq!(class_gain(4, Group::Hud), 32767);
    assert_eq!(class_gain(4, Group::Sfx), (gain * 16383) >> 15);
    assert_eq!(class_gain(4, Group::Sfx), gain);
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
    let cues = [
        (0, 1),
        (2, 2),
        (3, 0),
        (4, 3),
        (5, 4),
        (6, 0),
        (7, 5),
        (10, 8),
        (11, 9),
        (20, 16),
        (21, 17),
        (28, 0),
        (88, 0),
        (89, 21),
        (90, 22),
        (110, 30),
        (128, 44),
        (129, 45),
        (135, 51),
        (145, 61),
        (147, 63),
    ];
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
    assert_eq!(target(103), ("FE/FE_MB.abk", "PlayFrontEndSample_RS", 80));
}

#[test]
#[ignore = "needs the installed front-end bank"]
fn installed_rap_sheet_cues_replace_only_each_other() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut audio = Audio::replay(dir, 48_000);
    let mut player = MenuPlayer::default();
    player.start(&mut audio, 103);
    player.update(&mut audio, 1.0 / 60.0);
    assert!(player.active.iter().any(|cue| cue.rap_sheet && !cue.module.silent()));
    player.start(&mut audio, 110);
    player.start(&mut audio, 104);
    assert_eq!(player.active.len(), 2);
    let rs = player.active.iter().find(|cue| cue.rap_sheet).unwrap();
    assert_eq!(rs.words[0], 81);
    assert_eq!(target(110).1, "PlayFrontEndSample");
    assert_eq!(target(111).1, "PlayFrontEndSample");
    stop_rap_sheet(103);
    player.service(&mut audio, 0.0);
    assert_eq!(player.active.len(), 2, "a mismatched skip leaves the held cue");
    stop_rap_sheet(104);
    player.service(&mut audio, 0.0);
    assert_eq!(player.active.len(), 1, "a matching skip stops only the held cue");
}

#[test]
#[ignore = "needs the installed common bank"]
fn installed_movie_gate_blocks_menu_cues_and_photo_keeps_them() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut mixer = blackbox_mixmap::Mixer::new(super::super::mixer::mix_map(&dir).unwrap(), &[1]);
    let fehud = blackbox_mixmap::ObjectRef { state: 0, instance: 0, object: 7 };
    mixer.attach(fehud, true);
    mixer.process(1.0 / 60.0);
    assert_eq!(mixer.raw_slot(fehud, 1), Some(class_gain(4, Group::Sfx) as u16));
    assert_eq!(mixer.raw_slot(fehud, 2), Some(class_gain(90, Group::Sfx) as u16));
    let mut audio = Audio::replay(dir, 48_000);
    let mut player = MenuPlayer::default();
    audio.set_presentation_mix(false, true);
    player.start(&mut audio, 4);
    assert!(player.active.is_empty());
    audio.set_presentation_mix(true, false);
    player.start(&mut audio, 4);
    assert_eq!(player.active.len(), 1);
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn installed_menu_trace_correlates_trigger_module_voice_and_silence_without_changing_pcm() {
    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("installed menu banks"),
    ))
    .unwrap();
    let evidence = std::env::var_os("NFSMW_AUDIO_EVIDENCE_DIR").map(std::path::PathBuf::from);
    let path = evidence.as_ref().map_or_else(
        || std::env::temp_dir().join(format!("nfsmw-menu-correlation-{}.jsonl", std::process::id())),
        |dir| dir.join("menu-correlation.jsonl"),
    );
    let mut traced = Audio::replay(dir.clone(), 48000);
    let mut plain = Audio::replay(dir, 48000);
    traced.start_trace(&path).unwrap();
    let mut a = MenuPlayer::default();
    let mut b = MenuPlayer::default();
    take();
    take_hud();
    for id in [4, 5, 6] {
        request_trigger(id);
        a.service(&mut traced, 1.0 / 60.0);
        request_trigger(id);
        b.service(&mut plain, 1.0 / 60.0);
        let mut peak = 0.0_f32;
        for _ in 0..120 {
            a.update(&mut traced, 1.0 / 60.0);
            b.update(&mut plain, 1.0 / 60.0);
            let left = traced.render_replay(800).to_vec();
            let right = plain.render_replay(800).to_vec();
            assert!(left.iter().all(|sample| sample.is_finite()));
            peak = left.iter().fold(peak, |peak, sample| peak.max(sample.abs()));
            assert_eq!(left, right, "tracing must not alter PCM for trigger {id}");
        }
        if id != 6 {
            assert!(peak > 0.0001, "audible trigger {id} must render nonzero PCM");
        }
    }
    request_hud(4);
    a.service(&mut traced, 1.0 / 60.0);
    request_hud(4);
    b.service(&mut plain, 1.0 / 60.0);
    for _ in 0..60 {
        a.update(&mut traced, 1.0 / 60.0);
        b.update(&mut plain, 1.0 / 60.0);
        let left = traced.render_replay(800).to_vec();
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert_eq!(left, plain.render_replay(800));
    }
    a.unavailable.insert(4);
    request_trigger(4);
    a.service(&mut traced, 1.0 / 60.0);
    traced.trace.stop();
    let records: Vec<serde_json::Value> =
        std::fs::read_to_string(&path).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    if evidence.is_none() {
        std::fs::remove_file(path).unwrap();
    }
    let events: Vec<_> = records.iter().filter(|r| r["event"] == "menu_trigger").collect();
    assert_eq!(events.len(), 5);
    let mut instances = Vec::new();
    for (event, id) in events.iter().take(3).zip([4, 5, 6]) {
        assert_eq!(event["trigger"], id);
        assert_eq!(event["group"], "Sfx");
        assert_eq!(event["bank"], "GLOBAL/FE_COMMON_MB.abk");
        assert_eq!(event["class"], "PlayCommonSample");
        assert_eq!(event["params"], serde_json::json!([id, blackbox_mixmap::q15_from_db(-1423), 4096, 0]));
        assert_eq!(event["success"], true);
        let instance = event["instance"].as_u64().filter(|n| *n > 0).unwrap();
        instances.push(instance);
        assert!(records.iter().any(|r| r["event"] == "module_create" && r["instance"] == instance));
        assert!(records.iter().any(|r| r["event"] == "module_tick" && r["instance"] == instance));
        let expected = if id == 6 { "voice_silent" } else { "voice_start" };
        let sequence = |name: &str| {
            records.iter().find(|r| r["event"] == name && r["instance"] == instance).unwrap()["sequence"]
                .as_u64()
                .unwrap()
        };
        assert!(sequence("module_create") < event["sequence"].as_u64().unwrap());
        assert!(event["sequence"].as_u64().unwrap() < sequence(expected));
        if id == 6 {
            assert!(!records.iter().any(|r| r["event"] == "voice_start" && r["instance"] == instance));
        }
    }
    instances.sort_unstable();
    instances.dedup();
    assert_eq!(instances.len(), 3);
    assert_eq!(events[3]["trigger"], 4);
    assert_eq!(events[3]["group"], "Hud");
    let hud_instance = events[3]["instance"].as_u64().filter(|n| *n > 0).unwrap();
    assert!(!instances.contains(&hud_instance));
    let hud_create = records.iter().find(|r| r["event"] == "module_create" && r["instance"] == hud_instance).unwrap();
    let hud_voice = records.iter().find(|r| r["event"] == "voice_start" && r["instance"] == hud_instance).unwrap();
    assert!(hud_create["sequence"].as_u64().unwrap() < events[3]["sequence"].as_u64().unwrap());
    assert!(events[3]["sequence"].as_u64().unwrap() < hud_voice["sequence"].as_u64().unwrap());
    assert_eq!(events[4]["trigger"], 4);
    assert_eq!(events[4]["success"], false);
    assert_eq!(events[4]["instance"], 0);
    assert_eq!(events[4]["detail"], "cached unavailable");
    assert_eq!(records.iter().filter(|r| r["event"] == "module_create").count(), 4);
    assert_eq!(records.last().unwrap()["event"], "session_end");
    assert_eq!(records.last().unwrap()["dropped"], 0);
    assert_eq!(records.last().unwrap()["write_failed"], false);
    assert_eq!(records.last().unwrap()["records"], records.len() - 1);
    for (index, record) in records[..records.len() - 1].iter().enumerate() {
        assert_eq!(record["sequence"], index + 1);
    }
}
