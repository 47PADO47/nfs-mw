//! Console commands that deal with sound.

use super::{Audio, Group};

/// Whether `name` is one of the commands of this module.
pub fn handles(name: &str) -> bool {
    matches!(name, "sound")
}

/// Run a sound command. `audio` is `None` when the game was started without sound.
pub fn run(audio: Option<&mut Audio>, name: &str, args: &[&str]) -> Result<String, String> {
    let audio = audio.ok_or("sound is off (started with --no-sound or taking a screenshot)")?;
    match name {
        "sound" => sound(audio, args),
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
