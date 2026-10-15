//! Every sound of every bank in the install, in the order the sidebar lists them: the banks by folder, then each
//! bank's sounds by index. A bank is read only to count its sounds, and then dropped.

use game_install::GameDir;

/// The folders under `SOUND/` that hold `.abk` sound banks (`docs/formats/audio.md`, folder map). `EVT_SYS`,
/// `MIXMAPS`, `NISREVDATA`, `PFDATA`, `SPEECH`, `STREAMS` and `FXEDIT` hold no banks of this kind.
pub const BANK_FOLDERS: [&str; 8] = ["ENGINE", "FE", "GLOBAL", "IG_GLOBAL", "NOS", "SHIFTING", "SKIDS", "TURBO"];

/// One sound: the bank it is in (a path under `SOUND/`, such as `FE/FE_MB.abk`) and its index in that bank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub bank: String,
    pub index: usize,
}

impl Entry {
    /// The name the sidebar shows and the search matches: `FE/FE_MB.abk #3`.
    pub fn name(&self) -> String {
        format!("{} #{}", self.bank, self.index)
    }
}

/// What the listing found: every sound, and the banks that could not be read (their names and why).
#[derive(Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub unreadable: Vec<String>,
}

/// Lists every sound of every bank in the install.
pub fn list(dir: &GameDir) -> Listing {
    let mut listing = Listing::default();
    for folder in BANK_FOLDERS {
        for file in dir.files_in(&format!("SOUND/{folder}")) {
            if !file.to_ascii_lowercase().ends_with(".abk") {
                continue;
            }
            let bank = format!("{folder}/{file}");
            match count(dir, &bank) {
                Ok(count) => listing.entries.extend((0..count).map(|index| Entry { bank: bank.clone(), index })),
                Err(_) => listing.unreadable.push(bank),
            }
        }
    }
    listing
}

/// How many sounds the bank `bank` (a path under `SOUND/`) holds.
fn count(dir: &GameDir, bank: &str) -> Result<usize, String> {
    let bytes = dir.read(&format!("SOUND/{bank}")).map_err(|e| format!("{e:#}"))?;
    let parsed = ea_audio::abk::Bank::parse(&bytes).map_err(|e| e.to_string())?;
    Ok(parsed.sounds().len())
}

/// The sidebar's names, one per entry, in the same order.
pub fn names(entries: &[Entry]) -> Vec<String> {
    entries.iter().map(Entry::name).collect()
}

/// The entry to start on: the first sound of the bank `wanted` names (in any case, with or without its folder), or
/// the first entry when `wanted` is absent. `None` when no bank matches.
pub fn start(entries: &[Entry], wanted: Option<&str>) -> Option<usize> {
    let Some(wanted) = wanted else { return (!entries.is_empty()).then_some(0) };
    entries.iter().position(|entry| {
        let bank = entry.bank.to_ascii_lowercase();
        let wanted = wanted.to_ascii_lowercase();
        bank == wanted || bank.rsplit('/').next() == Some(wanted.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(bank: &str, index: usize) -> Entry {
        Entry { bank: bank.into(), index }
    }

    #[test]
    fn a_name_is_the_bank_and_the_index() {
        assert_eq!(entry("FE/FE_MB.abk", 3).name(), "FE/FE_MB.abk #3");
    }

    #[test]
    fn start_finds_a_bank_with_or_without_its_folder_in_any_case() {
        let entries = [entry("FE/FE_MB.abk", 0), entry("FE/FE_MB.abk", 1), entry("GLOBAL/FE_COMMON_MB.abk", 0)];
        assert_eq!(start(&entries, None), Some(0));
        assert_eq!(start(&entries, Some("fe_common_mb.abk")), Some(2));
        assert_eq!(start(&entries, Some("GLOBAL/fe_common_mb.abk")), Some(2));
        assert_eq!(start(&entries, Some("nothing.abk")), None);
        assert_eq!(start(&[], None), None);
    }
}
