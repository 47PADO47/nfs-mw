//! Which 2012 cars are police cars, and how they drive in the cop waves of the 2005 game's ten heat levels.
//!
//! **What is a police car** is read from the bundle ([`chameleon_vehicle::police_marks`]): the five police vehicles of the 2012
//! game are the only bundles with a siren corona object, a light bar part or the SWAT van's name, and the table of the game's
//! logic that pairs each vehicle with its role gives the same five (`docs/formats/chameleon-police.md`). None of them is
//! in a list of cars the player may drive, so they are `role = "cop"`.
//!
//! **Where they appear** is not in the 2012 data in a form that was decoded (the heat levels of that game are 40 behaviour
//! records with their own tiers), so the waves are a design: each car gets the levels of the class of car it is, with the
//! numbers of the 2005 game's own waves as the model (patrol cars at the low levels, the sport-utility units at heat 6 and
//! 7, a pursuit sports car from heat 5). The user overrides any of it in `[police]` of `config.toml`.

use std::collections::BTreeMap;

use chameleon_vehicle::PoliceMarks;
use serde::Deserialize;

use crate::config::matches;

/// The unit class of a police car (the manifest's `cop_class`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    #[default]
    Patrol,
    Pursuit,
    Heavy,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Patrol => "patrol",
            Self::Pursuit => "pursuit",
            Self::Heavy => "heavy",
        }
    }
}

/// Who may use a police car: the cop waves only, or the player's lists too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PoliceRole {
    #[default]
    Cop,
    Both,
}

impl PoliceRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cop => "cop",
            Self::Both => "both",
        }
    }
}

/// One wave entry: at heat level `heat` the wave wants `count` cars of the kind, picked with weight `chance` percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub heat: u32,
    /// Cars of the kind the wave wants (1 to 8); left out: 2.
    #[serde(default = "default_count")]
    pub count: u32,
    /// The weight of the kind in the pick, percent (1 to 100); left out: 10.
    #[serde(default = "default_chance")]
    pub chance: u32,
}

fn default_count() -> u32 {
    DEFAULT_COUNT
}

fn default_chance() -> u32 {
    DEFAULT_CHANCE
}

/// `[police.car.<id or name fragment>]`: what the user sets for one car. A `heat` list or `cop` entries replace the car's waves.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CarChoice {
    pub class: Option<Class>,
    /// Levels with the game's default numbers (the manifest's short form).
    pub heat: Option<Vec<u32>>,
    /// Entries with their own numbers (the manifest's `[[cop]]`).
    pub cop: Vec<Wave>,
}

/// The `[police]` table of `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Police {
    /// The cars whose bundle marks them as police vehicles become police cars (default). `false`: no car is made one unless
    /// `add` names it.
    pub auto: bool,
    /// More cars to make police cars: ids or name fragments.
    pub add: Vec<String>,
    /// Cars that stay player cars although their bundle marks them as police: ids or name fragments.
    pub remove: Vec<String>,
    /// `"cop"` (default): police cars are for the cop waves only; `"both"`: the player may drive them too.
    pub role: PoliceRole,
    /// Changes for one car or a make, by id or name fragment, applied in the order of their names.
    pub car: BTreeMap<String, CarChoice>,
}

impl Default for Police {
    fn default() -> Self {
        Self { auto: true, add: Vec::new(), remove: Vec::new(), role: PoliceRole::Cop, car: BTreeMap::new() }
    }
}

/// How a police car is written to its manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub role: PoliceRole,
    pub class: Class,
    /// The wave entries, ascending. Empty: the car is in every level with the game's default numbers.
    pub waves: Vec<Wave>,
}

impl Plan {
    /// The manifest text of the plan: the `cop_class` line for the head of the file, and the `[[cop]]` tables for its end.
    pub fn manifest(&self) -> (String, String) {
        let head = format!("cop_class = \"{}\"\n", self.class.as_str());
        let tables: String = self
            .waves
            .iter()
            .map(|w| format!("\n[[cop]]\nheat = {}\ncount = {}\nchance = {}\n", w.heat, w.count, w.chance))
            .collect();
        (head, tables)
    }
}

impl Police {
    /// Whether the car with `id` and display `name`, whose bundle gives `marks`, is a police car.
    pub fn is_police(&self, id: &str, name: &str, marks: &PoliceMarks) -> bool {
        let named = |list: &Vec<String>| list.iter().any(|pattern| matches(pattern, id, name));
        if named(&self.remove) {
            return false;
        }
        (self.auto && marks.is_police()) || named(&self.add)
    }

    /// The plan of a police car: its defaults, then the user's tables over them in name order.
    pub fn plan(&self, id: &str, name: &str, marks: &PoliceMarks) -> Plan {
        let (mut class, mut waves) = default_plan(name, marks.swat);
        for (pattern, choice) in &self.car {
            if !matches(pattern, id, name) {
                continue;
            }
            class = choice.class.unwrap_or(class);
            if choice.heat.is_none() && choice.cop.is_empty() {
                continue;
            }
            let mut own: Vec<Wave> = choice
                .heat
                .iter()
                .flatten()
                .map(|&heat| Wave { heat, count: DEFAULT_COUNT, chance: DEFAULT_CHANCE })
                .chain(choice.cop.iter().copied())
                .collect();
            own.sort_by_key(|wave| wave.heat);
            waves = own;
        }
        Plan { role: self.role, class, waves }
    }

    /// The mistakes of the table: a level outside 1 to 10, a count outside 1 to 8, a chance outside 1 to 100, a level twice.
    pub fn check(&self) -> Result<(), String> {
        for (pattern, choice) in &self.car {
            let mut levels: Vec<u32> = choice.heat.iter().flatten().copied().collect();
            levels.extend(choice.cop.iter().map(|wave| wave.heat));
            for wave in &choice.cop {
                if !(1..=8).contains(&wave.count) {
                    return Err(format!("[police.car.{pattern}]: count {} is not from 1 to 8", wave.count));
                }
                if !(1..=100).contains(&wave.chance) {
                    return Err(format!("[police.car.{pattern}]: chance {} is not from 1 to 100", wave.chance));
                }
            }
            if let Some(level) = levels.iter().find(|&&level| !(1..=10).contains(&level)) {
                return Err(format!("[police.car.{pattern}]: heat level {level} is not from 1 to 10"));
            }
            levels.sort_unstable();
            if let Some(pair) = levels.windows(2).find(|pair| pair[0] == pair[1]) {
                return Err(format!("[police.car.{pattern}]: heat level {} is listed twice", pair[0]));
            }
        }
        Ok(())
    }
}

/// The count and chance of a level the user lists without numbers: the manifest's own defaults.
const DEFAULT_COUNT: u32 = 2;
const DEFAULT_CHANCE: u32 = 10;

/// One kind of 2012 police car and its default place in the 2005 game's waves: the name fragments that find it, its class and
/// its `(heat, count, chance)` entries. The numbers are a design modelled on the 2005 waves, not data of the 2012 game.
struct Profile {
    fragments: &'static [&'static str],
    class: Class,
    waves: &'static [(u32, u32, u32)],
}

const PROFILES: [Profile; 5] = [
    // The patrol sedan: the first levels, as the 2005 game's mid-size cop is (4 cars at heat 1).
    Profile { fragments: &["crown"], class: Class::Patrol, waves: &[(1, 4, 100), (2, 4, 60), (3, 3, 40)] },
    // The muscle pursuit car: the low to middle levels.
    Profile {
        fragments: &["charger"],
        class: Class::Patrol,
        waves: &[(2, 3, 40), (3, 4, 60), (4, 5, 100), (5, 4, 60), (6, 3, 40)],
    },
    // The sports car: from heat 5, as the 2005 game's sport cops are.
    Profile {
        fragments: &["corvette"],
        class: Class::Pursuit,
        waves: &[(5, 2, 50), (6, 3, 60), (7, 3, 70), (8, 4, 80), (9, 4, 90), (10, 4, 100)],
    },
    // The sport utility: heat 6 and 7 with the 2005 game's own numbers for its patrol SUV, then a few at the top.
    Profile {
        fragments: &["explorer", "interceptor"],
        class: Class::Heavy,
        waves: &[(6, 2, 80), (7, 6, 90), (8, 2, 50), (9, 2, 50), (10, 2, 50)],
    },
    // The SWAT van: a heavy unit of the top levels.
    Profile { fragments: &["swat"], class: Class::Heavy, waves: &[(8, 1, 30), (9, 2, 40), (10, 2, 50)] },
];

/// The class and waves of a police car named `name`. A car no profile fits (one the user added, a police car of another
/// install) is a patrol car in every level with the default numbers.
fn default_plan(name: &str, swat: bool) -> (Class, Vec<Wave>) {
    let lower = name.to_ascii_lowercase();
    let found = PROFILES.iter().find(|profile| {
        profile.fragments.iter().any(|fragment| lower.contains(fragment)) || (swat && profile.fragments == ["swat"])
    });
    let Some(profile) = found else { return (Class::Patrol, Vec::new()) };
    let waves = profile.waves.iter().map(|&(heat, count, chance)| Wave { heat, count, chance }).collect();
    (profile.class, waves)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked() -> PoliceMarks {
        PoliceMarks { light_bar: true, ..PoliceMarks::default() }
    }

    fn waves(plan: &Plan) -> Vec<(u32, u32, u32)> {
        plan.waves.iter().map(|w| (w.heat, w.count, w.chance)).collect()
    }

    #[test]
    fn only_marked_cars_are_police_by_default() {
        let police = Police::default();
        assert!(police.is_police("DODCHARGERSR", "Dodge Charger SRT8 Cop", &marked()));
        assert!(!police.is_police("POR911CARR12", "Porsche 911 Carrera S 2012", &PoliceMarks::default()));
    }

    #[test]
    fn auto_off_leaves_only_the_added_cars_and_remove_wins() {
        let off = Police { auto: false, ..Police::default() };
        assert!(!off.is_police("A", "Cop A", &marked()));
        let added = Police { auto: false, add: vec!["Porsche".into()], ..Police::default() };
        assert!(added.is_police("POR911CARR12", "Porsche 911", &PoliceMarks::default()));
        let removed = Police { remove: vec!["swat".into()], ..Police::default() };
        let swat = PoliceMarks { swat: true, ..PoliceMarks::default() };
        assert!(!removed.is_police("NFSSWATVAN12", "NFS Swat Van 2012", &swat));
    }

    #[test]
    fn each_police_car_has_the_class_and_levels_of_its_kind() {
        let police = Police::default();
        let swat = PoliceMarks { swat: true, ..PoliceMarks::default() };
        let van = police.plan("NFSSWATVAN12", "NFS Swat Van 2012", &swat);
        assert_eq!((van.class, waves(&van)), (Class::Heavy, vec![(8, 1, 30), (9, 2, 40), (10, 2, 50)]));
        let suv = police.plan("FORDEXPLORE", "Ford Explorer Interceptor 2011", &marked());
        assert_eq!(suv.class, Class::Heavy);
        assert_eq!(&waves(&suv)[..2], &[(6, 2, 80), (7, 6, 90)]);
        let vette = police.plan("CHEVCORVETTE", "Chevrolet Corvette Z06 2006", &marked());
        assert_eq!(vette.class, Class::Pursuit);
        assert_eq!(police.plan("FORCROWNVICT", "Ford Crown Victoria COP", &marked()).waves[0].heat, 1);
    }

    #[test]
    fn an_unknown_police_car_is_a_patrol_car_in_every_level() {
        let plan = Police::default().plan("X", "Some Police Car", &marked());
        assert_eq!((plan.class, plan.waves.len(), plan.role), (Class::Patrol, 0, PoliceRole::Cop));
    }

    #[test]
    fn the_users_tables_replace_the_waves_and_the_class() {
        let text = "
[car.crown]
heat = [4, 5]
class = \"pursuit\"
[car.swat]
cop = [{ heat = 3, count = 1, chance = 5 }]
";
        let police: Police = toml::from_str(text).unwrap();
        let crown = police.plan("FORCROWNVICT", "Ford Crown Victoria COP", &marked());
        assert_eq!((crown.class, waves(&crown)), (Class::Pursuit, vec![(4, 2, 10), (5, 2, 10)]));
        let swat = PoliceMarks { swat: true, ..PoliceMarks::default() };
        let van = police.plan("NFSSWATVAN12", "NFS Swat Van 2012", &swat);
        assert_eq!((van.class, waves(&van)), (Class::Heavy, vec![(3, 1, 5)]));
        assert!(police.check().is_ok());
    }

    #[test]
    fn the_manifest_text_carries_the_class_and_one_table_per_level() {
        let waves = vec![Wave { heat: 6, count: 2, chance: 80 }];
        let (head, tables) = Plan { role: PoliceRole::Cop, class: Class::Heavy, waves }.manifest();
        assert_eq!(head, "cop_class = \"heavy\"\n");
        assert_eq!(tables, "\n[[cop]]\nheat = 6\ncount = 2\nchance = 80\n");
    }

    #[test]
    fn bad_tables_are_refused() {
        let bad = |text: &str| toml::from_str::<Police>(text).unwrap().check().is_err();
        assert!(bad("[car.a]\nheat = [11]\n"));
        assert!(bad("[car.a]\nheat = [2, 2]\n"));
        assert!(bad("[car.a]\ncop = [{ heat = 2, count = 9, chance = 5 }]\n"));
        assert!(bad("[car.a]\ncop = [{ heat = 2, count = 2, chance = 0 }]\n"));
        assert!(toml::from_str::<Police>("[car.a]\nspeed = 3\n").is_err());
    }

    /// The five police vehicles of the install, and nothing else, are marked (`docs/formats/chameleon-police.md`).
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_real_install_marks_exactly_the_five_police_vehicles() {
        let root = std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR");
        let source = crate::source::Source::open(std::path::Path::new(&root)).unwrap();
        let mut found: Vec<(u32, String)> = Vec::new();
        for file in source.cars() {
            let (name, marks) = source.identify(file);
            if marks.is_police() {
                found.push((file.number, name.unwrap_or_default()));
            }
        }
        let numbers: Vec<u32> = found.iter().map(|(number, _)| *number).collect();
        assert_eq!(numbers, [122699, 122706, 122713, 510072, 1467242], "{found:?}");
        for (_, name) in &found {
            let plan = Police::default().plan("ID", name, &PoliceMarks::default());
            println!("{name}: {} {:?}", plan.class.as_str(), plan.waves);
        }
    }
}
