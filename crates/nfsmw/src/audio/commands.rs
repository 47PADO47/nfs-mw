//! Console commands that deal with sound.

use super::{Audio, EngineMix, Group};

/// Whether `name` is one of the commands of this module.
pub fn handles(name: &str) -> bool {
    matches!(name, "sound" | "engine" | "radio")
}

/// Run a sound command. `audio` is `None` when the game was started without sound.
pub fn run(audio: Option<&mut Audio>, name: &str, args: &[&str]) -> Result<String, String> {
    let audio = audio.ok_or("sound is off (started with --no-sound or taking a screenshot)")?;
    match name {
        "sound" => sound(audio, args),
        "engine" => engine(audio, args),
        "radio" => super::radio::command(audio, args),
        other => Err(format!("unknown sound command {other:?}")),
    }
}

fn sound(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    let status = if audio.available() { "" } else { " (no sound device)" };
    match args {
        [] => Ok(format!(
            "sound{status}: sound <bank> lists a bank, sound <bank> <index> plays a sound. Banks are paths under SOUND/, e.g. IG_GLOBAL/Siren_MB.abk or SHIFTING/GEAR_MED_Lev3.abk"
        )),
        [bank] => {
            let count = audio.bank_len(bank)?;
            Ok(format!("{bank}: {count} sounds{status}"))
        }
        [bank, index] => {
            let index: usize = index.parse().map_err(|_| format!("{index:?} is not a sound number"))?;
            let data = audio.bank_sound(bank, index)?;
            let seconds = data.duration().as_secs_f32();
            audio.play(Group::Sfx, data)?;
            Ok(format!("playing {bank} #{index} ({seconds:.2} s){status}"))
        }
        _ => Err("usage: sound [bank [index]]".into()),
    }
}

/// `engine <car> [percent]` holds a car's engine loops at a fraction of its RPM range; `engine off` stops it.
fn engine(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    match args {
        ["off"] => Ok(match audio.test_engine.take() {
            Some((_, sound)) => format!("engine {} stopped", sound.engine.name),
            None => "no engine is playing".to_owned(),
        }),
        [car, rest @ ..] if rest.len() <= 1 => {
            let percent: f32 = match rest {
                [p] => p.trim_end_matches('%').parse().map_err(|_| format!("{p:?} is not a percentage"))?,
                _ => 50.0,
            };
            let engine = audio.load_car_engine(car)?;
            let sound_rpm = 1000.0 + 9000.0 * engine.sound.engine.remap_rpm(percent / 100.0);
            let frequency = engine.sound.engine.ginsu_frequency(sound_rpm);
            let mix = EngineMix::shared(frequency, 1.0, 1.0, 0.0);
            let name = engine.sound.engine.name.clone();
            let handle = audio.start_engine(super::EngineVoice { start: mix, ..engine.voice })?;
            handle.set(mix);
            audio.test_engine = Some((handle, engine.sound));
            Ok(format!("engine {name} at {percent:.0}% of its range ({:.0}); engine off stops it", frequency))
        }
        _ => Err("usage: engine <car> [percent] | engine off".into()),
    }
}
