//! The sound messages the post-race and prologue packages send, read from the install: which script or response
//! sends each one, when, and the trigger the menu dispatcher plays for it (`None`: silent here).
use blackbox_feng::package::{Package, ResponseKind};
use game_install::GameDir;

use crate::ui::{Catalog, SCREEN_FILES};

/// The FEng target of the sound system.
const TO_SOUND: u32 = 0xFFFF_FFFB;
const CHECKED_PACKAGES: &[&str] = &[
    "PostRace_Pursuit.fng",
    "PostRace_Results.fng",
    "InGameMilestones.fng",
    "SafeHouseMilestones.fng",
    "Car_Select.fng",
    "Dialog.fng",
    "SMS_Message.fng",
    "SMS_Mailboxes.fng",
    "Pause_Main.fng",
    "SafeHouseReputationOverview.fng",
    "SafeHouseRivalBio.fng",
    "SafeHouseRaceSheet.fng",
    "SafeHouseBounty.fng",
    "InGameDialog.fng",
    "Options.fng",
    "Pause_Options.fng",
    "Paint.fng",
    "CustomizePerformance.fng",
    "CustomizePerformance_BACKROOM.fng",
    "ChallengeSeries.fng",
    "MyCarsManager.fng",
    "MW_LS_Splash.fng",
    "WS_MW_LS_Splash.fng",
    "MC_Main.fng",
    "MC_List.fng",
    "MC_ProfileManager.fng",
];

/// Authored sound posts with no stock generic match or screen substitution.
fn authored_silence(package: &str, message: u32) -> bool {
    matches!(
        (package.to_ascii_lowercase().as_str(), message),
        ("car_select.fng", 0x4ADD_E92C)
            | ("safehouserivalbio.fng", 0x46DD_67DC | 0xE89E_F82F)
            | ("options.fng" | "mycarsmanager.fng" | "mc_profilemanager.fng", 0x480E_A221)
    )
}

/// One message a package sends to the sound system.
#[derive(Debug)]
struct Sent {
    object: u32,
    /// The script that sends it, or the message the response answers.
    via: String,
    message: u32,
}

fn sound_messages(package: &Package) -> Vec<Sent> {
    let mut sent = Vec::new();
    for object in &package.objects {
        for script in &object.scripts {
            for event in script.events.iter().filter(|event| event.target == TO_SOUND) {
                let via = format!("script {:08X} at tick {}", script.id, event.time);
                sent.push(Sent { object: object.name_hash, via, message: event.message });
            }
        }
        for list in &object.responses {
            for response in list.responses.iter().filter(|r| r.kind == ResponseKind::PostToSound) {
                let via = format!("response to {:08X}", list.message);
                sent.push(Sent { object: object.name_hash, via, message: response.param.number() });
            }
        }
    }
    for list in &package.responses {
        for response in list.responses.iter().filter(|r| r.kind == ResponseKind::PostToSound) {
            let via = format!("package response to {:08X}", list.message);
            sent.push(Sent { object: 0, via, message: response.param.number() });
        }
    }
    sent
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); prints the sound messages of the post-race and prologue packages"]
fn installed_post_race_and_prologue_sound_messages() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut files = SCREEN_FILES.to_vec();
    files.push("GLOBAL/INGAMEC.BUN");
    let catalog = Catalog::load(&dir, &files);
    for &name in CHECKED_PACKAGES {
        let package = catalog.find(name).unwrap_or_else(|| panic!("missing sound inventory package {name}"));
        if name == "Car_Select.fng" {
            assert!(sound_messages(package).iter().any(|sent| sent.message == 0x4ADD_E92C));
        }
    }
    let only = std::env::var("NFSMW_INVENTORY_PACKAGES").ok();
    let mut total = 0;
    let mut unmapped = Vec::new();
    for entry in catalog.entries() {
        let Some(package) = catalog.find(&entry.name) else { continue };
        let check = CHECKED_PACKAGES.iter().any(|name| name.eq_ignore_ascii_case(&entry.name));
        let print =
            only.as_deref().is_none_or(|names| names.split(',').any(|name| name.eq_ignore_ascii_case(&entry.name)));
        for sent in sound_messages(package) {
            let mapped = super::stock_trigger(sent.message);
            let silent = authored_silence(&entry.name, sent.message);
            if check && (mapped.is_none() != silent) {
                unmapped.push(format!("{} {sent:?} resolved to {mapped:?}", entry.name));
            }
            if !print {
                continue;
            }
            total += 1;
            eprintln!(
                "SOUND_MESSAGE package={} file={} object={:08X} via=\"{}\" message={:08X} trigger={:?}",
                entry.name, entry.file, sent.object, sent.via, sent.message, mapped
            );
        }
    }
    assert!(total > 0, "no package sends a sound message");
    assert!(unmapped.is_empty(), "unexpected sound coverage:\n{}", unmapped.join("\n"));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); traces screen cues with the offline mixer"]
fn installed_screen_cues_reach_the_authored_banks() {
    use super::super::Audio;
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let path = std::env::temp_dir().join(format!("nfsmw-screen-cues-{}.jsonl", std::process::id()));
    let mut audio = Audio::replay(dir.clone(), 48_000);
    audio.trace.start(&path).unwrap();
    let mut player = super::MenuPlayer::default();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun"]);
    let mut runtime = blackbox_feng::Runtime::new();
    let pursuit = catalog.find("PostRace_Pursuit.fng").expect("pursuit results package");
    let package = runtime.load(pursuit.clone());
    let tally = runtime.find(package, 0x6408_FBF4).expect("pursuit tally object");
    runtime.run_script(tally, 0x2CB4_DBD0);
    runtime.update(0.5);
    let tally_cues = runtime.take_outgoing().into_iter().filter_map(|outgoing| match outgoing {
        blackbox_feng::runtime::Outgoing::Sound { package, message: 0x8F73_85AB, .. } => Some((package, 0x8F73_85AB)),
        _ => None,
    });
    super::take();
    super::tests::request_batch(tally_cues);
    assert_eq!(super::take(), [110; 8], "installed tally script must queue each timed cue");
    let cues = [
        (0x8F73_85AB, 110),
        (0x4A80_5994, 2),
        (0x9197_0159, 21),
        (0xB647_1CC6, 90),
        (0xB11F_B113, 147),
        (0x50B3_FA79, 135),
        (0x44D4_3738, 145),
    ];
    super::tests::request_batch(
        cues.map(|(message, _)| (package, message)).into_iter().chain([(package, 0x4ADD_E92C)]),
    );
    player.service(&mut audio, 1.0 / 60.0);
    for _ in 0..12 {
        audio.render_replay(800);
        audio.trace.advance(1.0 / 60.0);
        player.service(&mut audio, 1.0 / 60.0);
    }
    audio.trace.stop();
    let trace = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let records: Vec<serde_json::Value> = trace.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    let requests: Vec<_> = records.iter().filter(|record| record["event"] == "menu_trigger").collect();
    assert_eq!(requests.len(), cues.len());
    for (record, (message, id)) in requests.into_iter().zip(cues) {
        let (bank, class, slot) = super::target(id);
        assert_eq!(record["trigger"], id, "{message:08X}");
        assert_eq!(record["bank"], bank);
        assert_eq!(record["class"], class);
        assert_eq!(record["params"][0], slot);
        assert_eq!(record["success"], true);
        assert!(
            records.iter().any(|voice| voice["event"] == "voice_start" && voice["instance"] == record["instance"]),
            "no voice for {message:08X}: {record}"
        );
        eprintln!("SCREEN_CUE {message:08X} {record}");
    }
}

#[test]
#[ignore = "needs the installed purchase dialog and common bank"]
fn installed_purchase_dialog_button_posts_the_common_accept_cue() {
    use blackbox_feng::ids::BUTTON_PRESSED;
    use blackbox_feng::runtime::Outgoing;
    use blackbox_feng::{ObjectRef, Runtime};

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let dialog = catalog.find("Dialog.fng").expect("purchase dialog");
    let mut runtime = Runtime::new();
    let package = runtime.load(dialog.clone());
    let index = dialog.objects.iter().position(|object| object.name_hash == 0xB8A7_C6CC).expect("dialog answer");
    runtime.post(package, BUTTON_PRESSED, Some(ObjectRef { package, index }));
    runtime.update(1.0 / 60.0);
    let accept: Vec<_> = runtime
        .take_outgoing()
        .into_iter()
        .filter_map(|out| match out {
            Outgoing::Sound { message: 0x7B6B_89D7, authored: true, .. } => super::stock_trigger(0x7B6B_89D7),
            _ => None,
        })
        .collect();
    assert_eq!(accept, [4]);
}

/// The photo's two cues, played headless with the audio trace on: the voices the shutter graph starts (sample, rate,
/// gain), the recordings of its bank and of the `game_camera` moment (sample rate, channels, length).
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); prints what the photo's cues play"]
fn installed_photo_cues_report_their_voices() {
    use super::super::Audio;
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let path = std::env::temp_dir().join(format!("nfsmw-photo-cues-{}.jsonl", std::process::id()));
    let mut audio = Audio::replay(dir.clone(), 48_000);
    audio.trace.start(&path).unwrap();
    let mut player = super::MenuPlayer::default();
    super::photo::take();
    super::request_photo_finish();
    player.service(&mut audio, 0.0);
    for _ in 0..360 {
        player.update(&mut audio, 1.0 / 60.0);
        audio.trace.advance(1.0 / 60.0);
        audio.render_replay(800);
    }
    audio.trace.stop();
    let trace = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    for line in trace.lines().filter(|line| !line.contains("\"module_tick\"") && !line.contains("\"voice_update\"")) {
        eprintln!("PHOTO_TRACE {line}");
    }
    let bank = "IG_GLOBAL/FX_Camera.abk";
    for index in 1..=audio.bank_len(bank).unwrap() {
        let Ok(data) = audio.bank_sound(bank, index) else { continue };
        let channels = audio.sound_channels.get(&(bank.to_owned(), index)).copied();
        eprintln!(
            "PHOTO_SAMPLE bank={bank} index={index} rate={} frames={} secs={:.3} channels={channels:?} looping={}",
            data.sample_rate,
            data.frames.len(),
            data.frames.len() as f64 / f64::from(data.sample_rate),
            data.settings.loop_region.is_some()
        );
    }
    let db = audio.database().unwrap();
    let key = blackbox_attrib::vlt_hash("game_camera");
    let moment = nfsmw_data::sound::MomentSound::by_key(&db, key).expect("game_camera moment");
    let index = dir.read("SOUND/STREAMS/NISAudio.idx").unwrap();
    let takes = nfsmw_data::sound::moment_takes(&index, moment).unwrap();
    eprintln!("PHOTO_MOMENT game_camera={moment:?} takes={takes:?}");
    let parsed = ea_audio::speech::SpeechIndex::parse(&index).unwrap();
    let file =
        super::super::radio::feeder::FileSource::open(dir.resolve("SOUND/STREAMS/NISAudio.big").unwrap()).unwrap();
    for (bank, take) in takes {
        let pcm = parsed.banks()[bank].decode(&file, take).unwrap();
        eprintln!(
            "PHOTO_MOMENT_TAKE bank={bank} take={take} rate={} channels={} secs={:.3}",
            pcm.sample_rate,
            pcm.channels,
            pcm.duration_secs()
        );
    }
}
