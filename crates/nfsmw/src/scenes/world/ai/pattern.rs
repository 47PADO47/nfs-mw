//! Traffic patterns: which cars and how fast, by neighbourhood.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§3, §4).

use glam::Vec3;

use super::manager::{Rule, STOCK_RECORDS};
use super::traffic::Cruise;

/// The pattern in use before any zone has been found.
pub const START_PATTERN: &str = "default";

/// One neighbourhood's traffic.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub cruise: Cruise,
    pub rules: Vec<Rule>,
}

/// Finds the pattern for a place: the name of the pattern zone at a point in physics space.
pub type Locator = Box<dyn Fn(Vec3) -> Option<String>>;

pub struct Patterns {
    list: Vec<Pattern>,
    locate: Locator,
    current: usize,
    /// How many of each pattern's rules are the install's: the mods' rules follow them.
    stock: Vec<usize>,
    /// How many mod traffic cars the rules were last made for; `None` before the first time.
    mod_generation: Option<usize>,
}

impl Patterns {
    pub fn new(list: Vec<Pattern>, locate: Locator) -> Self {
        let current = list.iter().position(|p| p.name == START_PATTERN).unwrap_or(0);
        let stock = list.iter().map(|p| p.rules.len().min(STOCK_RECORDS)).collect();
        Self { list, locate, current, stock, mod_generation: None }
    }

    /// Gives each pattern the rules the mods add for it (`rules_for(pattern name)`), after the install's own. `generation`
    /// says which mods' cars the rules are for (their number: it changes when a mod adds cars while the game runs), so the
    /// rules are made again only then. Returns whether the rules of the pattern in force changed.
    pub fn refresh_mod(&mut self, generation: usize, rules_for: impl Fn(&str) -> Vec<Rule>) -> bool {
        if self.mod_generation == Some(generation) {
            return false;
        }
        self.mod_generation = Some(generation);
        let mut changed = false;
        for (at, pattern) in self.list.iter_mut().enumerate() {
            let mut rules = pattern.rules[..self.stock[at]].to_vec();
            rules.extend(rules_for(&pattern.name));
            if rules == pattern.rules {
                continue;
            }
            changed |= at == self.current;
            pattern.rules = rules;
        }
        changed
    }

    pub fn current(&self) -> Option<&Pattern> {
        self.list.get(self.current)
    }

    /// Switches to the pattern of the zone at `at`; no zone leaves the pattern as it is. Returns whether it
    /// changed.
    pub fn select(&mut self, at: Vec3) -> bool {
        let Some(name) = (self.locate)(at) else { return false };
        let Some(index) = self.list.iter().position(|p| p.name == name) else { return false };
        let changed = index != self.current;
        self.current = index;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(name: &str, car: &str) -> Pattern {
        Pattern {
            name: name.into(),
            cruise: Cruise::default(),
            rules: vec![Rule { name: car.into(), rate: 3.0, max_instances: 0, percent: 0 }],
        }
    }

    #[test]
    fn the_zone_under_the_player_picks_the_pattern() {
        let locate: Locator = Box::new(|p| match p.x > 0.0 {
            true => Some("downtown".to_owned()),
            false => None,
        });
        let mut patterns = Patterns::new(vec![pattern("default", "trafha"), pattern("downtown", "traftaxi")], locate);
        assert_eq!(patterns.current().unwrap().name, "default");
        assert!(!patterns.select(Vec3::new(-5.0, 0.0, 0.0)), "no zone: the pattern stays");
        assert!(patterns.select(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(patterns.current().unwrap().rules[0].name, "traftaxi");
        assert!(!patterns.select(Vec3::new(6.0, 0.0, 0.0)), "the same zone is not a change");
    }

    fn mod_rule(name: &str) -> Rule {
        Rule { name: name.into(), rate: 6.0, max_instances: 0, percent: 20 }
    }

    #[test]
    fn the_rules_the_mods_add_follow_the_installs_and_are_made_again_only_for_new_cars() {
        let locate: Locator = Box::new(|_| None);
        let mut patterns = Patterns::new(vec![pattern("default", "trafha"), pattern("downtown", "traftaxi")], locate);
        let only_downtown = |name: &str| match name {
            "downtown" => vec![mod_rule("hatch")],
            _ => Vec::new(),
        };
        assert!(!patterns.refresh_mod(0, only_downtown), "no mod car: nothing changes");
        assert!(!patterns.refresh_mod(1, only_downtown), "the current pattern is `default`, which got no rule");
        let all = |_: &str| vec![mod_rule("hatch"), mod_rule("van")];
        assert!(patterns.refresh_mod(2, all), "the current pattern got two rules");
        let names: Vec<&str> = patterns.current().unwrap().rules.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["trafha", "hatch", "van"], "the install's rule first");
        assert!(!patterns.refresh_mod(2, all), "the same cars: the rules are not made again");
        let fewer = |_: &str| vec![mod_rule("van")];
        assert!(patterns.refresh_mod(3, fewer));
        let names: Vec<&str> = patterns.current().unwrap().rules.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["trafha", "van"], "a rule that is gone does not stay");
    }
}
