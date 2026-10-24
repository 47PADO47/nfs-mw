//! The names of the computer-driven racers. The original gives each one a name from the language table's `RACERNAME_000`
//! to `RACERNAME_149` (`docs/provenance/race-names.md`): a random pick that is drawn again while another racer of the race
//! holds it. The rewrite adds the names mods bring (`[racers]` in `mod.toml`) to the same pool.
//!
//! A career race fixes some names instead: a boss race names its racer after the Blacklist bin
//! (`BLACKLIST_RIVAL_<bin>_LEADERBOARD`), and a character with a `RacerName` (the prologue's) always has that one
//! (`docs/specs/racer-names.md`).

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

/// The Blacklist bin a race's collection name puts it in: `race_bin_10/10_2_1_sprint` is in bin 10. `None` for a
/// race outside the numbered bins.
pub fn bin_of(race_name: &str) -> Option<u8> {
    let (folder, _) = race_name.split_once('/')?;
    let bin: u8 = folder.strip_prefix("race_bin_")?.parse().ok()?;
    (1..=21).contains(&bin).then_some(bin)
}

/// The label of the name a career race fixes for an opponent, if it does. A boss race (`boss_bin` is the race's bin)
/// names the racer after the bin; otherwise the character's own `RacerName` counts. Only a career race fixes names: a
/// quick or custom race draws every one **[exe]** (`docs/specs/racer-names.md` section 1).
pub fn fixed_label(career: bool, boss_bin: Option<u8>, racer_name: Option<&str>) -> Option<String> {
    if !career {
        return None;
    }
    if let Some(bin) = boss_bin {
        return Some(format!("BLACKLIST_RIVAL_{bin:02}_LEADERBOARD"));
    }
    racer_name.filter(|label| !label.is_empty()).map(str::to_owned)
}

/// Gives the opponents of `plan` (every racer after the player) their names: the fixed one when the data has one and the
/// language table holds its text, else one drawn from the install's list and the mods'.
pub(super) fn name_opponents(plan: &mut [super::RacerPlan], dir: &game_install::GameDir, seed: u32) {
    // The pool is the install's list of names; the fixed names are in the current language.
    let table = nfsmw_data::shared::language(dir).ok().and_then(|bytes| StringTable::from_file(&bytes).ok());
    let pool = pool(table.as_ref(), crate::modding::content::racer_names());
    let Some((player, rest)) = plan.split_first_mut() else { return };
    let resolve = |label: &str| crate::i18n::game(label_hash(label));
    name_racers(rest, &player.name, &resolve, &pool, seed);
}

/// Names `racers` (the opponents): a racer with a `name_label` that `resolve` knows gets that text, the others draw from
/// `pool` without repeating a name of the race (the player's `player` and the fixed ones count).
pub(super) fn name_racers(
    racers: &mut [super::RacerPlan],
    player: &str,
    resolve: &dyn Fn(&str) -> Option<String>,
    pool: &[String],
    seed: u32,
) {
    let fixed: Vec<Option<String>> = racers.iter().map(|r| r.name_label.as_deref().and_then(resolve)).collect();
    let mut taken: Vec<String> = fixed.iter().flatten().cloned().collect();
    taken.push(player.to_owned());
    let mut drawn = assign(pool, &taken, fixed.iter().filter(|name| name.is_none()).count(), seed).into_iter();
    for (racer, fixed) in racers.iter_mut().zip(fixed) {
        racer.name = fixed.or_else(|| drawn.next()).unwrap_or_else(|| NO_NAME.to_owned());
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

    fn racer(label: Option<&str>) -> super::super::RacerPlan {
        super::super::RacerPlan {
            name: "collection/path".into(),
            name_label: label.map(str::to_owned),
            model: Some("x".into()),
            spawn: crate::scenes::world::road::Spawn { position: glam::Vec3::ZERO, heading: 0.0 },
            skill: 0.5,
            bias: 0.5,
            tried: Vec::new(),
            person: None,
        }
    }

    fn language(label: &str) -> Option<String> {
        match label {
            "BLACKLIST_RIVAL_10_LEADERBOARD" => Some("BARON".into()),
            "BLACKLIST_RIVAL_16_LEADERBOARD" => Some("ROG".into()),
            _ => None,
        }
    }

    #[test]
    fn a_race_folder_names_its_bin() {
        assert_eq!(bin_of("race_bin_10/10_2_1_sprint"), Some(10));
        assert_eq!(bin_of("race_bin_01/1_1_2_circuit"), Some(1));
        assert_eq!(bin_of("race_bin_challenge/1_1_1"), None);
        assert_eq!(bin_of("16_1_1_circuit"), None);
        assert_eq!(bin_of("race_bin_00/x"), None);
    }

    #[test]
    fn only_a_career_race_fixes_names_and_a_boss_race_goes_by_the_bin() {
        let rival = "BLACKLIST_RIVAL_10_LEADERBOARD";
        assert_eq!(fixed_label(true, Some(10), None).as_deref(), Some(rival));
        assert_eq!(fixed_label(true, Some(10), Some("OTHER")).as_deref(), Some(rival), "the bin wins");
        let rog = "BLACKLIST_RIVAL_16_LEADERBOARD";
        assert_eq!(fixed_label(true, None, Some(rog)).as_deref(), Some(rog));
        assert_eq!(fixed_label(true, None, Some("")), None);
        assert_eq!(fixed_label(true, None, None), None);
        assert_eq!(fixed_label(false, Some(10), Some("X")), None, "a quick race draws every name");
    }

    #[test]
    fn a_fixed_name_stays_and_the_others_are_drawn_around_it() {
        let pool = pool_of(&["BARON", "A", "B", "C"]);
        let mut racers = vec![racer(Some("BLACKLIST_RIVAL_10_LEADERBOARD")), racer(None), racer(None)];
        name_racers(&mut racers, "ME", &language, &pool, 3);
        assert_eq!(racers[0].name, "BARON");
        for drawn in &racers[1..] {
            assert!(["A", "B", "C"].contains(&drawn.name.as_str()), "{}: BARON is taken", drawn.name);
        }
        assert_ne!(racers[1].name, racers[2].name);
    }

    #[test]
    fn a_label_the_table_lacks_falls_back_to_the_pool() {
        let pool = pool_of(&["A"]);
        let mut racers = vec![racer(Some("BLACKLIST_RIVAL_99_LEADERBOARD"))];
        name_racers(&mut racers, "ME", &language, &pool, 0);
        assert_eq!(racers[0].name, "A");
        let mut empty = vec![racer(None)];
        name_racers(&mut empty, "ME", &language, &[], 0);
        assert_eq!(empty[0].name, NO_NAME);
    }

    #[test]
    fn a_prologue_racer_keeps_its_authored_name_every_time() {
        let pool: Vec<String> = (0..20).map(|n| format!("R{n}")).collect();
        for seed in 0..8 {
            let mut racers = vec![racer(Some("BLACKLIST_RIVAL_16_LEADERBOARD")), racer(None), racer(None)];
            name_racers(&mut racers, "ME", &language, &pool, seed);
            assert_eq!(racers[0].name, "ROG");
            assert!(racers[1..].iter().all(|r| r.name != "ROG" && r.name != "ME"));
        }
    }

    #[test]
    fn a_race_of_sixteen_gets_fifteen_names_none_repeated_and_none_the_players() {
        let pool: Vec<String> = (0..16).map(|n| format!("R{n}")).collect();
        let names = assign(&pool, &pool_of(&["R0"]), 15, 9);
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 15, "{names:?}");
        assert!(!names.contains(&"R0".to_owned()));
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
