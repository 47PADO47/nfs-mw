//! Traffic patterns: which cars and how fast, by neighbourhood.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§3, §4).

use glam::Vec3;

use super::manager::Rule;
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
}

impl Patterns {
    pub fn new(list: Vec<Pattern>, locate: Locator) -> Self {
        let current = list.iter().position(|p| p.name == START_PATTERN).unwrap_or(0);
        Self { list, locate, current }
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
}
