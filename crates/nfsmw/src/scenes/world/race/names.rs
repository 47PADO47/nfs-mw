//! The names of the computer-driven racers. The original gives each one a name from the language table's `RACERNAME_000`
//! to `RACERNAME_149` (`docs/provenance/race-names.md`): a random pick that is drawn again while another racer of the race
//! holds it. The rewrite adds the names mods bring (`[racers]` in `mod.toml`) to the same pool.

use blackbox_text::{StringTable, label_hash};

/// What a racer is called when the language table has no racer names (the original's text for that case).
pub const NO_NAME: &str = "UNKNOWN";

/// The names the language table holds: `RACERNAME_000`, `RACERNAME_001`, and so on up to the first one it lacks.
pub fn install_names(table: &StringTable) -> Vec<String> {
    (0..).map_while(|n| table.get(label_hash(&format!("RACERNAME_{n:03}")))).collect()
}

/// The pool of a run: the install's names, then the mods', each name once (case does not tell two apart).
pub fn pool(table: Option<&StringTable>, mods: &[String]) -> Vec<String> {
    let mut pool: Vec<String> = table.map(install_names).unwrap_or_default();
    for name in mods {
        if !pool.iter().any(|known| known.eq_ignore_ascii_case(name)) {
            pool.push(name.clone());
        }
    }
    pool
}

/// A name from `pool` that no racer in `taken` has, starting from a position `seed` picks. A pool that has none left
/// repeats one; an empty pool gives [`NO_NAME`].
pub fn pick(pool: &[String], taken: &[String], seed: u32) -> String {
    if pool.is_empty() {
        return NO_NAME.to_owned();
    }
    let start = seed as usize % pool.len();
    let free = (0..pool.len()).map(|step| &pool[(start + step) % pool.len()]).find(|name| !taken.contains(name));
    free.unwrap_or(&pool[start]).clone()
}

/// Gives the opponents of `plan` (every racer after the player) their names, from the install's list and the mods'.
pub(super) fn name_opponents(plan: &mut [super::RacerPlan], dir: &game_install::GameDir, seed: u32) {
    let table = nfsmw_data::shared::language(dir).ok().and_then(|bytes| StringTable::from_file(&bytes).ok());
    let pool = pool(table.as_ref(), crate::modding::content::racer_names());
    let Some((player, rest)) = plan.split_first_mut() else { return };
    let names = assign(&pool, std::slice::from_ref(&player.name), rest.len(), seed);
    for (racer, name) in rest.iter_mut().zip(names) {
        racer.name = name;
    }
}

/// Names for `count` racers, none repeated and none equal to `taken` (the player's name). `seed` varies the picks.
pub fn assign(pool: &[String], taken: &[String], count: usize, seed: u32) -> Vec<String> {
    let mut used = taken.to_vec();
    (0..count)
        .map(|i| {
            let name = pick(
                pool,
                &used,
                seed.wrapping_mul(0x9E37_79B1).wrapping_add((i as u32).wrapping_mul(0x85EB_CA6B)) >> 7,
            );
            used.push(name.clone());
            name
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_of(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn mod_names_join_the_pool_once() {
        let mods = pool_of(&["SLICK", "slick", "MAVERICK"]);
        assert_eq!(pool(None, &mods), ["SLICK", "MAVERICK"]);
    }

    #[test]
    fn a_pick_skips_names_in_use() {
        let pool = pool_of(&["A", "B", "C"]);
        assert_eq!(pick(&pool, &[], 4), "B");
        assert_eq!(pick(&pool, &pool_of(&["B"]), 4), "C");
        assert_eq!(pick(&pool, &pool_of(&["B", "C", "A"]), 4), "B", "a full pool repeats");
        assert_eq!(pick(&[], &[], 0), NO_NAME);
    }

    #[test]
    fn a_grid_gets_different_names_that_avoid_the_player() {
        let pool = pool_of(&["A", "B", "C", "D", "E"]);
        let names = assign(&pool, &pool_of(&["A"]), 4, 17);
        assert_eq!(names.len(), 4);
        assert!(!names.contains(&"A".to_owned()));
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "{names:?}");
    }
}
