//! `bb.audio`: a one-shot sound by its bank and its index in that bank. The sound is queued for the audio plugin
//! (`audio::oneshot`), which plays it once on the sound-effects group. The game has no lookup from an event name to a sound
//! yet, so the sound is named by its index: `event` is the index as text (`"3"`), or `bank` is `"FE_MB.abk:3"` with `event`
//! empty. A name that is not an index, a bank name with a dot that is not `.abk`, or a volume that is not a number of zero or more
//! answers `BB_ERR_ARG`. A sound the install does not have is logged by the plugin, not returned.

use blackbox_modding::abi::BB_ERR_ARG;
use blackbox_modding::host::{AudioBackend, BackendResult};

use crate::audio::oneshot;

/// The `bb.audio` port of the game. It has no state: the queue is in `audio::oneshot`.
pub struct AudioPort;

/// A sound as the mods name it: a bank and the index in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundRef {
    pub bank: String,
    pub index: usize,
}

impl AudioBackend for AudioPort {
    fn play(&self, _owner: &str, bank: &str, event: &str, volume: f32) -> BackendResult<u64> {
        let sound = parse_sound(bank, event).ok_or(BB_ERR_ARG)?;
        if !volume.is_finite() || volume < 0.0 {
            return Err(BB_ERR_ARG);
        }
        Ok(oneshot::request(sound.bank, sound.index, volume))
    }
}

/// Reads the sound a `play` names. With `event` empty, `bank` may carry the index after its last `:`.
pub fn parse_sound(bank: &str, event: &str) -> Option<SoundRef> {
    let (bank, index) = split_ref(bank, event);
    let bank = bank_file(bank.trim())?;
    let index = index.trim().parse().ok()?;
    Some(SoundRef { bank, index })
}

/// The bank and the index as given, or, with no index in `event`, the bank and index that `bank` joins with a `:`.
fn split_ref<'a>(bank: &'a str, event: &'a str) -> (&'a str, &'a str) {
    if !event.trim().is_empty() {
        return (bank, event);
    }
    bank.rsplit_once(':').unwrap_or((bank, event))
}

/// The bank file a name gives: a name with no dot is the bank of that name (`IG_GLOBAL` is `IG_GLOBAL.abk`), a name with
/// a dot must end in `.abk`. A name that goes up out of the install is refused. The folders of the path are kept.
fn bank_file(bank: &str) -> Option<String> {
    if bank.is_empty() || bank.contains("..") || bank.contains('\\') {
        return None;
    }
    if !bank.contains('.') {
        return Some(format!("{bank}.abk"));
    }
    bank.to_ascii_lowercase().ends_with(".abk").then(|| bank.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(bank: &str, index: usize) -> Option<SoundRef> {
        Some(SoundRef { bank: bank.to_owned(), index })
    }

    #[test]
    fn a_bank_and_an_index_name_a_sound() {
        assert_eq!(parse_sound("FE/FE_MB.abk", "3"), sound("FE/FE_MB.abk", 3));
        assert_eq!(parse_sound(" FE_MB.abk ", " 0 "), sound("FE_MB.abk", 0));
        assert_eq!(parse_sound("IG_GLOBAL", "3"), sound("IG_GLOBAL.abk", 3), "a bank name without .abk is that bank");
    }

    #[test]
    fn a_bank_with_its_index_after_a_colon_names_a_sound() {
        assert_eq!(parse_sound("FE/FE_MB.abk:12", ""), sound("FE/FE_MB.abk", 12));
        assert_eq!(parse_sound("FE_MB.abk:x", ""), None, "the index must be a number");
    }

    #[test]
    fn an_event_name_or_a_bad_bank_names_no_sound() {
        assert_eq!(parse_sound("FE_MB.abk", "gear_shift"), None, "no lookup by event name yet");
        assert_eq!(parse_sound("FE_MB.abk", ""), None, "no index");
        assert_eq!(parse_sound("FE_MB.txt", "1"), None, "not a bank");
        assert_eq!(parse_sound("../FE_MB.abk", "1"), None, "no way up out of the install");
        assert_eq!(parse_sound("", "1"), None);
    }

    #[test]
    fn a_bad_volume_is_refused_and_a_good_one_is_queued() {
        assert_eq!(AudioPort.play("mod", "FE_MB.abk", "1", f32::NAN).err(), Some(BB_ERR_ARG));
        assert_eq!(AudioPort.play("mod", "FE_MB.abk", "1", -0.5).err(), Some(BB_ERR_ARG));
        assert_eq!(AudioPort.play("mod", "FE_MB.abk", "gear", 1.0).err(), Some(BB_ERR_ARG));
        let first = AudioPort.play("mod", "FE_MB.abk", "1", 0.5).expect("a sound is queued");
        let second = AudioPort.play("mod", "FE_MB.abk:2", "", 1.0).expect("a sound is queued");
        assert_eq!(second, first + 1);
    }
}
