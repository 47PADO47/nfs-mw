//! Turning the shared recordings into WAV files under `SOUND/MODSHARED/` and the lines of the `[car_sound.shared]` table
//! that name them.

use crate::sample::{Sample, wav_bytes};

/// The folder of the shared recordings, in the install's layout.
pub(super) const FOLDER: &str = "SOUND/MODSHARED";
/// The peak the loudest recording of a group is brought to; the others keep their level relative to it.
const PEAK: f32 = 0.9;
/// A recording is not boosted more than this.
const MAX_BOOST: f32 = 8.0;
/// Most files in a list (what the car manifest allows).
pub(super) const MAX_FILES: usize = 16;

/// A recording ready to write.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Shot {
    /// The path in the install's layout (`SOUND/MODSHARED/IMPACT/1000000000B392A.WAV`).
    pub path: String,
    pub wav: Vec<u8>,
}

/// The path of the recording of resource `id` in the group `kind` (`IMPACT`, `SCRAPE`, `POP`).
pub(super) fn path_of(kind: &str, id: u64) -> String {
    format!("{FOLDER}/{kind}/{id:X}.WAV")
}

/// The gain that brings the loudest of `samples` to [`PEAK`], at most [`MAX_BOOST`] times; 1 for silence.
fn group_gain<'a>(samples: impl Iterator<Item = &'a Sample>) -> f32 {
    let loudest = samples.map(Sample::peak).fold(0.0f32, f32::max);
    if loudest > 1e-4 {
        return (PEAK / loudest).min(MAX_BOOST);
    }
    1.0
}

/// WAV files of `samples` (resource id, decoded), all in proportion: the loudest has the peak.
pub(super) fn shots(kind: &str, samples: &[(u64, Sample)]) -> Vec<Shot> {
    let gain = group_gain(samples.iter().map(|(_, sample)| sample));
    samples
        .iter()
        .map(|(id, sample)| Shot { path: path_of(kind, *id), wav: wav_bytes(&sample.samples, sample.rate, gain) })
        .collect()
}

/// One recording alone, brought to the peak (a pop is picked for a car by its graph, so it stands on its own).
pub(super) fn lone_shot(kind: &str, id: u64, sample: &Sample) -> Shot {
    let gain = group_gain(std::iter::once(sample));
    Shot { path: path_of(kind, id), wav: wav_bytes(&sample.samples, sample.rate, gain) }
}

/// The `key = ["path", ...]` line of a list, empty for a list with no file.
pub(super) fn list_line(key: &str, paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let quoted: Vec<String> = paths.iter().take(MAX_FILES).map(|path| format!("\"{path}\"")).collect();
    format!("{key} = [{}]\n", quoted.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(peak: f32) -> Sample {
        Sample { id: 1, samples: vec![0.0, peak, -peak], rate: 48_000, looped: false }
    }

    fn peak_of(wav: &[u8]) -> i16 {
        wav[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes([b[0], b[1]]).abs()).max().unwrap()
    }

    #[test]
    fn a_group_keeps_its_proportions_and_its_loudest_has_the_peak() {
        let made = shots("IMPACT", &[(0xA, sample(1.8)), (0xB, sample(0.9))]);
        assert_eq!(made[0].path, "SOUND/MODSHARED/IMPACT/A.WAV");
        let (loud, quiet) = (f32::from(peak_of(&made[0].wav)), f32::from(peak_of(&made[1].wav)));
        assert!((loud / 32767.0 - PEAK).abs() < 0.001, "{loud}");
        assert!((quiet / loud - 0.5).abs() < 0.01, "{quiet} {loud}");
    }

    #[test]
    fn a_silent_recording_is_not_boosted_and_a_quiet_one_at_most_eight_times() {
        let silent = shots("POP", &[(1, sample(0.0))]);
        assert_eq!(peak_of(&silent[0].wav), 0);
        let quiet = lone_shot("POP", 2, &sample(0.05));
        assert!(f32::from(peak_of(&quiet.wav)) / 32767.0 <= 0.05 * MAX_BOOST + 0.001);
    }

    #[test]
    fn a_list_line_names_at_most_sixteen_files_and_an_empty_list_has_no_line() {
        assert_eq!(list_line("scrape", &[]), "");
        assert_eq!(list_line("scrape", &["A.WAV".into(), "B.WAV".into()]), "scrape = [\"A.WAV\", \"B.WAV\"]\n");
        let many: Vec<String> = (0..20).map(|n| format!("F{n}.WAV")).collect();
        assert_eq!(list_line("pops", &many).matches('"').count(), 2 * MAX_FILES);
    }
}
