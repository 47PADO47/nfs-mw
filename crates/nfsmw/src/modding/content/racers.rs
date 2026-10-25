//! The racer names of the mods: the `[racers]` table of each loaded mod's `mod.toml`. The race setup draws the names of
//! the computer-driven racers from these and from the game's own list (`scenes/world/race/names.rs`).

use std::sync::OnceLock;

use blackbox_modding::content::parse_racers;

use super::ModDir;

/// The names of this run, set once at start-up.
static SEALED: OnceLock<Vec<String>> = OnceLock::new();

/// The racer names the loaded mods add, in load order: empty before start-up, or when no mod adds any.
pub(crate) fn names() -> &'static [String] {
    SEALED.get().map_or(&[], Vec::as_slice)
}

/// Read the `[racers]` table of each mod and seal the names. A mod without the table, or with a bad one, adds none.
pub(super) fn register(mods: &[ModDir]) {
    let names: Vec<String> = mods.iter().flat_map(read_names).collect();
    if SEALED.set(names).is_err() {
        log::warn!("mod racer names were already sealed for this run; the new set is not used");
    }
}

/// The names one mod adds.
fn read_names(m: &ModDir) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(m.dir.join("mod.toml")) else { return Vec::new() };
    match parse_racers(&text) {
        Ok(section) => section.map(|s| s.names).unwrap_or_default(),
        Err(error) => {
            log::warn!("mod {}: [racers] is not read: {error}", m.id);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A folder of its own for each call: the tests run in parallel and would otherwise write one `mod.toml` over the other.
    fn names_of(manifest: &str) -> Vec<String> {
        // One folder per call: the tests run in parallel and each writes its own `mod.toml`.
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let call = CALLS.fetch_add(1, Ordering::Relaxed);

        let dir = std::env::temp_dir().join(format!("nfsmw-mod-racers-{}-{call}", std::process::id()));

        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.toml"), manifest).unwrap();

        read_names(&ModDir { id: "example.racers".to_owned(), dir, priority: 0 })
    }

    #[test]
    fn a_racers_table_gives_its_names() {
        assert_eq!(names_of("[racers]\nnames = [\"MAVERICK\", \"DEE\"]\n"), ["MAVERICK", "DEE"]);
    }

    #[test]
    fn no_table_or_a_bad_one_gives_none() {
        assert!(names_of("[mod]\nid = \"x\"\n").is_empty());
        assert!(names_of("[racers]\nnames = [\"\"]\n").is_empty());
    }
}
