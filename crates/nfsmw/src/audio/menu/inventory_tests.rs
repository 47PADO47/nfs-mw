//! The sound messages the post-race and prologue packages send, read from the install: which script or response
//! sends each one, when, and the trigger the menu dispatcher plays for it (`None`: silent here).
use blackbox_feng::package::{Package, ResponseKind};
use game_install::GameDir;

use super::trigger;
use crate::ui::{Catalog, SCREEN_FILES};

/// The FEng target of the sound system.
const TO_SOUND: u32 = 0xFFFF_FFFB;

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
                let via = format!("script {:08X} at {} ms", script.id, event.time);
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
    let only = std::env::var("NFSMW_INVENTORY_PACKAGES").ok();
    let mut total = 0;
    for entry in catalog.entries() {
        if only.as_deref().is_some_and(|names| !names.split(',').any(|name| name.eq_ignore_ascii_case(&entry.name))) {
            continue;
        }
        let Some(package) = catalog.find(&entry.name) else { continue };
        for sent in sound_messages(package) {
            total += 1;
            eprintln!(
                "SOUND_MESSAGE package={} file={} object={:08X} via=\"{}\" message={:08X} trigger={:?}",
                entry.name,
                entry.file,
                sent.object,
                sent.via,
                sent.message,
                trigger(sent.message)
            );
        }
    }
    assert!(total > 0, "no package sends a sound message");
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
